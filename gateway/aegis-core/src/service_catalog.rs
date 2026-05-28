//! Service catalogue — one-line business-impact text for each known service.
//!
//! The catalogue lets Aegis answer the question Splunk's buyer's guide
//! flags as universally important: *"What does this mean for the business?"*
//! When a chain points at `payment-api`, we don't surface a dollar figure
//! (we don't know your contract) — but we *can* say "payment-api handles
//! all transaction processing", which is enough context for an on-call
//! engineer at 2am.
//!
//! Operators load the catalogue from TOML config:
//!
//! ```toml
//! [services]
//! payment-api  = "Handles all transaction processing."
//! checkout     = "Customer-facing checkout flow."
//! orders       = "Order fulfilment pipeline."
//! ```
//!
//! Unknown services simply have no business-impact line attached to their
//! decision card — that's fine and intentional.

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct ServiceCatalog {
    inner: Arc<HashMap<String, String>>,
}

impl ServiceCatalog {
    /// Build a catalogue from `(name, impact)` pairs.
    pub fn with_entries(entries: &[(&str, &str)]) -> Self {
        let mut map = HashMap::with_capacity(entries.len());
        for (k, v) in entries {
            map.insert((*k).to_string(), (*v).to_string());
        }
        Self { inner: Arc::new(map) }
    }

    /// Build a catalogue from an owned map (e.g. loaded from TOML).
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self { inner: Arc::new(map) }
    }

    pub fn lookup(&self, service: &str) -> Option<&str> {
        self.inner.get(service).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_known_service_impact() {
        let c = ServiceCatalog::with_entries(&[("payment-api", "Money flows.")]);
        assert_eq!(c.lookup("payment-api"), Some("Money flows."));
    }

    #[test]
    fn lookup_returns_none_for_unknown_service() {
        let c = ServiceCatalog::default();
        assert!(c.lookup("payment-api").is_none());
    }
}
