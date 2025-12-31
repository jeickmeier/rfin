//! Schema Validation Tests
//!
//! Tests ensuring schema files stay synchronized with Rust types:
//!
//! - [`parity`]: JSON schema ↔ Rust type synchronization
//! - [`ts_export`]: TypeScript type generation (requires `ts_export` feature)

pub mod parity;
#[cfg(feature = "ts_export")]
pub mod ts_export;
