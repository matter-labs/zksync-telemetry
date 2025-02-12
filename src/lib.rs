// lib.rs
pub mod config;
pub mod error;
pub mod keys; // Make the module public
pub mod telemetry;
mod utils;

pub use config::TelemetryConfig;
pub use error::{TelemetryError, TelemetryResult};
pub use keys::TelemetryKeys;
pub use telemetry::Telemetry; // Re-export TelemetryKeys
