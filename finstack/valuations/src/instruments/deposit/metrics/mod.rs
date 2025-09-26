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
mod par_rate;
mod quote_rate;
mod risk_bucketed_dv01;
mod year_fraction;

pub use df_end::DfEndCalculator;
pub use df_end_from_quote::DfEndFromQuoteCalculator;
pub use df_start::DfStartCalculator;
pub use par_rate::DepositParRateCalculator;
pub use quote_rate::QuoteRateCalculator;
pub use risk_bucketed_dv01::BucketedDv01Calculator;
pub use year_fraction::YearFractionCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Registers all deposit metrics to a registry.
///
/// Each metric is registered with the "Deposit" instrument type to ensure
/// proper applicability filtering.
pub fn register_deposit_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(MetricId::Yf, Arc::new(YearFractionCalculator), &["Deposit"]) // accrual year fraction
        .register_metric(MetricId::DfStart, Arc::new(DfStartCalculator), &["Deposit"]) // DF at start
        .register_metric(MetricId::DfEnd, Arc::new(DfEndCalculator), &["Deposit"]) // DF at end
        .register_metric(
            MetricId::DepositParRate,
            Arc::new(DepositParRateCalculator),
            &["Deposit"],
        ) // par simple rate
        .register_metric(
            MetricId::DfEndFromQuote,
            Arc::new(DfEndFromQuoteCalculator),
            &["Deposit"],
        ) // implied DF(end)
        .register_metric(
            MetricId::QuoteRate,
            Arc::new(QuoteRateCalculator),
            &["Deposit"],
        ) // quoted rate passthrough
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(BucketedDv01Calculator),
            &["Deposit"],
        );
}
