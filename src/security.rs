//! Security and verification utilities for service discovery

use crate::{
    error::Result,
    service::ServiceInfo,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::signature::{self, KeyPair, Ed25519KeyPair};
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
const SEED_LENGTH: usize = 32;

/// Structure for verifying services with signature-based authentication
pub struct ServiceVerifier {
    key_pair: Ed25519KeyPair,
}

impl ServiceVerifier {
    /// Create a new service verifier
    pub fn new() -> Result<Self> {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)?;
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())?;
        
        Ok(Self { key_pair })
    }

    /// Verify a service using its digital signature
    pub fn verify_service(&self, service: &ServiceInfo) -> Result<bool> {
        let attributes = Some(&service.attributes);

        // Get required attributes
        let (signature, timestamp) = match (
            attributes.and_then(|a| a.get("signature")),
            attributes.and_then(|a| a.get("timestamp")),
        ) {
            (Some(sig), Some(ts)) => (sig, ts),
            _ => return Ok(false),
        };

        // Verify timestamp is within threshold
        let timestamp = timestamp.parse::<u64>()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        if now.saturating_sub(timestamp) > 300 {
            return Ok(false);
        }

        // Generate message for verification
        let message = self.generate_signing_message(service, timestamp)?;
        let signature_bytes = BASE64.decode(signature.as_bytes())?;

        // Verify signature
        match signature::UnparsedPublicKey::new(
            &signature::ED25519,
            &self.key_pair.public_key().as_ref()
        ).verify(
            message.as_bytes(),
            &signature_bytes,
        ) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Generate a signature for a service
    pub fn sign_service(&self, service: &mut ServiceInfo) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        service.insert_attribute("timestamp", timestamp.to_string());

        let message = self.generate_signing_message(service, timestamp)?;
        let signature = self.key_pair.sign(message.as_bytes());
        
        service.insert_attribute("signature", BASE64.encode(signature.as_ref()));
        Ok(())
    }

    fn generate_signing_message(&self, service: &ServiceInfo, timestamp: u64) -> Result<String> {
        let mut sorted_attrs: Vec<_> = service.attributes.iter()
            .filter(|(k, _)| *k != "signature" && *k != "timestamp")
            .collect();

        sorted_attrs.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        let mut message = format!(
            "{}|{}|{}|{}",
            service.name,
            service.service_type.full_name(),
            service.address,
            service.port
        );

        for (k, v) in sorted_attrs {
            message.push_str(&format!("|{k}={v}"));
        }

        message.push_str(&format!("|timestamp={timestamp}"));
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_sign_and_verify() -> Result<()> {
        let security = ServiceVerifier::new()?;

        // Create test service
        let mut service = ServiceInfo::new(
            "test_service",
            "_http._tcp",
            8080,
            None
        ).unwrap()
        .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Sign service
        security.sign_service(&mut service)?;

        // Verify service succeeds
        assert!(service.attributes.contains_key("signature"));
        assert!(service.attributes.contains_key("timestamp"));
        assert!(security.verify_service(&service)?);

        Ok(())
    }
}
