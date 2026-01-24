//! Database repository functions

use sqlx::Row;
use uuid::Uuid;

use crate::error::DbError;
use crate::models::*;
use crate::pool::DbPool;

/// Repository for provider operations
pub struct ProviderRepo;

impl ProviderRepo {
    /// Get all enabled providers
    pub async fn get_all(pool: &DbPool) -> Result<Vec<Provider>, DbError> {
        let providers = sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE is_enabled = true ORDER BY name",
        )
        .fetch_all(pool)
        .await?;

        Ok(providers)
    }

    /// Get provider by name
    pub async fn get_by_name(pool: &DbPool, name: &str) -> Result<Option<Provider>, DbError> {
        let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(provider)
    }
}

/// Repository for model pricing operations
pub struct ModelPricingRepo;

impl ModelPricingRepo {
    /// Get current pricing for a model
    pub async fn get_by_model_id(
        pool: &DbPool,
        model_id: &str,
    ) -> Result<Option<ModelPricing>, DbError> {
        // Use raw query due to Decimal type
        let row = sqlx::query(
            r#"
            SELECT
                mp.id, mp.provider_id, mp.model_id, mp.model_name,
                mp.input_per_million::float8, mp.output_per_million::float8,
                mp.cached_input_per_million::float8, mp.reasoning_per_million::float8,
                mp.context_window, mp.max_output_tokens, mp.is_enabled,
                mp.effective_from, mp.effective_until, mp.created_at, mp.updated_at
            FROM model_pricing mp
            WHERE mp.model_id = $1
                AND mp.is_enabled = true
                AND mp.effective_from <= NOW()
                AND (mp.effective_until IS NULL OR mp.effective_until > NOW())
            ORDER BY mp.effective_from DESC
            LIMIT 1
            "#,
        )
        .bind(model_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| ModelPricing {
            id: r.get("id"),
            provider_id: r.get("provider_id"),
            model_id: r.get("model_id"),
            model_name: r.get("model_name"),
            input_per_million: r.get("input_per_million"),
            output_per_million: r.get("output_per_million"),
            cached_input_per_million: r.get("cached_input_per_million"),
            reasoning_per_million: r.get("reasoning_per_million"),
            context_window: r.get("context_window"),
            max_output_tokens: r.get("max_output_tokens"),
            is_enabled: r.get("is_enabled"),
            effective_from: r.get("effective_from"),
            effective_until: r.get("effective_until"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Get all current pricing
    pub async fn get_all_current(pool: &DbPool) -> Result<Vec<ModelPricingSimple>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (mp.model_id)
                mp.model_id, mp.model_name, p.name as provider_name,
                mp.input_per_million::float8, mp.output_per_million::float8,
                mp.cached_input_per_million::float8,
                mp.context_window, mp.max_output_tokens
            FROM model_pricing mp
            JOIN providers p ON mp.provider_id = p.id
            WHERE mp.is_enabled = true
                AND p.is_enabled = true
                AND mp.effective_from <= NOW()
                AND (mp.effective_until IS NULL OR mp.effective_until > NOW())
            ORDER BY mp.model_id, mp.effective_from DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ModelPricingSimple {
                model_id: r.get("model_id"),
                model_name: r.get("model_name"),
                provider_name: r.get("provider_name"),
                input_per_million: r.get("input_per_million"),
                output_per_million: r.get("output_per_million"),
                cached_input_per_million: r.get("cached_input_per_million"),
                context_window: r.get("context_window"),
                max_output_tokens: r.get("max_output_tokens"),
            })
            .collect())
    }

    /// Get pricing for a provider
    pub async fn get_by_provider(
        pool: &DbPool,
        provider_name: &str,
    ) -> Result<Vec<ModelPricingSimple>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (mp.model_id)
                mp.model_id, mp.model_name, p.name as provider_name,
                mp.input_per_million::float8, mp.output_per_million::float8,
                mp.cached_input_per_million::float8,
                mp.context_window, mp.max_output_tokens
            FROM model_pricing mp
            JOIN providers p ON mp.provider_id = p.id
            WHERE p.name = $1
                AND mp.is_enabled = true
                AND p.is_enabled = true
                AND mp.effective_from <= NOW()
                AND (mp.effective_until IS NULL OR mp.effective_until > NOW())
            ORDER BY mp.model_id, mp.effective_from DESC
            "#,
        )
        .bind(provider_name)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ModelPricingSimple {
                model_id: r.get("model_id"),
                model_name: r.get("model_name"),
                provider_name: r.get("provider_name"),
                input_per_million: r.get("input_per_million"),
                output_per_million: r.get("output_per_million"),
                cached_input_per_million: r.get("cached_input_per_million"),
                context_window: r.get("context_window"),
                max_output_tokens: r.get("max_output_tokens"),
            })
            .collect())
    }
}

