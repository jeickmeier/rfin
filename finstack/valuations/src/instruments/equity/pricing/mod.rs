//! Equity pricing module.
//!
//! This module houses pricing logic for the `Equity` instrument, following the
//! repository convention of keeping pricing separate from instrument type
//! definitions and metric calculators. See `fx_spot/pricing` and `cds/pricing`
//! for examples of this structure.
//!
//! Exposed components:
//! - `EquityPricer`: stateless pricer computing PV as `price_per_share * shares`.

mod pricer;

pub use pricer::EquityPricer;


