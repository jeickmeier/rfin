//! Metrics for Term Loan instruments.

mod all_in_rate;
mod bucketed_cs01;
mod cs01;
mod discount_margin;
mod ytc;
mod ytm;
mod ytn;
mod ytw;

pub use all_in_rate::AllInRateCalculator;
pub use bucketed_cs01::BucketedCs01Calculator;
pub use cs01::Cs01Calculator;
pub use discount_margin::DiscountMarginCalculator;
pub use ytc::YtcCalculator;
pub use ytm::YtmCalculator;
pub use ytn::{Yt2yCalculator, Yt3yCalculator, Yt4yCalculator};
pub use ytw::YtwCalculator;

use crate::metrics::MetricRegistry;

/// Register all Term Loan metrics with the registry.
pub fn register_term_loan_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
    instrument: "TermLoan",
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::TermLoan,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Cs01, Cs01Calculator),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::TermLoan,
            >::default()),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::TermLoan,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
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
    registry.register_metric(MetricId::Ytm, Arc::new(YtmCalculator), &["TermLoan"]);
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
