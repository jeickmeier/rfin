//! Common functionality shared across multiple instruments.
//!
//! This module contains utilities, models, and types that are used
//! by multiple instrument implementations, including:
//! - Core instrument traits (Instrument)
//! - NPV calculation interfaces (Discountable)
//! - Option pricing models (Black-Scholes, binomial/trinomial trees, SABR)
//! - Common helper functions
//! - Shared data structures and enums

// Core instrument traits and metadata
pub(crate) mod traits;

// Unified dependency representation
pub(crate) mod dependencies;

// NPV calculation interface
pub(crate) mod discountable;

// Shared utilities and helper functions
pub(crate) mod helpers;
// Shared volatility override/surface resolution.
#[cfg(feature = "mc")]
pub(crate) mod two_clock;
pub(crate) mod validation;
pub(crate) mod vol_resolution;

// Common parameter types shared across instruments
pub(crate) mod fx_dates;
pub(crate) mod parameters;

// Option pricing models and frameworks (includes closed-form, volatility, and tree models)
pub(crate) mod models;

// Common pricing patterns and infrastructure
pub(crate) mod pricing;

// Periodized present value calculations
pub(crate) mod period_pv;

// Re-export pricer helper used by instrument pricer modules.
#[doc(hidden)]
pub(crate) use pricing::GenericInstrumentPricer;
