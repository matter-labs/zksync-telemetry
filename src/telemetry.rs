use crate::{TelemetryConfig, TelemetryError, TelemetryResult, TelemetryProps};
use posthog_rs::{
    client, Client as PostHogClient, ClientOptionsBuilder as PostHogClientOptionsBuilder, Event,
    EventBase, Exception,
};
use sentry;
use std::sync::Arc;
use once_cell::sync::OnceCell;

pub struct Telemetry {
    app_name: String,
    app_version: String,
    config: TelemetryConfig,
    posthog: Option<PostHogClient>,
    sentry_guard: Option<sentry::ClientInitGuard>,
}

impl Telemetry {
    pub async fn new(
        app_name: &str,
        app_version: &str,
        config_name: &str,
        posthog_key: Option<String>,
        sentry_dsn: Option<String>,
        custom_config_path: Option<std::path::PathBuf>,
    ) -> TelemetryResult<Self> {
        let config = TelemetryConfig::new(config_name, custom_config_path)?;

        let (posthog, sentry_guard) = if config.enabled {
            let posthog = if let Some(key) = posthog_key {
                let app = app_name.to_string();
                let version = app_version.to_string();
                let client_options = PostHogClientOptionsBuilder::default()
                    .api_key(key)
                    .default_distinct_id(config.instance_id.clone())
                    .enable_panic_capturing(sentry_dsn.is_none())
                    .on_panic_exception(Some(Arc::new(move |panic_exception: &mut Exception| {
                        let _ =
                            Telemetry::add_posthog_default_props(panic_exception, &app, &version);
                    })))
                    .build()
                    .expect("Failed to build posthog client options");
                Some(client(client_options).await)
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
                    scope.set_tag("app_version", app_version);
                    scope.set_tag("platform", std::env::consts::OS);
                    scope.set_tag("zksync_telemetry_version", env!("CARGO_PKG_VERSION"));
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
            app_name: app_name.to_string(),
            app_version: app_version.to_string(),
            config,
            posthog,
            sentry_guard,
        })
    }

    pub async fn track_event(
        &self,
        event_name: &str,
        properties: TelemetryProps,
    ) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(client) = &self.posthog {
            let mut event = Event::new(event_name, &self.config.instance_id);

            if let Some(props_map) = properties.to_map() {
                for (key, value) in props_map {
                    event
                        .insert_prop(key, value)
                        .map_err(|e| TelemetryError::SendError(e.to_string()))?;
                }
            }
            Telemetry::add_posthog_default_props(&mut event, &self.app_name, &self.app_version)?;

            client
                .capture(event)
                .await
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn track_error(
        &self,
        error: Box<&(dyn std::error::Error + Send + Sync)>,
    ) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.sentry_guard.is_some() {
            sentry::capture_error(*error);
        } else if let Some(posthog_client) = &self.posthog {
            let mut exception = Exception::new(*error, &self.config.instance_id);
            Telemetry::add_posthog_default_props(
                &mut exception,
                &self.app_name,
                &self.app_version,
            )?;

            posthog_client
                .capture_exception(exception)
                .await
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        }

        Ok(())
    }

    fn add_posthog_default_props(
        event: &mut impl EventBase,
        app_name: &str,
        app_version: &str,
    ) -> TelemetryResult<()> {
        event
            .insert_prop("app", app_name)
            .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        event
            .insert_prop("app_version", app_version)
            .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        event
            .insert_prop("platform", std::env::consts::OS)
            .map_err(|e| TelemetryError::SendError(e.to_string()))?;
        event
            .insert_prop("zksync_telemetry_version", env!("CARGO_PKG_VERSION"))
            .map_err(|e| TelemetryError::SendError(e.to_string()))?;

        Ok(())
    }

    // No need for explicit shutdown now as the guard handles it
}
 
static TELEMETRY: OnceCell<Telemetry> = OnceCell::new();

pub async fn init_telemetry(
    app_name: &str,
    app_version: &str,
    config_name: &str,
    posthog_key: Option<String>,
    sentry_dsn: Option<String>,
    custom_config_path: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let telemetry = Telemetry::new(app_name, app_version, config_name, posthog_key, sentry_dsn, custom_config_path).await?;
    TELEMETRY.set(telemetry).map_err(|_| anyhow::format_err!("Telemetry is already set"))
}

pub fn get_telemetry() -> Option<&'static Telemetry> {
    TELEMETRY.get()
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

    #[tokio::test]
    async fn test_telemetry_disabled_by_default_in_tests() {
        let (_, config_path) = setup();

        let telemetry = Telemetry::new(
            "test-app",
            "1.0.0",
            "zksync-telemetry",
            Some("fake-key".to_string()),
            Some("fake-dsn".to_string()),
            Some(config_path.into()),
        )
        .await
        .unwrap();

        assert!(!telemetry.config.enabled);
    }

    #[tokio::test]
    async fn test_track_event_when_disabled() {
        let (_, config_path) = setup();

        let telemetry = Telemetry::new(
            "test-app",
            "1.0.0",
            "zksync-telemetry",
            None,
            None,
            Some(config_path.into()),
        )
        .await
        .unwrap();

        let properties = TelemetryProps::new()
            .insert(
                "test",
                Some("value"),
            ).take();

        assert!(telemetry
            .track_event("test_event", properties)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_sentry_error_capture() {
        let (_, config_path) = setup();

        let telemetry = Telemetry::new(
            "test-app",
            "1.0.0",
            "zksync-telemetry",
            None,
            Some("https://public@example.com/1".to_string()),
            Some(config_path.into()),
        )
        .await
        .unwrap();

        assert!(telemetry
            .track_error(Box::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "test error"
            )))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_posthog_error_capture() {
        let (_, config_path) = setup();

        let telemetry = Telemetry::new(
            "test-app",
            "1.0.0",
            "zksync-telemetry",
            Some("fake-key".to_string()),
            None,
            Some(config_path.into()),
        )
        .await
        .unwrap();

        assert!(telemetry
            .track_error(Box::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "test error"
            )))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_telemetry_init() {
        let (_, config_path) = setup();

        let mut telemetry = get_telemetry();
        assert!(telemetry.is_none());

        init_telemetry(
            "test-app",
            "1.0.0",
            "zksync-telemetry",
            Some("fake-key".to_string()),
            Some("fake-dsn".to_string()),
            Some(config_path.into()),
        )
        .await.unwrap();
        
        telemetry = get_telemetry();

        assert!(telemetry.is_some());
    }
}
