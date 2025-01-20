//! Telemetry key management for PostHog and Sentry integration.
use crate::error::{TelemetryError, TelemetryResult};

/// Structure holding API keys for telemetry services
#[derive(Clone, Debug)]
pub struct TelemetryKeys {
    pub posthog_key: Option<String>,
    pub sentry_dsn: Option<String>,
}

impl TelemetryKeys {
    /// Creates new instance with keys from environment
    pub fn new() -> TelemetryResult<Self> {
        Ok(Self {
            posthog_key: Self::get_posthog_key()?,
            sentry_dsn: Self::get_sentry_dsn()?,
        })
    }

    /// Retrieves PostHog API key from environment
    fn get_posthog_key() -> TelemetryResult<Option<String>> {
        match std::env::var("ANVIL_POSTHOG_KEY") {
            Ok(key) if !key.trim().is_empty() => {
                if !key.starts_with("phc_") {
                    return Err(TelemetryError::ConfigError(
                        "Invalid PostHog key format. Must start with 'phc_'".to_string()
                    ));
                }
                Ok(Some(key))
            }
            _ => Ok(None)
        }
    }

    /// Retrieves Sentry DSN from environment
    fn get_sentry_dsn() -> TelemetryResult<Option<String>> {
        match std::env::var("ANVIL_SENTRY_DSN") {
            Ok(dsn) if !dsn.trim().is_empty() => {
                // Basic Sentry DSN validation
                if !dsn.starts_with("http") || !dsn.contains("@sentry.io") {
                    return Err(TelemetryError::ConfigError(
                        "Invalid Sentry DSN format".to_string()
                    ));
                }
                Ok(Some(dsn))
            }
            _ => Ok(None)
        }
    }

    /// Creates an instance with custom keys
    pub fn with_keys(
        posthog_key: Option<String>,
        sentry_dsn: Option<String>
    ) -> TelemetryResult<Self> {
        // Validate PostHog key if provided
        if let Some(key) = &posthog_key {
            if !key.starts_with("phc_") {
                return Err(TelemetryError::ConfigError(
                    "Invalid PostHog key format. Must start with 'phc_'".to_string()
                ));
            }
        }

        // Validate Sentry DSN if provided
        if let Some(dsn) = &sentry_dsn {
            if !dsn.starts_with("http") || !dsn.contains("@sentry.io") {
                return Err(TelemetryError::ConfigError(
                    "Invalid Sentry DSN format".to_string()
                ));
            }
        }

        Ok(Self {
            posthog_key,
            sentry_dsn,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_keys() {
        let valid_keys = TelemetryKeys::with_keys(
            Some("phc_validkey123".to_string()),
            Some("https://key@sentry.io/123".to_string()),
        );
        assert!(valid_keys.is_ok());

        let invalid_posthog = TelemetryKeys::with_keys(
            Some("invalid_key".to_string()),
            None,
        );
        assert!(invalid_posthog.is_err());

        let invalid_sentry = TelemetryKeys::with_keys(
            None,
            Some("invalid_dsn".to_string()),
        );
        assert!(invalid_sentry.is_err());
    }

    #[test]
    fn test_env_vars() {
        unsafe {
            std::env::set_var("ANVIL_POSTHOG_KEY", "phc_testkey123");
            std::env::set_var("ANVIL_SENTRY_DSN", "https://test@sentry.io/123");
        }
        
        let keys = TelemetryKeys::new().unwrap();
        assert_eq!(keys.posthog_key.unwrap(), "phc_testkey123");
        assert_eq!(keys.sentry_dsn.unwrap(), "https://test@sentry.io/123");
    }
}