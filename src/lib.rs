// lib.rs
pub mod config;
pub mod error;
pub mod telemetry;
pub mod keys;  // Make the module public
mod utils;

pub use config::TelemetryConfig;
pub use error::{TelemetryError, TelemetryResult};
pub use telemetry::Telemetry;
pub use keys::TelemetryKeys;  // Re-export TelemetryKeys