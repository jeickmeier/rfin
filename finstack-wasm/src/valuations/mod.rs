//! Valuation and risk APIs exposed to JS/TS.
//!
//! This module contains the WASM-facing wrappers around the Rust `finstack-valuations` crate:
//! instruments, curve calibration, pricing registry, results envelopes, and risk tooling.
//!
//! The primary entry point for pricing is `standardRegistry()` / `PricerRegistry`.
//!
//! @example
//! ```javascript
//! import init, { standardRegistry, MarketContext, FsDate } from "finstack-wasm";
//!
//! await init();
//! const registry = standardRegistry();
//! const market = new MarketContext();
//! const asOf = new FsDate(2024, 1, 2);
//! // registry.priceInstrument(instrument, "discounting", market, asOf)
//! ```

pub mod attribution;
pub mod calibration;
pub mod cashflow;
pub mod common;
pub mod conventions;
pub mod covenants;
pub mod dataframe;
pub mod instruments;
pub mod margin;
pub mod metrics;
pub mod performance;
pub mod pricer;
pub mod results;
pub mod risk;
