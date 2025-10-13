//! Deposit-specific metrics module.
//!
//! Provides metric calculators for deposit instruments, split into focused
//! files for clarity and parity with other instruments. Metrics include:
//! - Year fraction (instrument day-count)
//! - Discount factors at start and end dates
//! - Par (simple) rate
//! - Implied end-date discount factor from a quoted rate
//! - Quoted rate passthrough
//!
//! See unit tests and `examples/` for usage.

mod df_end;
mod df_end_from_quote;
mod df_start;
mod dv01;
mod par_rate;
mod quote_rate;
mod theta;
mod year_fraction;

pub use df_end::DfEndCalculator;
pub use df_end_from_quote::DfEndFromQuoteCalculator;
pub use df_start::DfStartCalculator;
pub use dv01::DepositDv01Calculator;
pub use par_rate::DepositParRateCalculator;
pub use quote_rate::QuoteRateCalculator;
pub use year_fraction::YearFractionCalculator;

use crate::metrics::MetricRegistry;

/// Registers all deposit metrics to a registry.
///
/// Each metric is registered with the "Deposit" instrument type to ensure
/// proper applicability filtering.
pub fn register_deposit_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Deposit",
        metrics: [
            (Yf, YearFractionCalculator),
            (DfStart, DfStartCalculator),
            (DfEnd, DfEndCalculator),
            (DepositParRate, DepositParRateCalculator),
            (DfEndFromQuote, DfEndFromQuoteCalculator),
            (QuoteRate, QuoteRateCalculator),
            (Dv01, DepositDv01Calculator),
            (Theta, theta::ThetaCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01::<
                crate::instruments::Deposit,
            >::default()),
        ]
    };
}
