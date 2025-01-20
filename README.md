# ZKsync Telemetry Library

A comprehensive telemetry solution for zkSync CLI applications that combines PostHog analytics and Sentry error tracking while maintaining user privacy and consent.

## Features

- ✅ Privacy-focused telemetry collection
- ✅ Opt-in by default
- ✅ Automatic CI environment detection
- ✅ Cross-platform support
- ✅ Configurable data collection
- ✅ Error tracking with context
- ✅ Usage analytics
- ✅ Persistent configuration

## Detailed Integration Guide

### 1. Add Dependency

Add the library to your `Cargo.toml`:
```toml
[dependencies]
zksync-telemetry = "0.1.0"
```

### 2. Initialize Telemetry

```rust
use zksync_telemetry::Telemetry;
use std::error::Error;

fn initialize_telemetry() -> Result<Telemetry, Box<dyn Error>> {
    let telemetry = Telemetry::new(
        "your-cli-name",                     // Name of your CLI application
        Some("your-posthog-key".to_string()),// PostHog API key
        Some("your-sentry-dsn".to_string()), // Sentry DSN
        None,                                // Use default config path
    )?;

    Ok(telemetry)
}
```

#### Configuration Options Explained:
- `app_name`: Used for config file location and analytics grouping
- `posthog_key`: Your PostHog API key (optional)
- `sentry_dsn`: Your Sentry DSN (optional)
- `custom_config_path`: Override default config location (optional)

### 3. Track Events

```rust
use std::collections::HashMap;

fn track_cli_usage(telemetry: &Telemetry, command: &str) -> Result<(), Box<dyn Error>> {
    let mut properties = HashMap::new();
    
    // Add event properties
    properties.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    properties.insert(
        "os".to_string(),
        serde_json::Value::String(std::env::consts::OS.to_string()),
    );

    // Track the event
    telemetry.track_event("command_executed", properties)?;
    
    Ok(())
}
```

### 4. Track Errors

```rust
fn handle_operation(telemetry: &Telemetry) -> Result<(), Box<dyn Error>> {
    match some_risky_operation() {
        Ok(result) => Ok(result),
        Err(error) => {
            // Track the error
            telemetry.track_error(&error)?;
            Err(error.into())
        }
    }
}
```

### 5. Complete Integration Example

```rust
use zksync_telemetry::{Telemetry, TelemetryConfig};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize telemetry
    let telemetry = Telemetry::new(
        "my-cli-app",
        Some("ph_key".to_string()),
        Some("sentry_dsn".to_string()),
        None,
    )?;

    // Use throughout your application
    let mut properties = HashMap::new();
    properties.insert(
        "action".to_string(),
        serde_json::Value::String("start".to_string()),
    );

    // Track application start
    telemetry.track_event("app_start", properties)?;

    // Your application logic here
    match do_something_important() {
        Ok(_) => {
            let mut success_props = HashMap::new();
            success_props.insert(
                "status".to_string(),
                serde_json::Value::String("success".to_string()),
            );
            telemetry.track_event("operation_complete", success_props)?;
        }
        Err(e) => {
            telemetry.track_error(&e)?;
        }
    }

    Ok(())
}
```

### 6. Managing User Consent

Users can update their telemetry consent:

```rust
use zksync_telemetry::TelemetryConfig;

fn update_telemetry_settings(enabled: bool) -> Result<(), Box<dyn Error>> {
    let mut config = TelemetryConfig::new(
        "my-cli-app",
        None,  // Use default config path
    )?;

    config.update_consent(enabled)?;
    Ok(())
}
```

### 7. API Key Management

The library provides flexible management of PostHog and Sentry API keys through the `TelemetryKeys` structure. Both keys are optional, but telemetry features will be disabled for services without valid keys.

#### Default Usage

```rust
use zksync_telemetry::{Telemetry, TelemetryKeys};

fn main() {
    // TelemetryKeys::new() now returns Result
    let keys = TelemetryKeys::new()
        .expect("Failed to initialize telemetry keys");
        
    let telemetry = Telemetry::new(
        "your-app-name",
        keys.posthog_key,    // Now Option<String>
        keys.sentry_dsn,     // Now Option<String>
        None,
    ).expect("Failed to initialize telemetry");
}
```

#### Environment Variables
The library looks for and validates the following environment variables:

`ANVIL_POSTHOG_KEY`: PostHog API key (must start with 'phc_')
`ANVIL_SENTRY_DSN`: Sentry DSN (must be a valid Sentry URL)

Example:
```bash
# Valid PostHog key starting with 'phc_'
export ANVIL_POSTHOG_KEY="phc_your_actual_posthog_key"
# Valid Sentry DSN URL
export ANVIL_SENTRY_DSN="https://your_key@sentry.io/your_project"
./your-application
```

#### Custom Keys

You can also provide custom keys programmatically:
```rust
// Both keys are now optional
let keys = TelemetryKeys::with_keys(
    Some("phc_your_posthog_key".to_string()),
    Some("https://your_key@sentry.io/your_project".to_string()),
).expect("Invalid keys provided");

// Or with only PostHog
let posthog_only = TelemetryKeys::with_keys(
    Some("phc_your_posthog_key".to_string()),
    None,
).expect("Invalid PostHog key");
```

#### Key Validation
The library validates keys before accepting them:

+ PostHog keys must start with phc_
+ Sentry DSNs must be valid URLs starting with 'http' and containing '@sentry.io'
+ Invalid keys will result in an error
+ Missing keys will disable corresponding features

#### Security Considerations

+ No default keys are provided - valid keys must be supplied
+ Keys can be rotated by using environment variables
+ No sensitive information is collected or transmitted
+ Different keys can be used for different environments (development, staging, production)
+ The library will function without keys, but telemetry will be disabled

#### Best Practices

+ Store keys securely using environment variables
+ Rotate keys periodically
+ Use different keys for different environments
+ Monitor key usage through PostHog/Sentry dashboards
+ Consider disabling telemetry in development/test environments
+ Validate key format before using them

### 8. Important Notes

#### Configuration Storage
- Unix/Linux: `$XDG_CONFIG_HOME/.<app_name>/telemetry.json`
- macOS: `~/Library/Application Support/com.matter-labs.<app_name>/telemetry.json`
- Windows: `%APPDATA%\matter-labs\<app_name>\telemetry.json`
- Custom location can be specified via `custom_config_path`

For example, if your CLI app is named "era-test-node":
- macOS: `/Users/<username>/Library/Application Support/com.matter-labs.era-test-node/telemetry.json`
- Linux: `~/.config/era-test-node/telemetry.json`
- Windows: `C:\Users\<username>\AppData\Roaming\matter-labs\era-test-node\telemetry.json`

#### CI Environment Detection
- Automatically detects CI environments
- Disables telemetry prompts in non-interactive environments
- Supports major CI platforms (GitHub Actions, Jenkins, Travis, etc.)

#### Privacy Considerations
- Only collects explicitly specified data
- No PII collection
- All data collection is opt-in
- Users can opt-out at any time
- Configuration is stored locally
- No automatic data collection

#### Collected Data
The library collects:
- Basic usage statistics (commands used)
- Error reports (without sensitive data)
- Platform information (OS, version)
- CLI configuration (non-sensitive settings)

Does NOT collect:
- Personal information
- Sensitive configuration
- Private keys or addresses
- User-specific data
- File paths or system information
