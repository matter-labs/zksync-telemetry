use crate::{TelemetryConfig, TelemetryError, TelemetryProps, TelemetryResult};
use once_cell::sync::OnceCell;
use posthog_rs::{
    CaptureExceptionOptions, ClientOptionsBuilder as PostHogClientOptionsBuilder,
    ErrorTrackingOptionsBuilder as PostHogErrorTrackingOptionsBuilder, Event,
};

pub struct Telemetry {
    config: TelemetryConfig,
    /// Whether the process-wide PostHog client was initialised for this instance.
    ///
    /// `posthog-rs` keeps a single global client per process: it is the only client that can
    /// capture panics, and telemetry is a process-wide singleton anyway (see [`init_telemetry`]).
    posthog: bool,
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
                // Panics go to Sentry when it is configured, otherwise to PostHog.
                Telemetry::init_posthog(key, app_name, app_version, sentry_dsn.is_none()).await?;
                true
            } else {
                false
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
            (false, None)
        };

        Ok(Self {
            config,
            posthog,
            sentry_guard,
        })
    }

    /// Initialises the global PostHog client.
    ///
    /// A client that is already initialised (e.g. by another [`Telemetry`] instance in the same
    /// process) is reused as is.
    async fn init_posthog(
        api_key: String,
        app_name: &str,
        app_version: &str,
        capture_panics: bool,
    ) -> TelemetryResult<()> {
        let error_tracking = PostHogErrorTrackingOptionsBuilder::default()
            .capture_panics(capture_panics)
            .build()
            .map_err(|e| TelemetryError::InitializationError(e.to_string()))?;

        let app = app_name.to_string();
        let version = app_version.to_string();
        let client_options = PostHogClientOptionsBuilder::default()
            .api_key(api_key)
            .error_tracking(error_tracking)
            // Attach the default properties to every event in one place: this also covers the
            // `$exception` events produced by the panic hook, which are not built by this crate.
            .before_send(move |mut event: Event| {
                Telemetry::add_posthog_default_props(&mut event, &app, &version);
                Some(event)
            })
            .build()
            .map_err(|e| TelemetryError::InitializationError(e.to_string()))?;

        match posthog_rs::init_global(client_options).await {
            Ok(()) | Err(posthog_rs::Error::AlreadyInitialized) => Ok(()),
            Err(e) => Err(TelemetryError::InitializationError(e.to_string())),
        }
    }

    pub async fn track_event(
        &self,
        event_name: &str,
        properties: TelemetryProps,
    ) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.posthog {
            let mut event = Event::new(event_name, self.config.instance_id.as_str());

            if let Some(props_map) = properties.to_map() {
                for (key, value) in props_map {
                    event
                        .insert_prop(key, value)
                        .map_err(|e| TelemetryError::SendError(e.to_string()))?;
                }
            }

            // `capture` only queues the event on a background worker; flush so that the event is
            // actually delivered before returning, as CLI processes tend to exit right after.
            posthog_rs::capture(event);
            posthog_rs::flush().await;
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
        } else if self.posthog {
            let options =
                CaptureExceptionOptions::new().distinct_id(self.config.instance_id.as_str());
            posthog_rs::capture_exception_with(*error, options)
                .await
                .map_err(|e| TelemetryError::SendError(e.to_string()))?;
            posthog_rs::flush().await;
        }

        Ok(())
    }

    fn add_posthog_default_props(event: &mut Event, app_name: &str, app_version: &str) {
        // Only serialization of the value can fail here, and these are plain strings.
        let _ = event.insert_prop("app", app_name);
        let _ = event.insert_prop("app_version", app_version);
        let _ = event.insert_prop("platform", std::env::consts::OS);
        let _ = event.insert_prop("zksync_telemetry_version", env!("CARGO_PKG_VERSION"));
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
    let telemetry = Telemetry::new(
        app_name,
        app_version,
        config_name,
        posthog_key,
        sentry_dsn,
        custom_config_path,
    )
    .await?;
    TELEMETRY
        .set(telemetry)
        .map_err(|_| anyhow::format_err!("Telemetry is already set"))
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

        let properties = TelemetryProps::new().insert("test", Some("value")).take();

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
        .await
        .unwrap();

        telemetry = get_telemetry();

        assert!(telemetry.is_some());
    }
}
