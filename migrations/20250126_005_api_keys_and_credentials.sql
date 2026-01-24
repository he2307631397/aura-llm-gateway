-- API Keys and Encrypted Provider Credentials
-- Adds tables for customer API key authentication and encrypted storage of provider credentials

-- Enable pgcrypto for encryption functions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- API Keys table for customer authentication
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Key identifier (public prefix, e.g., "aura_live_abc123")
    key_id VARCHAR(50) NOT NULL UNIQUE,
    -- Hashed secret key (never store plaintext)
    key_hash VARCHAR(128) NOT NULL,
    -- Human-readable name for the key
    name VARCHAR(200) NOT NULL,
    -- Optional description
    description TEXT,
    -- Owner user ID (nullable for org-level keys)
    user_id VARCHAR(100),
    -- Organization ID (for team keys)
    organization_id UUID,
    -- Key scopes/permissions as JSON array (e.g., ["responses:create", "conversations:read"])
    scopes JSONB NOT NULL DEFAULT '["responses:create"]',
    -- Rate limit: requests per minute (null = default)
    rate_limit_rpm INT,
    -- Monthly token budget (null = unlimited)
    monthly_token_limit BIGINT,
    -- Current month token usage
    current_month_tokens BIGINT NOT NULL DEFAULT 0,
    -- Month/year for usage tracking reset
    usage_reset_month VARCHAR(7), -- Format: "2025-01"
    -- Key status: active, revoked, expired
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked', 'expired')),
    -- Expiration date (null = never expires)
    expires_at TIMESTAMPTZ,
    -- Last used timestamp
    last_used_at TIMESTAMPTZ,
    -- IP allowlist (null = any IP allowed)
    allowed_ips JSONB,
    -- Metadata for custom attributes
    metadata JSONB,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_api_keys_key_id ON api_keys(key_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_organization_id ON api_keys(organization_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_status ON api_keys(status);

-- API Key usage logs (for detailed tracking)
CREATE TABLE IF NOT EXISTS api_key_usage (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    -- Request metadata
    request_id VARCHAR(100) NOT NULL,
    model_id VARCHAR(100) NOT NULL,
    provider_name VARCHAR(50) NOT NULL,
    -- Token usage
    input_tokens INT NOT NULL DEFAULT 0,
    output_tokens INT NOT NULL DEFAULT 0,
    cached_tokens INT,
    reasoning_tokens INT,
    -- Cost
    cost_usd DOUBLE PRECISION,
    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_key_usage_api_key_id ON api_key_usage(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_usage_created_at ON api_key_usage(created_at);

-- Encrypted provider credentials storage
-- Allows per-organization provider credentials with envelope encryption
CREATE TABLE IF NOT EXISTS provider_credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Owner (one of these should be set)
    user_id VARCHAR(100),
    organization_id UUID,
    -- Provider name (openai, anthropic, google, etc.)
    provider_name VARCHAR(50) NOT NULL,
    -- Encrypted API key (encrypted with DEK)
    encrypted_api_key BYTEA NOT NULL,
    -- Data encryption key wrapped with master key (envelope encryption)
    -- Format: nonce (12 bytes) || wrapped_key
    wrapped_dek BYTEA NOT NULL,
    -- Key derivation parameters for the encryption
    -- Stores nonce used for API key encryption
    encryption_params JSONB NOT NULL,
    -- Optional custom base URL for the provider
    base_url VARCHAR(500),
    -- Whether this credential is active
    is_active BOOLEAN NOT NULL DEFAULT true,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    CONSTRAINT unique_user_provider UNIQUE NULLS NOT DISTINCT (user_id, provider_name),
    CONSTRAINT unique_org_provider UNIQUE NULLS NOT DISTINCT (organization_id, provider_name)
);

CREATE INDEX IF NOT EXISTS idx_provider_credentials_user_id ON provider_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_provider_credentials_organization_id ON provider_credentials(organization_id);
CREATE INDEX IF NOT EXISTS idx_provider_credentials_provider ON provider_credentials(provider_name);

-- Organizations table (for team management)
CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(200) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    -- Owner user ID
    owner_id VARCHAR(100) NOT NULL,
    -- Settings
    settings JSONB DEFAULT '{}',
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_organizations_slug ON organizations(slug);
CREATE INDEX IF NOT EXISTS idx_organizations_owner_id ON organizations(owner_id);

-- Organization members
CREATE TABLE IF NOT EXISTS organization_members (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id VARCHAR(100) NOT NULL,
    -- Role: owner, admin, member
    role VARCHAR(20) NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member')),
    -- Timestamps
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Constraints
    UNIQUE(organization_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_organization_members_org_id ON organization_members(organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_members_user_id ON organization_members(user_id);

-- Add triggers for updated_at
DROP TRIGGER IF EXISTS update_api_keys_updated_at ON api_keys;
CREATE TRIGGER update_api_keys_updated_at BEFORE UPDATE ON api_keys
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_provider_credentials_updated_at ON provider_credentials;
CREATE TRIGGER update_provider_credentials_updated_at BEFORE UPDATE ON provider_credentials
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_organizations_updated_at ON organizations;
CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to validate API key on request
-- Returns the key record if valid, NULL otherwise
CREATE OR REPLACE FUNCTION validate_api_key(p_key_id VARCHAR, p_key_hash VARCHAR)
RETURNS TABLE (
    id UUID,
    key_id VARCHAR,
    user_id VARCHAR,
    organization_id UUID,
    scopes JSONB,
    rate_limit_rpm INT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        ak.id,
        ak.key_id,
        ak.user_id,
        ak.organization_id,
        ak.scopes,
        ak.rate_limit_rpm
    FROM api_keys ak
    WHERE ak.key_id = p_key_id
      AND ak.key_hash = p_key_hash
      AND ak.status = 'active'
      AND (ak.expires_at IS NULL OR ak.expires_at > NOW());
END;
$$ LANGUAGE plpgsql;

-- Function to increment API key usage
CREATE OR REPLACE FUNCTION increment_api_key_usage(
    p_api_key_id UUID,
    p_input_tokens INT,
    p_output_tokens INT
)
RETURNS VOID AS $$
DECLARE
    current_month VARCHAR(7);
BEGIN
    current_month := TO_CHAR(NOW(), 'YYYY-MM');

    UPDATE api_keys
    SET
        current_month_tokens = CASE
            WHEN usage_reset_month = current_month
            THEN current_month_tokens + p_input_tokens + p_output_tokens
            ELSE p_input_tokens + p_output_tokens
        END,
        usage_reset_month = current_month,
        last_used_at = NOW()
    WHERE id = p_api_key_id;
END;
$$ LANGUAGE plpgsql;
