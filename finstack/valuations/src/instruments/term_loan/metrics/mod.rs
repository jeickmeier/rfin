//! Metrics for Term Loan instruments.

mod dv01;
mod cs01;
mod ytm;
mod ytw;
mod ytc;
mod ytn;
mod all_in_rate;
mod discount_margin;
mod bucketed_cs01;

pub use all_in_rate::AllInRateCalculator;
pub use cs01::Cs01Calculator;
pub use discount_margin::DiscountMarginCalculator;
pub use dv01::Dv01Calculator;
pub use ytm::YtmCalculator;
pub use ytw::YtwCalculator;
pub use ytc::YtcCalculator;
pub use ytn::{Yt2yCalculator, Yt3yCalculator, Yt4yCalculator};
pub use bucketed_cs01::BucketedCs01Calculator;

use crate::metrics::MetricRegistry;

/// Register all Term Loan metrics with the registry.
pub fn register_term_loan_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
    instrument: "TermLoan",
        metrics: [
            (Dv01, Dv01Calculator),
            (Cs01, Cs01Calculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::TermLoan,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::TermLoan,
            >::default()),
            // Bucketed CS01 via discount curve bumps
            (BucketedCs01, BucketedCs01Calculator),
            (Ytw, YtwCalculator),
        ]
    }

    // Loan-specific metrics
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::custom("all_in_rate"),
        Arc::new(AllInRateCalculator),
        &["TermLoan"],
    );
    registry.register_metric(
        MetricId::Ytm,
        Arc::new(YtmCalculator),
        &["TermLoan"],
    );
    registry.register_metric(
        MetricId::DiscountMargin,
        Arc::new(DiscountMarginCalculator),
        &["TermLoan"],
    );

    // Yield to first call (custom id: ytc)
    registry.register_metric(
        MetricId::custom("ytc"),
        Arc::new(YtcCalculator),
        &["TermLoan"],
    );

    // Yields to fixed horizons
    registry.register_metric(
        MetricId::custom("yt2y"),
        Arc::new(Yt2yCalculator),
        &["TermLoan"],
    );
    registry.register_metric(
        MetricId::custom("yt3y"),
        Arc::new(Yt3yCalculator),
        &["TermLoan"],
    );
    registry.register_metric(
        MetricId::custom("yt4y"),
        Arc::new(Yt4yCalculator),
        &["TermLoan"],
    );
}


