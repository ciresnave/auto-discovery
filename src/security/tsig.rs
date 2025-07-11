use crate::error::{DiscoveryError, Result};
use metrics::{counter, histogram};
use ring::hmac;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use trust_dns_client::proto::rr::dnssec::{Algorithm, SigSigner, TsigSigner};
use trust_dns_proto::{
    op::Message,
    rr::{Name, Record, RecordType},
};

/// TSIG key algorithm type
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TsigAlgorithm {
    HmacSha1,
    HmacSha256,
    HmacSha384,
    HmacSha512,
}

impl TsigAlgorithm {
    fn to_dns_algorithm(&self) -> Algorithm {
        match self {
            Self::HmacSha1 => Algorithm::HMAC_SHA1,
            Self::HmacSha256 => Algorithm::HMAC_SHA256,
            Self::HmacSha384 => Algorithm::HMAC_SHA384,
            Self::HmacSha512 => Algorithm::HMAC_SHA512,
        }
    }

    fn to_ring_algorithm(&self) -> hmac::Algorithm {
        match self {
            Self::HmacSha1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            Self::HmacSha256 => hmac::HMAC_SHA256,
            Self::HmacSha384 => hmac::HMAC_SHA384,
            Self::HmacSha512 => hmac::HMAC_SHA512,
        }
    }
}

/// TSIG key information with expiry and metadata
#[derive(Clone)]
pub struct TsigKey {
    name: Name,
    algorithm: TsigAlgorithm,
    secret: Vec<u8>,
    created_at: SystemTime,
    expires_at: Option<SystemTime>,
    key_id: String,
}

impl TsigKey {
    /// Create a new TSIG key with optional expiry
    pub fn new(
        name: &str,
        algorithm: TsigAlgorithm,
        secret: &[u8],
        expires_in: Option<Duration>,
    ) -> Result<Self> {
        let created_at = SystemTime::now();
        let expires_at = expires_in.map(|duration| created_at + duration);
        let key_id = format!("{}_{}", name, uuid::Uuid::new_v4());

        Ok(Self {
            name: Name::from_ascii(name).map_err(|e| {
                DiscoveryError::SecurityError(format!("Invalid TSIG key name: {}", e))
            })?,
            algorithm,
            secret: secret.to_vec(),
            created_at,
            expires_at,
            key_id,
        })
    }

    /// Check if the key has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expiry| SystemTime::now() > expiry)
            .unwrap_or(false)
    }

    /// Get the key ID
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Create a TSIG signer
    pub fn create_signer(&self) -> Result<TsigSigner> {
        let key = hmac::Key::new(self.algorithm.to_ring_algorithm(), &self.secret);
        let sig_signer = SigSigner::new(
            key,
            self.name.clone(),
            self.algorithm.to_dns_algorithm(),
            Default::default(),
        );
        
        Ok(TsigSigner::new(sig_signer))
    }
}

/// Manager for TSIG keys with rotation support
pub struct TsigKeyManager {
    active_keys: Arc<RwLock<Vec<TsigKey>>>,
    rotation_interval: Duration,
}

impl TsigKeyManager {
    /// Create a new TSIG key manager
    pub fn new(rotation_interval: Duration) -> Self {
        Self {
            active_keys: Arc::new(RwLock::new(Vec::new())),
            rotation_interval,
        }
    }

    /// Add a new TSIG key
    pub async fn add_key(&self, key: TsigKey) {
        let mut keys = self.active_keys.write().await;
        counter!("autodiscovery_tsig_keys_total", 1);
        keys.push(key);
    }

    /// Remove expired keys
    pub async fn remove_expired_keys(&self) -> usize {
        let mut keys = self.active_keys.write().await;
        let initial_len = keys.len();
        keys.retain(|key| !key.is_expired());
        let removed = initial_len - keys.len();
        counter!("autodiscovery_tsig_keys_expired_total", removed as u64);
        removed
    }

    /// Get a valid key for signing
    pub async fn get_signing_key(&self) -> Result<TsigKey> {
        let keys = self.active_keys.read().await;
        keys.iter()
            .find(|key| !key.is_expired())
            .cloned()
            .ok_or_else(|| DiscoveryError::SecurityError("No valid TSIG key available".into()))
    }

