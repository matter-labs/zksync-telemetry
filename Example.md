# Detailed Integration Example: anvil-node

## Real-World Integration Example

This section demonstrates how to integrate the telemetry library into `anvil-node`, a CLI application with multiple configuration options and commands.

### 1. Basic Setup

```rust
use zksync_telemetry::Telemetry;
use std::collections::HashMap;
use clap::ArgMatches;

pub struct AnvilNode {
    telemetry: Telemetry,
    // ... other anvil-node fields
}

impl AnvilNode {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize telemetry
        let telemetry = Telemetry::new(
            "anvil-node",
            Some(std::env::var("POSTHOG_API_KEY")
                .unwrap_or_else(|_| "your-posthog-key".to_string())),
            Some(std::env::var("SENTRY_DSN")
                .unwrap_or_else(|_| "your-sentry-dsn".to_string())),
            None,  // Use default config path
        )?;

        Ok(Self {
            telemetry,
            // ... initialize other fields
        })
    }
}
```

### 2. Configuration Tracking

Track how users are configuring and using the node:

```rust
impl AnvilNode {
    pub fn track_configuration(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let mut properties = HashMap::new();

        // Track basic node configuration
        self.track_basic_config(&mut properties, args);
        // Track network settings
        self.track_network_config(&mut properties, args);
        // Track debugging options
        self.track_debug_config(&mut properties, args);
        // Track gas configuration
        self.track_gas_config(&mut properties, args);
        // Track system configuration
        self.track_system_config(&mut properties, args);
        // Track cache settings
        self.track_cache_config(&mut properties, args);

        // Send telemetry event
        self.telemetry.track_event("node_started", properties)?;
        Ok(())
    }

    fn track_basic_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        // Track which command was used
        let command = if args.contains_id("fork") {
            "fork"
        } else if args.contains_id("replay_tx") {
            "replay_tx"
        } else {
            "run"
        };
        properties.insert(
            "command".to_string(),
            serde_json::Value::String(command.to_string())
        );

        // Track basic flags
        properties.insert(
            "no_mining".to_string(),
            serde_json::Value::Bool(args.get_flag("no-mining"))
        );
        properties.insert(
            "offline".to_string(),
            serde_json::Value::Bool(args.get_flag("offline"))
        );
    }

    fn track_network_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        // Track port configuration
        if let Some(port) = args.get_one::<u16>("port") {
            properties.insert(
                "custom_port".to_string(),
                serde_json::Value::Bool(*port != 8011)  // Track if default port was changed
            );
        }

        // Track chain ID configuration
        if let Some(chain_id) = args.get_one::<u64>("chain-id") {
            properties.insert(
                "custom_chain_id".to_string(),
                serde_json::Value::Bool(*chain_id != 260)  // Track if default chain-id was changed
            );
        }
    }

    fn track_debug_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        // Track debug mode
        properties.insert(
            "debug_mode".to_string(),
            serde_json::Value::Bool(args.get_flag("debug-mode"))
        );

        // Track logging configuration
        if let Some(log_level) = args.get_one::<String>("log") {
            properties.insert(
                "log_level".to_string(),
                serde_json::Value::String(log_level.clone())
            );
        }

        // Track other debug settings
        for setting in ["show-node-config", "show-tx-summary", "show-calls", "resolve-hashes"] {
            if let Some(value) = args.get_one::<bool>(setting) {
                properties.insert(
                    setting.replace('-', "_").to_string(),
                    serde_json::Value::Bool(*value)
                );
            }
        }
    }

    fn track_gas_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        // Track custom gas configurations
        let gas_options = ["l1-gas-price", "l2-gas-price", "l1-pubdata-price"];
        for option in gas_options {
            if args.contains_id(option) {
                properties.insert(
                    format!("custom_{}", option.replace('-', "_")),
                    serde_json::Value::Bool(true)
                );
            }
        }
    }

    fn track_system_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        // Track system contract configuration
        if let Some(contracts) = args.get_one::<String>("dev-system-contracts") {
            properties.insert(
                "system_contracts_type".to_string(),
                serde_json::Value::String(contracts.clone())
            );
        }

        properties.insert(
            "emulate_evm".to_string(),
            serde_json::Value::Bool(args.get_flag("emulate-evm"))
        );
    }

    fn track_cache_config(&self, properties: &mut HashMap<String, serde_json::Value>, args: &ArgMatches) {
        if let Some(cache_type) = args.get_one::<String>("cache") {
            properties.insert(
                "cache_type".to_string(),
                serde_json::Value::String(cache_type.clone())
            );
        }

        properties.insert(
            "reset_cache".to_string(),
            serde_json::Value::Bool(args.get_flag("reset-cache"))
        );
    }
}
```

