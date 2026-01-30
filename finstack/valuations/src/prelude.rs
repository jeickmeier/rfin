//! Convenient re-exports for pricing and risk calculations.
//!
//! This module provides a single import point for the most frequently used types
//! in finstack-valuations, making it easier to get started with pricing and risk.
//!
//! # Example
//!
//! ```rust
//! use finstack_valuations::prelude::*;
//!
//! let registry = create_standard_registry();
//! let bond = Bond::fixed_rate(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     Rate::from_percent(5.0),
//!     create_date(2025, time::Month::January, 15)?,
//!     create_date(2030, time::Month::January, 15)?,
//!     "USD-OIS"
//! );
//! # Ok::<(), finstack_core::Error>(())
//! ```

pub use crate::instruments::{Attributes, Instrument, InstrumentNpvExt};

pub use crate::pricer::{create_standard_registry, InstrumentType, ModelKey, PricerRegistry};

pub use crate::metrics::{standard_registry, MetricContext, MetricId, MetricRegistry};

pub use crate::results::{ResultsMeta, ValuationResult};

pub use crate::instruments::{
    Bond, CreditDefaultSwap, Deposit, EquityOption, FxForward, FxOption, FxSwap, InterestRateSwap,
    Repo, Swaption,
};

pub use finstack_core::prelude::*;
