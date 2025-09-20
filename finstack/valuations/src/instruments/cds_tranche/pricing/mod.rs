//! CDS Tranche pricing facade and engine re-export.
//!
//! Exposes the pricing entrypoints for `CdsTranche`. Core pricing logic
//! lives in `engine`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.
//!
//! Public surface:
//! - `CDSTranchePricer` – pricing and risk using a Gaussian Copula with base correlation
//! - `CDSTranchePricerConfig` – configuration for numerical/policy options

pub mod engine;

// Stable re-export aligned with other instruments (e.g., `CDSPricer` / `CDSIndexPricer`).
pub use engine::CDSTranchePricer;
pub use engine::CDSTranchePricerConfig;


