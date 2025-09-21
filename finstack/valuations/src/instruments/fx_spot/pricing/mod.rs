//! FX Spot pricing module.
//!
//! This module houses pricing logic for the `FxSpot` instrument, following the
//! project convention of keeping pricing separate from instrument type
//! definitions and metric calculators. See `cds/pricing` for a more complex
//! example of this structure.
//!
//! Exposed components:
//! - `FxSpotPricer`: stateless pricer that computes instrument PV in the quote currency.
//!
//! The pricer delegates FX conversions to core library traits (`FxProvider`)
//! and uses `Money::convert` to enforce currency safety and rounding policy.

mod engine;

pub use engine::FxSpotPricer;
