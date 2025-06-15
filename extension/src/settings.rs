use std::sync::Arc;

use arma_rs::{Context, ContextState, DirectReturn, Group, Value};
use dashmap::DashMap;

const WRITE_ONLY: [&str; 1] = ["OPENAI_API_KEY"];

pub fn group() -> Group {
    Group::new().command("set", cmd_set).command("get", cmd_get)
}

#[derive(Clone, Debug)]
pub struct Settings {
    inner: Arc<DashMap<String, Value>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inner: {
                let map = DashMap::new();
                map.insert(
                    "OPENAI_API_KEY".to_string(),
                    Value::String(
                        std::env::var("SAI_OPENAI_KEY").unwrap_or_else(|_| String::new()),
                    ),
                );
                Arc::new(map)
            },
        }
    }
}

impl Settings {
    pub fn set(&self, key: String, value: Value) {
        self.inner.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.get(key).map(|v| v.clone())
    }
}

#[allow(clippy::needless_pass_by_value)]
fn cmd_set(ctx: Context, key: String, value: Value) {
    let settings = ctx.global().get::<Settings>().unwrap_or_else(|| {
        ctx.global().set(Settings::default());
        ctx.global().get::<Settings>().unwrap()
    });
    settings.set(key, value);
}

#[allow(clippy::needless_pass_by_value)]
fn cmd_get(ctx: Context, key: String) -> Result<DirectReturn, String> {
    if WRITE_ONLY.contains(&key.as_str()) {
        return Err(format!("Key {key} is write-only"));
    }
    let settings = ctx.global().get::<Settings>().unwrap_or_else(|| {
        ctx.global().set(Settings::default());
        ctx.global().get::<Settings>().unwrap()
    });
    settings.get(&key).map_or_else(
        || Err(format!("Key {key} not found")),
        |value| Ok(Value::direct(value)),
    )
}
