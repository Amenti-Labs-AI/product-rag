use std::collections::HashMap;

use qdrant_client::qdrant::Value;

pub fn get_str(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
    payload.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub fn get_i64(payload: &HashMap<String, Value>, key: &str) -> Option<i64> {
    payload.get(key).and_then(|v| v.as_integer())
}

pub fn get_f64(payload: &HashMap<String, Value>, key: &str) -> Option<f64> {
    payload.get(key).and_then(|v| {
        v.as_double()
            .or_else(|| v.as_integer().map(|i| i as f64))
    })
}

pub fn get_bool(payload: &HashMap<String, Value>, key: &str) -> Option<bool> {
    payload.get(key).and_then(|v| v.as_bool())
}
