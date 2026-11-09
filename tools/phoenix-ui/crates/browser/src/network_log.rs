//! Request outcomes of one page for capture reports: method, URL, status and
//! failure reason. Bodies are not kept (see `response_capture` for that).

use std::collections::HashMap;

use serde_json::Value;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct NetworkEntry {
    pub method: String,
    pub url: String,
    pub resource_type: String,
    pub status: Option<i64>,
    pub error: Option<String>,
    pub canceled: bool,
    pub blocked_reason: Option<String>,
}

#[derive(Default)]
pub struct NetworkLog {
    order: Vec<String>,
    entries: HashMap<String, NetworkEntry>,
}

impl NetworkLog {
    pub fn event(&mut self, method: &str, params: &Value) {
        let Some(id) = params.get("requestId").and_then(Value::as_str) else {
            return;
        };
        let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or("").to_string();
        match method {
            "Network.requestWillBeSent" => {
                if !self.entries.contains_key(id) {
                    if self.order.len() >= MAX_ENTRIES {
                        return;
                    }
                    self.order.push(id.to_string());
                }
                // A redirect reuses the request id; the entry follows the hop.
                self.entries.insert(
                    id.to_string(),
                    NetworkEntry {
                        method: text(params.pointer("/request/method")),
                        url: text(params.pointer("/request/url")),
                        resource_type: text(params.get("type")),
                        ..Default::default()
                    },
                );
            }
            "Network.responseReceived" => {
                if let Some(entry) = self.entries.get_mut(id) {
                    entry.status = params
                        .pointer("/response/status")
                        .and_then(Value::as_f64)
                        .map(|s| s as i64);
                }
            }
            "Network.loadingFailed" => {
                if let Some(entry) = self.entries.get_mut(id) {
                    entry.error = Some(text(params.get("errorText")));
                    entry.canceled = params.get("canceled").and_then(Value::as_bool) == Some(true);
                    entry.blocked_reason = params
                        .get("blockedReason")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
            }
            _ => {}
        }
    }

    pub fn entries(&self) -> Vec<NetworkEntry> {
        self.order
            .iter()
            .filter_map(|id| self.entries.get(id).cloned())
            .collect()
    }
}
