//! Error types for the auto-discovery library

use std::{
    error::Error as StdError,
    fmt,
    io,
    num::ParseIntError,
    time::SystemTimeError,
};
use base64::DecodeError;
use ring::error::{KeyRejected, Unspecified};

/// The primary error type for the auto-discovery crate
#[derive(Debug)]
pub enum DiscoveryError {
    /// Invalid configuration error
    Configuration(String),
    /// Invalid service data error
    InvalidData(String),
    /// Invalid service info error
    InvalidServiceInfo { 
        /// The field that contains invalid data
        field: String, 
        /// The reason why the field is invalid
        reason: String 
    },
    /// Service not found error
    ServiceNotFound(String),
    /// DNS resolution error
    DnsResolution(String),
    /// mDNS protocol error
    Mdns(String),
    /// UPnP/SSDP protocol error
    Upnp(String),
    /// DNS-SD protocol error
    DnsSd(String),
    /// Network operation error
    Network(String),
    /// Protocol operation timeout
    Timeout(String),
    /// Service verification error
    Verification(String),
    /// Protocol error
    Protocol(String),
    /// I/O error
    Io(io::Error),
    /// Security error
    Security(String),
    /// Other error types
    Other(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(msg) => write!(f, "Configuration error: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
            Self::InvalidServiceInfo { field, reason } => {
                write!(f, "Invalid service info ({field}): {reason}")
            }
            Self::ServiceNotFound(msg) => write!(f, "Service not found: {msg}"),
            Self::DnsResolution(msg) => write!(f, "DNS resolution error: {msg}"),
            Self::Mdns(msg) => write!(f, "mDNS error: {msg}"),
            Self::Upnp(msg) => write!(f, "UPnP error: {msg}"),
            Self::DnsSd(msg) => write!(f, "DNS-SD error: {msg}"),
            Self::Network(msg) => write!(f, "Network error: {msg}"),
            Self::Timeout(msg) => write!(f, "Timeout: {msg}"),
            Self::Verification(msg) => write!(f, "Verification error: {msg}"),
            Self::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Security(msg) => write!(f, "Security error: {msg}"),
            Self::Other(msg) => write!(f, "Error: {msg}"),
        }
    }
}

impl StdError for DiscoveryError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for DiscoveryError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseIntError> for DiscoveryError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidData(err.to_string())
    }
}

impl From<SystemTimeError> for DiscoveryError {
    fn from(err: SystemTimeError) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<DecodeError> for DiscoveryError {
    fn from(err: DecodeError) -> Self {
        Self::Security(err.to_string())
    }
}

impl From<Unspecified> for DiscoveryError {
    fn from(err: Unspecified) -> Self {
        Self::Security(err.to_string())
    }
}

impl From<KeyRejected> for DiscoveryError {
    fn from(err: KeyRejected) -> Self {
        Self::Security(err.to_string())
    }
}

impl From<mdns_sd::Error> for DiscoveryError {
    fn from(err: mdns_sd::Error) -> Self {
        Self::Mdns(err.to_string())
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Fatal error requiring immediate attention
    Fatal,
    /// Error condition
    Error,
    /// Warning condition
    Warning,
    /// Informational message
    Info,
}

/// Common result type for library operations
pub type Result<T> = std::result::Result<T, DiscoveryError>;

impl DiscoveryError {
    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(msg: S) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a new invalid data error
    pub fn invalid_data<S: Into<String>>(msg: S) -> Self {
        Self::InvalidData(msg.into())
    }

    /// Create a new service not found error
    pub fn service_not_found<S: Into<String>>(msg: S) -> Self {
        Self::ServiceNotFound(msg.into())
    }

    /// Create a new DNS resolution error
    pub fn dns_resolution<S: Into<String>>(msg: S) -> Self {
        Self::DnsResolution(msg.into())
    }

    /// Create a new mDNS protocol error
    pub fn mdns<S: Into<String>>(msg: S) -> Self {
        Self::Mdns(msg.into())
    }

    /// Create a new UPnP protocol error
    pub fn upnp<S: Into<String>>(msg: S) -> Self {
        Self::Upnp(msg.into())
    }

    /// Create a new DNS-SD protocol error
    pub fn dns_sd<S: Into<String>>(msg: S) -> Self {
        Self::DnsSd(msg.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        Self::Network(msg.into())
    }

    /// Create a new timeout error
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create a new verification error
    pub fn verification<S: Into<String>>(msg: S) -> Self {
        Self::Verification(msg.into())
    }

    /// Create a new protocol error
    pub fn protocol<S: Into<String>>(msg: S) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create a new security error
    pub fn security<S: Into<String>>(msg: S) -> Self {
        Self::Security(msg.into())
    }

    /// Create a new other error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    /// Create a new invalid service error
    pub fn invalid_service<S: Into<String>>(msg: S) -> Self {
        Self::InvalidData(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Timeout(_) | Self::Protocol(_)
        )
    }

    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Configuration(_) | Self::InvalidData(_) => ErrorSeverity::Fatal,
            Self::Security(_) | Self::Verification(_) => ErrorSeverity::Error,
            Self::Network(_) | Self::DnsResolution(_) | Self::Protocol(_) => ErrorSeverity::Warning,
            Self::Timeout(_) => ErrorSeverity::Info,
            _ => ErrorSeverity::Warning,
        }
    }
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        assert_eq!(
            DiscoveryError::DnsResolution("test".to_string()).severity(),
            ErrorSeverity::Warning
        );
        assert_eq!(
            DiscoveryError::protocol("test".to_string()).severity(),
            ErrorSeverity::Warning
        );
    }

    #[test]
    fn test_error_retryable() {
        assert!(DiscoveryError::Timeout("5".to_string()).is_retryable());
        assert!(!DiscoveryError::invalid_service("test".to_string()).is_retryable());
    }
}