### 3. Error Tracking

Track node-specific errors:

```rust
impl AnvilNode {
    pub fn handle_operation<T>(&self, operation: impl FnOnce() -> Result<T, Box<dyn std::error::Error>>) 
        -> Result<T, Box<dyn std::error::Error>> 
    {
        match operation() {
            Ok(result) => Ok(result),
            Err(error) => {
                // Track the error
                self.telemetry.track_error(&error)?;
                // You might want to add custom context
                let mut properties = HashMap::new();
                properties.insert(
                    "operation_type".to_string(),
                    serde_json::Value::String("node_operation".to_string())
                );
                self.telemetry.track_event("operation_failed", properties)?;
                Err(error)
            }
        }
    }
}
```

### 4. Usage in Main

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup clap command and parse arguments
    let args = setup_cli().get_matches();

    // Initialize node with telemetry
    let node = AnvilNode::new()?;
    
    // Track initial configuration
    node.track_configuration(&args)?;

    // Run node with error tracking
    node.handle_operation(|| {
        // Node startup logic
        Ok(())
    })?;

    Ok(())
}
```

### 5. Example Telemetry Data

This setup will collect anonymous usage data like:

```json
{
    "event": "node_started",
    "properties": {
        "command": "run",
        "no_mining": false,
        "offline": false,
        "custom_port": false,
        "custom_chain_id": false,
        "debug_mode": true,
        "log_level": "info",
        "show_node_config": true,
        "custom_l1_gas_price": false,
        "custom_l2_gas_price": false,
        "system_contracts_type": "built-in",
        "emulate_evm": false,
        "cache_type": "disk",
        "reset_cache": false
    }
}
```

This data helps answer questions like:
- What percentage of users run in debug mode?
- Which commands are most popular?
- What cache types are preferred?
- How many users customize gas prices?
- What logging levels are commonly used?

### 6. Privacy Considerations

The example above carefully tracks only:
- Configuration choices
- Feature usage
- Non-sensitive settings
- Error patterns

It explicitly avoids collecting:
- Network addresses
- Account information
- Custom paths
- Environment-specific data
- Private keys or sensitive configs

### 5. Example Telemetry Data

This setup will collect anonymous usage data. Here's a representation of what gets sent to PostHog:

```json
{
    "event": "node_started",
    "properties": {
        "command": "run",
        "no_mining": false,
        "offline": false,
        "custom_port": false,
        "custom_chain_id": false,
        "debug_mode": true,
        "log_level": "info",
        "show_node_config": true,
        "custom_l1_gas_price": false,
        "custom_l2_gas_price": false,
        "system_contracts_type": "built-in",
        "emulate_evm": false,
        "cache_type": "disk",
        "reset_cache": false
    }
}
```

### Data Visualization and Analysis
The JSON above is just a representation of the data format. The actual data is sent to:

#### 1. PostHog Analytics Dashboard

+ Data is automatically aggregated and visualized
+ Create custom insights like:
    + Command usage distribution (Pie Charts)
    + Debug mode adoption over time (Line Graphs)
    + Cache type preferences (Bar Charts)
    + Feature usage funnels
+ Query examples:
```sql
Copy-- Get percentage of users using debug mode
SELECT 
    count(distinct_id) as total_users,
    countIf(properties.$debug_mode = true) / count(distinct_id) as debug_mode_percentage
FROM events
WHERE event = 'node_started'
GROUP BY date_trunc('week', timestamp)
```


#### 2. Sentry Error Dashboard

+ Automatic error grouping and categorization
+ Stack trace visualization
+ Error frequency trends
+ Context-rich error reports
+ Environment information

The dashboards in both services provide real-time insights into:

+ How users configure anvil-node
+ Which features are most used
+ Common error patterns
+ Usage trends over time

You don't need to process any JSON files manually - PostHog and Sentry handle all the visualization and analysis automatically through their web interfaces.