/// Repository for conversation operations
pub struct ConversationRepo;

impl ConversationRepo {
    /// Create a new conversation
    pub async fn create(pool: &DbPool, new: NewConversation) -> Result<Conversation, DbError> {
        let conv = sqlx::query_as::<_, Conversation>(
            r#"
            INSERT INTO conversations (user_id, title, model_id, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&new.user_id)
        .bind(&new.title)
        .bind(&new.model_id)
        .bind(&new.metadata)
        .fetch_one(pool)
        .await?;

        Ok(conv)
    }

    /// Get conversation by ID
    pub async fn get_by_id(pool: &DbPool, id: Uuid) -> Result<Option<Conversation>, DbError> {
        let conv = sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(conv)
    }

    /// Get conversations for a user
    pub async fn get_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<Conversation>, DbError> {
        let convs = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT * FROM conversations
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(convs)
    }

    /// Update conversation title
    pub async fn update_title(pool: &DbPool, id: Uuid, title: &str) -> Result<(), DbError> {
        sqlx::query("UPDATE conversations SET title = $2 WHERE id = $1")
            .bind(id)
            .bind(title)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete conversation
    pub async fn delete(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Create conversation with auto-generated title from first message
    pub async fn create_with_auto_title(
        pool: &DbPool,
        user_id: Option<String>,
        model_id: String,
        first_message: &str,
    ) -> Result<Conversation, DbError> {
        let title = if first_message.len() > 100 {
            format!("{}...", &first_message[..97])
        } else {
            first_message.to_string()
        };

        let new = NewConversation {
            user_id,
            title: Some(title),
            model_id,
            metadata: None,
        };

        Self::create(pool, new).await
    }
}

/// Repository for message operations
pub struct MessageRepo;

impl MessageRepo {
    /// Create a new message
    pub async fn create(pool: &DbPool, new: NewMessage) -> Result<Message, DbError> {
        let msg = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (conversation_id, role, content, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(new.conversation_id)
        .bind(&new.role)
        .bind(&new.content)
        .bind(&new.metadata)
        .fetch_one(pool)
        .await?;

        Ok(msg)
    }

    /// Get messages for a conversation
    pub async fn get_by_conversation(
        pool: &DbPool,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>, DbError> {
        let msgs = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE conversation_id = $1 ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        Ok(msgs)
    }
}

/// Repository for request log operations
pub struct RequestLogRepo;

impl RequestLogRepo {
    /// Create a new request log
    pub async fn create(pool: &DbPool, new: NewRequestLog) -> Result<RequestLog, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO request_logs (
                response_id, conversation_id, provider_name, model_id, user_id,
                input_tokens, output_tokens, cached_tokens, reasoning_tokens,
                cost_usd, latency_ms, status, error_code, error_message, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(&new.response_id)
        .bind(new.conversation_id)
        .bind(&new.provider_name)
        .bind(&new.model_id)
        .bind(&new.user_id)
        .bind(new.input_tokens)
        .bind(new.output_tokens)
        .bind(new.cached_tokens)
        .bind(new.reasoning_tokens)
        .bind(new.cost_usd)
        .bind(new.latency_ms)
        .bind(&new.status)
        .bind(&new.error_code)
        .bind(&new.error_message)
        .bind(&new.metadata)
        .fetch_one(pool)
        .await?;

        Ok(RequestLog {
            id: row.get("id"),
            response_id: row.get("response_id"),
            conversation_id: row.get("conversation_id"),
            provider_name: row.get("provider_name"),
            model_id: row.get("model_id"),
            user_id: row.get("user_id"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
            cached_tokens: row.get("cached_tokens"),
            reasoning_tokens: row.get("reasoning_tokens"),
            cost_usd: row.get::<Option<f64>, _>("cost_usd"),
            latency_ms: row.get("latency_ms"),
            status: row.get("status"),
            error_code: row.get("error_code"),
            error_message: row.get("error_message"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
        })
    }

    /// Get recent request logs for a user
    pub async fn get_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<RequestLog>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM request_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| RequestLog {
                id: row.get("id"),
                response_id: row.get("response_id"),
                conversation_id: row.get("conversation_id"),
                provider_name: row.get("provider_name"),
                model_id: row.get("model_id"),
                user_id: row.get("user_id"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cached_tokens: row.get("cached_tokens"),
                reasoning_tokens: row.get("reasoning_tokens"),
                cost_usd: row.get::<Option<f64>, _>("cost_usd"),
                latency_ms: row.get("latency_ms"),
                status: row.get("status"),
                error_code: row.get("error_code"),
                error_message: row.get("error_message"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

/// Repository for response operations
pub struct ResponseRepo;

impl ResponseRepo {
    /// Create a new response record
    pub async fn create(pool: &DbPool, new: NewResponse) -> Result<ResponseRecord, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO responses (
                id, conversation_id, model_id, status, previous_response_id,
                input_items, output_items,
                usage_input_tokens, usage_output_tokens, usage_cached_tokens, usage_reasoning_tokens,
                usage_cost_usd, error_code, error_message, incomplete_reason, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(&new.id)
        .bind(new.conversation_id)
        .bind(&new.model_id)
        .bind(&new.status)
        .bind(&new.previous_response_id)
        .bind(&new.input_items)
        .bind(&new.output_items)
        .bind(new.usage_input_tokens)
        .bind(new.usage_output_tokens)
        .bind(new.usage_cached_tokens)
        .bind(new.usage_reasoning_tokens)
        .bind(new.usage_cost_usd)
        .bind(&new.error_code)
        .bind(&new.error_message)
        .bind(&new.incomplete_reason)
        .bind(&new.metadata)
        .fetch_one(pool)
        .await?;

        Ok(ResponseRecord {
            id: row.get("id"),
            conversation_id: row.get("conversation_id"),
            model_id: row.get("model_id"),
            status: row.get("status"),
            previous_response_id: row.get("previous_response_id"),
            input_items: row.get("input_items"),
            output_items: row.get("output_items"),
            usage_input_tokens: row.get("usage_input_tokens"),
            usage_output_tokens: row.get("usage_output_tokens"),
            usage_cached_tokens: row.get("usage_cached_tokens"),
            usage_reasoning_tokens: row.get("usage_reasoning_tokens"),
            usage_cost_usd: row.get("usage_cost_usd"),
            error_code: row.get("error_code"),
            error_message: row.get("error_message"),
            incomplete_reason: row.get("incomplete_reason"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Find conversation ID by response ID (follows previous_response_id chain)
    pub async fn find_conversation_by_response_id(
        pool: &DbPool,
        response_id: &str,
    ) -> Result<Option<Uuid>, DbError> {
        let row = sqlx::query("SELECT conversation_id FROM responses WHERE id = $1")
            .bind(response_id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| r.get("conversation_id")))
    }

    /// Get all responses in a conversation (ordered chronologically)
    pub async fn get_by_conversation(
        pool: &DbPool,
        conversation_id: Uuid,
    ) -> Result<Vec<ResponseRecord>, DbError> {
        let rows = sqlx::query(
            "SELECT * FROM responses WHERE conversation_id = $1 ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ResponseRecord {
                id: r.get("id"),
                conversation_id: r.get("conversation_id"),
                model_id: r.get("model_id"),
                status: r.get("status"),
                previous_response_id: r.get("previous_response_id"),
                input_items: r.get("input_items"),
                output_items: r.get("output_items"),
                usage_input_tokens: r.get("usage_input_tokens"),
                usage_output_tokens: r.get("usage_output_tokens"),
                usage_cached_tokens: r.get("usage_cached_tokens"),
                usage_reasoning_tokens: r.get("usage_reasoning_tokens"),
                usage_cost_usd: r.get("usage_cost_usd"),
                error_code: r.get("error_code"),
                error_message: r.get("error_message"),
                incomplete_reason: r.get("incomplete_reason"),
                metadata: r.get("metadata"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }
}

// ============================================================================
// API Key Repository
// ============================================================================

/// Repository for API key operations
pub struct ApiKeyRepo;

impl ApiKeyRepo {
    /// Create a new API key
    pub async fn create(pool: &DbPool, new: NewApiKey) -> Result<ApiKey, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO api_keys (
                key_id, key_hash, name, description, user_id, organization_id,
                scopes, rate_limit_rpm, monthly_token_limit, expires_at,
                allowed_ips, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(&new.key_id)
        .bind(&new.key_hash)
        .bind(&new.name)
        .bind(&new.description)
        .bind(&new.user_id)
        .bind(new.organization_id)
        .bind(&new.scopes)
        .bind(new.rate_limit_rpm)
        .bind(new.monthly_token_limit)
        .bind(new.expires_at)
        .bind(&new.allowed_ips)
        .bind(&new.metadata)
        .fetch_one(pool)
        .await?;

        Ok(Self::row_to_api_key(row))
    }

    /// Find API key by key_id (the public prefix)
    pub async fn find_by_key_id(pool: &DbPool, key_id: &str) -> Result<Option<ApiKey>, DbError> {
        let row = sqlx::query("SELECT * FROM api_keys WHERE key_id = $1")
            .bind(key_id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(Self::row_to_api_key))
    }

    /// Find API key by ID
    pub async fn find_by_id(pool: &DbPool, id: Uuid) -> Result<Option<ApiKey>, DbError> {
        let row = sqlx::query("SELECT * FROM api_keys WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(Self::row_to_api_key))
    }

    /// Get all API keys for a user
    pub async fn get_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<ApiKey>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_api_key).collect())
    }

    /// Get all API keys for an organization
    pub async fn get_by_organization(
        pool: &DbPool,
        organization_id: Uuid,
    ) -> Result<Vec<ApiKey>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM api_keys
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_api_key).collect())
    }

    /// Validate an API key and return it if valid
    pub async fn validate(
        pool: &DbPool,
        key_id: &str,
        key_hash: &str,
    ) -> Result<Option<ApiKey>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM api_keys
            WHERE key_id = $1
              AND key_hash = $2
              AND status = 'active'
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(key_id)
        .bind(key_hash)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(Self::row_to_api_key))
    }

    /// Update last_used_at timestamp
    pub async fn update_last_used(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Revoke an API key
    pub async fn revoke(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE api_keys SET status = 'revoked' WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete an API key
    pub async fn delete(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Increment token usage for an API key
    pub async fn increment_usage(
        pool: &DbPool,
        id: Uuid,
        input_tokens: i32,
        output_tokens: i32,
    ) -> Result<(), DbError> {
        sqlx::query("SELECT increment_api_key_usage($1, $2, $3)")
            .bind(id)
            .bind(input_tokens)
            .bind(output_tokens)
            .execute(pool)
            .await?;

        Ok(())
    }

    fn row_to_api_key(row: sqlx::postgres::PgRow) -> ApiKey {
        ApiKey {
            id: row.get("id"),
            key_id: row.get("key_id"),
            key_hash: row.get("key_hash"),
            name: row.get("name"),
            description: row.get("description"),
            user_id: row.get("user_id"),
            organization_id: row.get("organization_id"),
            scopes: row.get("scopes"),
            rate_limit_rpm: row.get("rate_limit_rpm"),
            monthly_token_limit: row.get("monthly_token_limit"),
            current_month_tokens: row.get("current_month_tokens"),
            usage_reset_month: row.get("usage_reset_month"),
            status: row.get("status"),
            expires_at: row.get("expires_at"),
            last_used_at: row.get("last_used_at"),
            allowed_ips: row.get("allowed_ips"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Repository for API key usage tracking
pub struct ApiKeyUsageRepo;

impl ApiKeyUsageRepo {
    /// Record API key usage
    pub async fn create(pool: &DbPool, new: NewApiKeyUsage) -> Result<ApiKeyUsage, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO api_key_usage (
                api_key_id, request_id, model_id, provider_name,
                input_tokens, output_tokens, cached_tokens, reasoning_tokens, cost_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(new.api_key_id)
        .bind(&new.request_id)
        .bind(&new.model_id)
        .bind(&new.provider_name)
        .bind(new.input_tokens)
        .bind(new.output_tokens)
        .bind(new.cached_tokens)
        .bind(new.reasoning_tokens)
        .bind(new.cost_usd)
        .fetch_one(pool)
        .await?;

        Ok(ApiKeyUsage {
            id: row.get("id"),
            api_key_id: row.get("api_key_id"),
            request_id: row.get("request_id"),
            model_id: row.get("model_id"),
            provider_name: row.get("provider_name"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
            cached_tokens: row.get("cached_tokens"),
            reasoning_tokens: row.get("reasoning_tokens"),
            cost_usd: row.get("cost_usd"),
            created_at: row.get("created_at"),
        })
    }

    /// Get usage for an API key within a time range
    pub async fn get_by_api_key(
        pool: &DbPool,
        api_key_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ApiKeyUsage>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM api_key_usage
            WHERE api_key_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(api_key_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApiKeyUsage {
                id: row.get("id"),
                api_key_id: row.get("api_key_id"),
                request_id: row.get("request_id"),
                model_id: row.get("model_id"),
                provider_name: row.get("provider_name"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cached_tokens: row.get("cached_tokens"),
                reasoning_tokens: row.get("reasoning_tokens"),
                cost_usd: row.get("cost_usd"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

// ============================================================================
// Provider Credentials Repository
// ============================================================================

/// Repository for encrypted provider credentials
pub struct ProviderCredentialRepo;

impl ProviderCredentialRepo {
    /// Store encrypted provider credentials
    pub async fn create(
        pool: &DbPool,
        new: NewProviderCredential,
    ) -> Result<ProviderCredential, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO provider_credentials (
                user_id, organization_id, provider_name,
                encrypted_api_key, wrapped_dek, encryption_params, base_url
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&new.user_id)
        .bind(new.organization_id)
        .bind(&new.provider_name)
        .bind(&new.encrypted_api_key)
        .bind(&new.wrapped_dek)
        .bind(&new.encryption_params)
        .bind(&new.base_url)
        .fetch_one(pool)
        .await?;

        Ok(Self::row_to_credential(row))
    }

    /// Get credentials for a user and provider
    pub async fn get_by_user_and_provider(
        pool: &DbPool,
        user_id: &str,
        provider_name: &str,
    ) -> Result<Option<ProviderCredential>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM provider_credentials
            WHERE user_id = $1 AND provider_name = $2 AND is_active = true
            "#,
        )
        .bind(user_id)
        .bind(provider_name)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(Self::row_to_credential))
    }

    /// Get credentials for an organization and provider
    pub async fn get_by_org_and_provider(
        pool: &DbPool,
        organization_id: Uuid,
        provider_name: &str,
    ) -> Result<Option<ProviderCredential>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM provider_credentials
            WHERE organization_id = $1 AND provider_name = $2 AND is_active = true
            "#,
        )
        .bind(organization_id)
        .bind(provider_name)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(Self::row_to_credential))
    }

    /// Get all credentials for a user
    pub async fn get_by_user(
        pool: &DbPool,
        user_id: &str,
    ) -> Result<Vec<ProviderCredential>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM provider_credentials
            WHERE user_id = $1 AND is_active = true
            ORDER BY provider_name
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_credential).collect())
    }

    /// Deactivate credentials
    pub async fn deactivate(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE provider_credentials SET is_active = false WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete credentials
    pub async fn delete(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("DELETE FROM provider_credentials WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    fn row_to_credential(row: sqlx::postgres::PgRow) -> ProviderCredential {
        ProviderCredential {
            id: row.get("id"),
            user_id: row.get("user_id"),
            organization_id: row.get("organization_id"),
            provider_name: row.get("provider_name"),
            encrypted_api_key: row.get("encrypted_api_key"),
            wrapped_dek: row.get("wrapped_dek"),
            encryption_params: row.get("encryption_params"),
            base_url: row.get("base_url"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

// ============================================================================
// Organization Repository
// ============================================================================

/// Repository for organization operations
pub struct OrganizationRepo;

impl OrganizationRepo {
    /// Create a new organization
    pub async fn create(pool: &DbPool, new: NewOrganization) -> Result<Organization, DbError> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (name, slug, owner_id, settings)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&new.name)
        .bind(&new.slug)
        .bind(&new.owner_id)
        .bind(&new.settings)
        .fetch_one(pool)
        .await?;

        Ok(org)
    }

    /// Find organization by ID
    pub async fn find_by_id(pool: &DbPool, id: Uuid) -> Result<Option<Organization>, DbError> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(org)
    }

    /// Find organization by slug
    pub async fn find_by_slug(pool: &DbPool, slug: &str) -> Result<Option<Organization>, DbError> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await?;

        Ok(org)
    }

    /// Get organizations for a user
    pub async fn get_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<Organization>, DbError> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT o.* FROM organizations o
            JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1
            ORDER BY o.name
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(orgs)
    }

    /// Delete organization
    pub async fn delete(pool: &DbPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// Repository for organization member operations
pub struct OrganizationMemberRepo;

impl OrganizationMemberRepo {
    /// Add a member to an organization
    pub async fn add_member(
        pool: &DbPool,
        new: NewOrganizationMember,
    ) -> Result<OrganizationMember, DbError> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            INSERT INTO organization_members (organization_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(new.organization_id)
        .bind(&new.user_id)
        .bind(&new.role)
        .fetch_one(pool)
        .await?;

        Ok(member)
    }

    /// Get members of an organization
    pub async fn get_members(
        pool: &DbPool,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationMember>, DbError> {
        let members = sqlx::query_as::<_, OrganizationMember>(
            r#"
            SELECT * FROM organization_members
            WHERE organization_id = $1
            ORDER BY joined_at
            "#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;

        Ok(members)
    }

    /// Check if user is a member of an organization
    pub async fn is_member(
        pool: &DbPool,
        organization_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DbError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM organization_members
            WHERE organization_id = $1 AND user_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Remove a member from an organization
    pub async fn remove_member(
        pool: &DbPool,
        organization_id: Uuid,
        user_id: &str,
    ) -> Result<(), DbError> {
        sqlx::query("DELETE FROM organization_members WHERE organization_id = $1 AND user_id = $2")
            .bind(organization_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update member role
    pub async fn update_role(
        pool: &DbPool,
        organization_id: Uuid,
        user_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE organization_members SET role = $3 WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await?;

        Ok(())
    }
}