    /// Start background key rotation task
    pub async fn start_key_rotation(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.rotation_interval);
            loop {
                interval.tick().await;
                let removed = self.remove_expired_keys().await;
                if removed > 0 {
                    tracing::info!("Removed {} expired TSIG keys", removed);
                }
            }
        });
    }
}

/// Sign a DNS message with TSIG
pub async fn sign_message(message: &mut Message, key_manager: &TsigKeyManager) -> Result<()> {
    let key = key_manager.get_signing_key().await?;
    let start = SystemTime::now();

    let mut signer = key.create_signer()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;

    let result = signer
        .sign_message(message, now)
        .map_err(|e| DiscoveryError::SecurityError(format!("Failed to sign message: {}", e)));

    let duration = SystemTime::now().duration_since(start).unwrap();
    histogram!("autodiscovery_tsig_sign_duration_seconds", duration.as_secs_f64());
    
    if result.is_err() {
        counter!("autodiscovery_tsig_sign_errors_total", 1);
    }

    result
}

/// Verify a TSIG-signed DNS message
pub async fn verify_message(message: &Message, key_manager: &TsigKeyManager) -> Result<bool> {
    let key = key_manager.get_signing_key().await?;
    let start = SystemTime::now();

    let signer = key.create_signer()?;
    let result = signer
        .verify_message(message)
        .map_err(|e| DiscoveryError::SecurityError(format!("Failed to verify message: {}", e)));

    let duration = SystemTime::now().duration_since(start).unwrap();
    histogram!("autodiscovery_tsig_verify_duration_seconds", duration.as_secs_f64());

    if result.is_err() {
        counter!("autodiscovery_tsig_verify_errors_total", 1);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tsig_key_manager() {
        let manager = Arc::new(TsigKeyManager::new(Duration::from_secs(60)));
        
        // Add a key that expires in 1 second
        let key1 = TsigKey::new(
            "test1.key.",
            TsigAlgorithm::HmacSha256,
            b"secretkey123",
            Some(Duration::from_secs(1)),
        ).unwrap();
        manager.add_key(key1).await;

        // Add a key that doesn't expire
        let key2 = TsigKey::new(
            "test2.key.",
            TsigAlgorithm::HmacSha256,
            b"secretkey456",
            None,
        ).unwrap();
        manager.add_key(key2).await;

        // Verify we can get a signing key
        assert!(manager.get_signing_key().await.is_ok());

        // Wait for first key to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        let removed = manager.remove_expired_keys().await;
        assert_eq!(removed, 1);

        // Verify we still have one valid key
        assert!(manager.get_signing_key().await.is_ok());
    }

    #[tokio::test]
    async fn test_tsig_signing_and_verification() {
        let manager = Arc::new(TsigKeyManager::new(Duration::from_secs(60)));
        
        let key = TsigKey::new(
            "test.key.",
            TsigAlgorithm::HmacSha256,
            b"secretkey123",
            None,
        ).unwrap();
        manager.add_key(key).await;

        let mut message = Message::new();
        message.set_id(1234);

        assert!(sign_message(&mut message, &manager).await.is_ok());
        assert!(verify_message(&message, &manager).await.unwrap());
    }

    #[tokio::test]
    async fn test_tsig_algorithm_support() {
        let manager = Arc::new(TsigKeyManager::new(Duration::from_secs(60)));
        
        let algorithms = [
            TsigAlgorithm::HmacSha1,
            TsigAlgorithm::HmacSha256,
            TsigAlgorithm::HmacSha384,
            TsigAlgorithm::HmacSha512,
        ];
        
        for algorithm in &algorithms {
            let key = TsigKey::new(
                "test.key.",
                *algorithm,
                b"secretkey123",
                None,
            ).unwrap();
            manager.add_key(key).await;

            let mut message = Message::new();
            message.set_id(1234);

            assert!(sign_message(&mut message, &manager).await.is_ok());
            assert!(verify_message(&message, &manager).await.unwrap());
        }
    }
}
