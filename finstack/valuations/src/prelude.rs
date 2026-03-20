//! Convenient re-exports for pricing and risk calculations.
//!
//! This module provides a single import point for the most frequently used types
//! in finstack-valuations, making it easier to get started with pricing and risk.
//!
//! Prefer typed rates in examples and downstream code when practical. In particular,
//! use [`finstack_core::types::Rate`] constructors instead of ambiguous raw
//! decimals when you want the example to communicate financial units clearly.
//!
//! Metrics returned from pricing calls are still governed by the semantic contract
//! documented on [`crate::metrics::MetricId`]; importing the prelude does not change
//! measure units, sign conventions, or bump conventions.
//!
//! # Example
//!
//! ```rust
//! use finstack_valuations::prelude::*;
//!
//! let registry = create_standard_registry();
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     Rate::from_percent(5.0),
//!     create_date(2025, time::Month::January, 15)?,
//!     create_date(2030, time::Month::January, 15)?,
//!     "USD-OIS"
//! );
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - Metric contract: [`crate::metrics::MetricId`]
//! - Result envelope: [`crate::results::ValuationResult`]

pub use crate::instruments::{Attributes, Instrument};

pub use crate::pricer::{
    create_standard_registry, InstrumentType, ModelKey, PricerRegistry, PricingError,
};

pub use crate::Result;

pub use crate::metrics::{standard_registry, MetricContext, MetricId, MetricRegistry};

pub use crate::results::{ResultsMeta, ValuationResult};

pub use crate::instruments::{
    Bond, CreditDefaultSwap, Deposit, EquityOption, FxForward, FxOption, FxSwap, InterestRateSwap,
    Repo, Swaption,
};

pub use finstack_core::prelude::*;
