use config::{Config, ConfigError, Environment, File};
use envconfig::Envconfig;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Serialize, Deserialize, Envconfig)]
pub struct DiscoveryConfig {
    #[envconfig(from = "DISCOVERY_DOMAIN", default = "local")]
    pub domain: String,

    #[envconfig(from = "DISCOVERY_TTL", default = "3600")]
    pub default_ttl: u64,

    #[envconfig(from = "DISCOVERY_TIMEOUT", default = "5")]
    pub timeout_seconds: u64,

    #[envconfig(from = "DISCOVERY_CACHE_SIZE", default = "1000")]
    pub max_cache_size: usize,

    #[envconfig(from = "DISCOVERY_CACHE_TTL", default = "300")]
    pub cache_ttl_seconds: u64,

    #[envconfig(from = "DISCOVERY_HEALTH_CHECK_INTERVAL", default = "30")]
    pub health_check_interval_seconds: u64,

    #[envconfig(from = "DISCOVERY_METRICS_ENABLED", default = "true")]
    pub metrics_enabled: bool,

    #[envconfig(from = "DISCOVERY_METRICS_PORT", default = "9000")]
    pub metrics_port: u16,

    #[envconfig(from = "DISCOVERY_DNS_SERVER")]
    pub dns_server: Option<SocketAddr>,

    #[envconfig(from = "DISCOVERY_TSIG_KEYNAME")]
    pub tsig_keyname: Option<String>,

    #[envconfig(from = "DISCOVERY_TSIG_SECRET")]
    pub tsig_secret: Option<String>,

    #[envconfig(from = "DISCOVERY_TLS_CERT")]
    pub tls_cert_path: Option<PathBuf>,

    #[envconfig(from = "DISCOVERY_TLS_KEY")]
    pub tls_key_path: Option<PathBuf>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            domain: "local".to_string(),
            default_ttl: 3600,
            timeout_seconds: 5,
            max_cache_size: 1000,
            cache_ttl_seconds: 300,
            health_check_interval_seconds: 30,
            metrics_enabled: true,
            metrics_port: 9000,
            dns_server: None,
            tsig_keyname: None,
            tsig_secret: None,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl DiscoveryConfig {
    /// Load configuration from environment and optional config file
    pub fn load() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Start with default config
        builder = builder.add_source(Config::try_from(&Self::default())?);

        // Load from config file if it exists
        if let Ok(config_dir) = std::env::var("DISCOVERY_CONFIG_DIR") {
            let config_path = PathBuf::from(config_dir).join("config.yaml");
            if config_path.exists() {
                builder = builder.add_source(File::from(config_path));
            }
        }

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("DISCOVERY")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Get the cache TTL as a Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_seconds)
    }

    /// Get the health check interval as a Duration
    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.health_check_interval_seconds)
    }

    /// Get the operation timeout as a Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    /// Check if TSIG authentication is configured
    pub fn has_tsig_config(&self) -> bool {
        self.tsig_keyname.is_some() && self.tsig_secret.is_some()
    }

    /// Check if TLS is configured
    pub fn has_tls_config(&self) -> bool {
        self.tls_cert_path.is_some() && self.tls_key_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.domain, "local");
        assert_eq!(config.default_ttl, 3600);
        assert_eq!(config.timeout_seconds, 5);
    }

    #[test]
    fn test_env_override() {
        env::set_var("DISCOVERY_DOMAIN", "example.com");
        env::set_var("DISCOVERY_TTL", "7200");
        
        let config = DiscoveryConfig::load().unwrap();
        assert_eq!(config.domain, "example.com");
        assert_eq!(config.default_ttl, 7200);

        env::remove_var("DISCOVERY_DOMAIN");
        env::remove_var("DISCOVERY_TTL");
    }

    #[test]
    fn test_duration_helpers() {
        let config = DiscoveryConfig {
            cache_ttl_seconds: 60,
            health_check_interval_seconds: 15,
            timeout_seconds: 10,
            ..Default::default()
        };

        assert_eq!(config.cache_ttl(), Duration::from_secs(60));
        assert_eq!(config.health_check_interval(), Duration::from_secs(15));
        assert_eq!(config.timeout(), Duration::from_secs(10));
    }
}
