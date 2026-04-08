//! JSON Schema proxy types for external types lacking `JsonSchema`.
//!
//! Provides schema-generating proxies for third-party types (e.g., `time::Date`)
//! that can be used with `#[schemars(with = "...")]` field attributes.

use schemars::JsonSchema;
use std::borrow::Cow;

/// Schema proxy for `time::Date` — produces `{"type": "string", "format": "date"}`.
///
/// Usage on fields: `#[schemars(with = "finstack_core::schema_defs::DateSchema")]`
pub struct DateSchema;

impl JsonSchema for DateSchema {
    fn schema_name() -> Cow<'static, str> {
        "Date".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "format": "date"
        })
    }
}
