use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Failed to initialize telemetry: {0}")]
    InitializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to send telemetry data: {0}")]
    SendError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("PostHog client error: {0}")]
    PostHogError(String),

    #[error("Sentry client error: {0}")]
    SentryError(String),

    #[error("Invalid configuration path: {0}")]
    InvalidPath(String),

    #[error("Environment error: {0}")]
    EnvironmentError(String),

    #[error("Permission denied: {0}")]
    PermissionError(String),
}

// Type alias for Result with our error type
pub type TelemetryResult<T> = Result<T, TelemetryError>;