//! Inflation-Linked Bond (ILB) pricing module.
//!
//! Houses the pricing engine and helpers for deterministic schedule
//! construction and PV. All pricing-related functions for ILB should live
//! under this module to keep `types.rs` focused on data and API surface.

mod engine;
pub mod pricer;

pub use engine::InflationLinkedBondEngine;
