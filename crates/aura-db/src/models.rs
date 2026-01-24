//! Database models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Provider record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub api_base_url: Option<String>,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model pricing record (per 1M tokens)
/// Note: Manually constructed from queries due to Decimal handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub model_id: String,
    pub model_name: String,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cached_input_per_million: Option<f64>,
    pub reasoning_per_million: Option<f64>,
    pub context_window: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub is_enabled: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ModelPricing {
    /// Calculate cost for given token counts
    pub fn calculate_cost(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: Option<u32>,
        reasoning_tokens: Option<u32>,
    ) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_million;

        let cached_cost = cached_tokens
            .and_then(|tokens| {
                self.cached_input_per_million
                    .map(|rate| (tokens as f64 / 1_000_000.0) * rate)
            })
            .unwrap_or(0.0);

        let reasoning_cost = reasoning_tokens
            .and_then(|tokens| {
                self.reasoning_per_million
                    .map(|rate| (tokens as f64 / 1_000_000.0) * rate)
            })
            .unwrap_or(0.0);

        input_cost + output_cost + cached_cost + reasoning_cost
    }
}

/// Simplified model pricing for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricingSimple {
    pub model_id: String,
    pub model_name: String,
    pub provider_name: String,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cached_input_per_million: Option<f64>,
    pub context_window: Option<i32>,
    pub max_output_tokens: Option<i32>,
}

/// Conversation record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub model_id: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Request log record - manually constructed due to Decimal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: Uuid,
    pub response_id: String,
    pub conversation_id: Option<Uuid>,
    pub provider_name: String,
    pub model_id: String,
    pub user_id: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub cached_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i32>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// New request log for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRequestLog {
    pub response_id: String,
    pub conversation_id: Option<Uuid>,
    pub provider_name: String,
    pub model_id: String,
    pub user_id: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub cached_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i32>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// New conversation for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConversation {
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub model_id: String,
    pub metadata: Option<serde_json::Value>,
}

/// New message for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessage {
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// Response record - stores complete Open Responses API response objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRecord {
    pub id: String,
    pub conversation_id: Uuid,
    pub model_id: String,
    pub status: String,
    pub previous_response_id: Option<String>,
    pub input_items: serde_json::Value,
    pub output_items: serde_json::Value,
    pub usage_input_tokens: Option<i32>,
    pub usage_output_tokens: Option<i32>,
    pub usage_cached_tokens: Option<i32>,
    pub usage_reasoning_tokens: Option<i32>,
    pub usage_cost_usd: Option<f64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub incomplete_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New response for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewResponse {
    pub id: String,
    pub conversation_id: Uuid,
    pub model_id: String,
    pub status: String,
    pub previous_response_id: Option<String>,
    pub input_items: serde_json::Value,
    pub output_items: serde_json::Value,
    pub usage_input_tokens: Option<i32>,
    pub usage_output_tokens: Option<i32>,
    pub usage_cached_tokens: Option<i32>,
    pub usage_reasoning_tokens: Option<i32>,
    pub usage_cost_usd: Option<f64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub incomplete_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// API Key Models
// ============================================================================

/// API Key status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl std::fmt::Display for ApiKeyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeyStatus::Active => write!(f, "active"),
            ApiKeyStatus::Revoked => write!(f, "revoked"),
            ApiKeyStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for ApiKeyStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(ApiKeyStatus::Active),
            "revoked" => Ok(ApiKeyStatus::Revoked),
            "expired" => Ok(ApiKeyStatus::Expired),
            _ => Err(format!("Unknown API key status: {}", s)),
        }
    }
}

/// API Key record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub key_id: String,
    pub key_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub scopes: serde_json::Value,
    pub rate_limit_rpm: Option<i32>,
    pub monthly_token_limit: Option<i64>,
    pub current_month_tokens: i64,
    pub usage_reset_month: Option<String>,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub allowed_ips: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiKey {
    /// Check if the key is currently valid
    pub fn is_valid(&self) -> bool {
        self.status == "active" && self.expires_at.map(|exp| exp > Utc::now()).unwrap_or(true)
    }

    /// Check if a scope is allowed
    pub fn has_scope(&self, scope: &str) -> bool {
        if let Some(scopes) = self.scopes.as_array() {
            scopes
                .iter()
                .any(|s| s.as_str() == Some(scope) || s.as_str() == Some("*"))
        } else {
            false
        }
    }

    /// Get scopes as a vector of strings
    pub fn get_scopes(&self) -> Vec<String> {
        self.scopes
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// New API key for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApiKey {
    pub key_id: String,
    pub key_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub scopes: serde_json::Value,
    pub rate_limit_rpm: Option<i32>,
    pub monthly_token_limit: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub allowed_ips: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// API key usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    pub id: Uuid,
    pub api_key_id: Uuid,
    pub request_id: String,
    pub model_id: String,
    pub provider_name: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cached_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// New API key usage for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApiKeyUsage {
    pub api_key_id: Uuid,
    pub request_id: String,
    pub model_id: String,
    pub provider_name: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cached_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
}

// ============================================================================
// Provider Credentials Models
// ============================================================================

/// Encrypted provider credentials record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredential {
    pub id: Uuid,
    pub user_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub provider_name: String,
    pub encrypted_api_key: Vec<u8>,
    pub wrapped_dek: Vec<u8>,
    pub encryption_params: serde_json::Value,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New provider credential for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProviderCredential {
    pub user_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub provider_name: String,
    pub encrypted_api_key: Vec<u8>,
    pub wrapped_dek: Vec<u8>,
    pub encryption_params: serde_json::Value,
    pub base_url: Option<String>,
}

// ============================================================================
// Organization Models
// ============================================================================

/// Organization record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: String,
    pub settings: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New organization for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrganization {
    pub name: String,
    pub slug: String,
    pub owner_id: String,
    pub settings: Option<serde_json::Value>,
}

/// Organization member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
}

impl std::fmt::Display for OrgRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrgRole::Owner => write!(f, "owner"),
            OrgRole::Admin => write!(f, "admin"),
            OrgRole::Member => write!(f, "member"),
        }
    }
}

/// Organization member record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

/// New organization member for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrganizationMember {
    pub organization_id: Uuid,
    pub user_id: String,
    pub role: String,
}
