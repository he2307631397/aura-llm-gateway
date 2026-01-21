//! Configuration management for Aura LLM Gateway
//!
//! This module provides configuration loading from environment variables
//! with sensible defaults for development.

use std::env;
use thiserror::Error;
use tracing::info;

/// Errors that can occur during configuration loading
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("invalid port number: {0}")]
    InvalidPort(String),

    #[error("invalid log level: {0}")]
    InvalidLogLevel(String),

    #[error("at least one provider API key must be configured")]
    NoProviderKeys,
}

/// Main configuration struct for the Aura LLM Gateway
///
/// Configuration is loaded from environment variables with optional
/// `.env` file support via dotenvy.
#[derive(Debug, Clone)]
pub struct Config {
    /// Host address to bind the server to
    pub host: String,

    /// Port number for the server
    pub port: u16,

    /// OpenAI API key (optional)
    pub openai_api_key: Option<String>,

    /// Anthropic API key (optional)
    pub anthropic_api_key: Option<String>,

    /// Google API key (optional)
    pub google_api_key: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,

    /// Database URL for PostgreSQL (optional, required for persistence features)
    pub database_url: Option<String>,

    /// Redis URL (optional, required for rate limiting and caching)
    pub redis_url: Option<String>,

    /// Admin API key for administrative endpoints
    pub admin_key: Option<String>,
}

impl Default for Config {
    /// Returns sensible defaults for development
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            openai_api_key: None,
            anthropic_api_key: None,
            google_api_key: None,
            log_level: "info".to_string(),
            database_url: None,
            redis_url: None,
            admin_key: None,
        }
    }
}

impl Config {
    /// Creates a new Config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration from environment variables
    ///
    /// This method will first attempt to load a `.env` file if present,
    /// then read configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if required configuration is missing or invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use aura_core::config::Config;
    ///
    /// let config = Config::from_env().expect("Failed to load config");
    /// println!("Server will listen on {}:{}", config.host, config.port);
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if present (ignore errors if file doesn't exist)
        let _ = dotenvy::dotenv();

        let host = env::var("AURA_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = env::var("AURA_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort(env::var("AURA_PORT").unwrap_or_default()))?;

        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        let base_level = log_level.split(',').next().unwrap_or(&log_level);
        let base_level = base_level.split('=').next().unwrap_or(base_level);
        if !valid_levels.contains(&base_level.to_lowercase().as_str())
            && !base_level.contains("aura")
        {
            // Allow complex log filters like "info,aura_proxy=debug"
            if !log_level.contains('=') && !log_level.contains(',') {
                return Err(ConfigError::InvalidLogLevel(log_level));
            }
        }

        let openai_api_key = env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty());
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty());
        let google_api_key = env::var("GOOGLE_API_KEY").ok().filter(|s| !s.is_empty());
        let database_url = env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
        let redis_url = env::var("REDIS_URL").ok().filter(|s| !s.is_empty());
        let admin_key = env::var("AURA_ADMIN_KEY").ok().filter(|s| !s.is_empty());

        let config = Self {
            host,
            port,
            openai_api_key,
            anthropic_api_key,
            google_api_key,
            log_level,
            database_url,
            redis_url,
            admin_key,
        };

        Ok(config)
    }

    /// Loads configuration from environment, requiring at least one provider key
    ///
    /// This is useful for production deployments where you want to ensure
    /// the gateway has at least one provider configured.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::NoProviderKeys` if no provider API keys are configured.
    pub fn from_env_strict() -> Result<Self, ConfigError> {
        let config = Self::from_env()?;
        config.validate()?;
        Ok(config)
    }

    /// Validates the configuration
    ///
    /// Checks that at least one provider API key is configured.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::NoProviderKeys` if no provider API keys are configured.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.openai_api_key.is_none()
            && self.anthropic_api_key.is_none()
            && self.google_api_key.is_none()
        {
            return Err(ConfigError::NoProviderKeys);
        }
        Ok(())
    }

    /// Returns true if OpenAI is configured
    pub fn has_openai(&self) -> bool {
        self.openai_api_key.is_some()
    }

    /// Returns true if Anthropic is configured
    pub fn has_anthropic(&self) -> bool {
        self.anthropic_api_key.is_some()
    }

    /// Returns true if Google is configured
    pub fn has_google(&self) -> bool {
        self.google_api_key.is_some()
    }

    /// Returns true if any provider is configured
    pub fn has_any_provider(&self) -> bool {
        self.has_openai() || self.has_anthropic() || self.has_google()
    }

    /// Returns the server address as a string (host:port)
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Logs the current configuration (with sensitive values masked)
    pub fn log_config(&self) {
        info!(
            host = %self.host,
            port = %self.port,
            log_level = %self.log_level,
            openai = %self.has_openai(),
            anthropic = %self.has_anthropic(),
            google = %self.has_google(),
            database = %self.database_url.is_some(),
            redis = %self.redis_url.is_some(),
            "Configuration loaded"
        );
    }
}

