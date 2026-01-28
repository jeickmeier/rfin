//! Market data model tests.
//!
//! Tests for market quote types, bumping, schema validation, and instrument building.
//!
//! ## Test Organization
//!
//! - `build/` - Instrument building from market quotes
//!   - `credit` - Credit instrument building from CDS quotes
//!   - `rates` - Rate instrument building from deposit/swap/FRA quotes
//! - `quote_bumps` - Tests for rate, spread, and vol bump operations
//! - `quote_schemas` - Schema validation and serialization tests for quote types

pub mod build;
mod quote_bumps;
mod quote_schemas;
