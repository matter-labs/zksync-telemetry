use serde_json::{Value, Map, Number};
use std::mem;

#[derive(Debug, Clone)]
pub struct TelemetryProps {
  inner: Value,
}

impl TelemetryProps {
  pub fn new() -> Self {
    Self {
      inner: Value::Object(Map::new()),
    }
  }

  pub fn from_str(value: &str) -> Self {
    Self {
      inner: Value::String(value.to_string()),
    }
  }

  pub fn from_string(value: String) -> Self {
    Self {
      inner: Value::String(value),
    }
  }

  pub fn from_number<T: Into<Number>>(value: T) -> Self {
    Self {
      inner: Value::Number(value.into()),
    }
  }

  pub fn from_bool(value: bool) -> Self {
    Self {
      inner: Value::Bool(value),
    }
  }

  pub fn from_array<T: Into<TelemetryProps>>(values: Vec<T>) -> Self {
    Self {
      inner: Value::Array(values.into_iter().map(|v| v.into().inner).collect()),
    }
  }

  pub fn insert<T>(&mut self, key: impl ToString, value: Option<T>) -> &mut Self
  where
    T: Into<TelemetryProps>
  {
    if let Some(props) = value {
      match &mut self.inner {
        Value::Object(map) => {
          map.insert(key.to_string(), props.into().inner);
        }
        _ => {
          let mut map = Map::new();
          map.insert(key.to_string(), props.into().inner);
          self.inner = Value::Object(map);
        }
      }
    }
    self
  }

  pub fn insert_with<K, V, RV, F>(&mut self, k: K, v: V, f: F) -> &mut Self
  where
    K: ToString,
    F: FnOnce(V) -> Option<RV>,
    RV: Into<TelemetryProps>
  {
    self.insert::<RV>(k, f(v));
    self
  }

  pub fn to_inner(self) -> Value {
    self.inner
  }

  pub fn to_map(self) -> Option<Map<String, Value>> {
    if let Value::Object(map) = self.inner {
      return Some(map)
    }
    None
  }

  pub fn take(&mut self) -> Self {
    mem::take(self)
  }
}

impl From<&str> for TelemetryProps {
  fn from(value: &str) -> Self {
      TelemetryProps::from_str(value)
  }
}

impl From<String> for TelemetryProps {
  fn from(value: String) -> Self {
      TelemetryProps::from_string(value)
  }
}

impl From<Number> for TelemetryProps {
  fn from(value: Number) -> Self {
      TelemetryProps::from_number(value)
  }
}

impl From<bool> for TelemetryProps {
  fn from(value: bool) -> Self {
      TelemetryProps::from_bool(value)
  }
}

impl From<Vec<TelemetryProps>> for TelemetryProps {
  fn from(value: Vec<TelemetryProps>) -> Self {
      TelemetryProps::from_array(value)
  }
}

impl Default for TelemetryProps {
  fn default() -> Self {
      Self::new()
  }
}