/// Builder for creating Config instances programmatically
///
/// Useful for testing or when you need to construct configuration
/// without environment variables.
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Creates a new ConfigBuilder with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the host address
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Sets the port number
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Sets the OpenAI API key
    pub fn openai_api_key(mut self, key: impl Into<String>) -> Self {
        self.config.openai_api_key = Some(key.into());
        self
    }

    /// Sets the Anthropic API key
    pub fn anthropic_api_key(mut self, key: impl Into<String>) -> Self {
        self.config.anthropic_api_key = Some(key.into());
        self
    }

    /// Sets the Google API key
    pub fn google_api_key(mut self, key: impl Into<String>) -> Self {
        self.config.google_api_key = Some(key.into());
        self
    }

    /// Sets the log level
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.log_level = level.into();
        self
    }

    /// Sets the database URL
    pub fn database_url(mut self, url: impl Into<String>) -> Self {
        self.config.database_url = Some(url.into());
        self
    }

    /// Sets the Redis URL
    pub fn redis_url(mut self, url: impl Into<String>) -> Self {
        self.config.redis_url = Some(url.into());
        self
    }

    /// Sets the admin API key
    pub fn admin_key(mut self, key: impl Into<String>) -> Self {
        self.config.admin_key = Some(key.into());
        self
    }

    /// Builds the Config instance
    pub fn build(self) -> Config {
        self.config
    }

    /// Builds and validates the Config instance
    pub fn build_validated(self) -> Result<Config, ConfigError> {
        let config = self.build();
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.log_level, "info");
        assert!(config.openai_api_key.is_none());
        assert!(config.anthropic_api_key.is_none());
        assert!(config.google_api_key.is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .host("127.0.0.1")
            .port(3000)
            .openai_api_key("sk-test")
            .log_level("debug")
            .build();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.openai_api_key, Some("sk-test".to_string()));
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_config_validation_no_providers() {
        let config = Config::default();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::NoProviderKeys)
        ));
    }

    #[test]
    fn test_config_validation_with_provider() {
        let config = ConfigBuilder::new().openai_api_key("sk-test").build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_has_provider_methods() {
        let config = ConfigBuilder::new()
            .openai_api_key("sk-test")
            .anthropic_api_key("sk-ant-test")
            .build();

        assert!(config.has_openai());
        assert!(config.has_anthropic());
        assert!(!config.has_google());
        assert!(config.has_any_provider());
    }

    #[test]
    fn test_server_addr() {
        let config = ConfigBuilder::new().host("localhost").port(9000).build();
        assert_eq!(config.server_addr(), "localhost:9000");
    }

    #[test]
    fn test_builder_validated_fails_without_provider() {
        let result = ConfigBuilder::new().build_validated();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_validated_succeeds_with_provider() {
        let result = ConfigBuilder::new()
            .openai_api_key("sk-test")
            .build_validated();
        assert!(result.is_ok());
    }
}
