use posthog_rs::{client, Client as PostHogClient, Event};
use sentry;
use std::collections::HashMap;
use crate::{TelemetryConfig, TelemetryError, TelemetryResult};

pub struct Telemetry {
    config: TelemetryConfig,
    posthog: Option<PostHogClient>,
    sentry_guard: Option<sentry::ClientInitGuard>,
}

impl Telemetry {
    pub fn new(
        app_name: &str,
        posthog_key: Option<String>,
        sentry_dsn: Option<String>,
        custom_config_path: Option<std::path::PathBuf>,
    ) -> TelemetryResult<Self> {
        let config = TelemetryConfig::new(app_name, custom_config_path)?;

        let (posthog, sentry_guard) = if config.enabled {
            let posthog = if let Some(key) = posthog_key {
                Some(client(key.as_str()))
            } else {
                None
            };

            let sentry_guard = if let Some(dsn) = sentry_dsn {
                let options = sentry::ClientOptions {
                    release: Some(env!("CARGO_PKG_VERSION").into()),
                    ..Default::default()
                };
                
                // Initialize Sentry and store the guard
                let guard = sentry::init((dsn, options));

                // Configure scope with default tags
                sentry::configure_scope(|scope| {
                    scope.set_tag("app", app_name);
                    scope.set_tag("version", env!("CARGO_PKG_VERSION"));
                    scope.set_tag("platform", std::env::consts::OS);
                });

                Some(guard)
            } else {
                None
            };

            (posthog, sentry_guard)
        } else {
            (None, None)
        };

        Ok(Self {
            config,
            posthog,
            sentry_guard,
        })
    }

    pub fn track_event(
        &self,
        event_name: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(client) = &self.posthog {
            let mut event = Event::new(
                event_name, 
                &self.config.instance_id
            );

            // Add all properties
            for (key, value) in properties {
                event.insert_prop(key, value)
                    .map_err(|e| TelemetryError::SendError(e.to_string()))?;
            }

            // Add default properties
            event.insert_prop("platform", std::env::consts::OS)
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;
            
            event.insert_prop("version", env!("CARGO_PKG_VERSION"))
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;

            client.capture(event)
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        }

        Ok(())
    }

    pub fn track_error(&self, error: &dyn std::error::Error) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.sentry_guard.is_some() {
            sentry::capture_error(error);
        }

        Ok(())
    }

    // No need for explicit shutdown now as the guard handles it
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("telemetry.json");
        (temp_dir, config_path.to_str().unwrap().to_string())
    }

    #[test]
    fn test_telemetry_disabled_by_default_in_tests() {
        let (_, config_path) = setup();
        
        let telemetry = Telemetry::new(
            "test-app",
            Some("fake-key".to_string()),
            Some("fake-dsn".to_string()),
            Some(config_path.into()),
        ).unwrap();

        assert!(!telemetry.config.enabled);
    }

    #[test]
    fn test_track_event_when_disabled() {
        let (_, config_path) = setup();
        
        let telemetry = Telemetry::new(
            "test-app",
            None,
            None,
            Some(config_path.into()),
        ).unwrap();

        let mut properties = HashMap::new();
        properties.insert(
            "test".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        assert!(telemetry.track_event("test_event", properties).is_ok());
    }

    #[test]
    fn test_sentry_error_capture() {
        let (_, config_path) = setup();
        
        let telemetry = Telemetry::new(
            "test-app",
            None,
            Some("https://public@example.com/1".to_string()),
            Some(config_path.into()),
        ).unwrap();

        let events = sentry::test::with_captured_events(|| {
            let error = std::io::Error::new(
                std::io::ErrorKind::Other,
                "test error"
            );
            telemetry.track_error(&error).unwrap();
        });

        // No events should be captured because telemetry is disabled by default in tests
        assert_eq!(events.len(), 0);
    }
}