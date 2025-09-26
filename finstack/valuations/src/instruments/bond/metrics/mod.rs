//! Bond-specific metric calculators split into per-metric modules.

pub mod accrued;
pub mod asw;
pub mod convexity;
pub mod cs01;
pub mod dm;
pub mod duration_macaulay;
pub mod duration_modified;
pub mod i_spread;
pub mod oas;
pub mod prices;
pub mod risk_bucketed_dv01;
pub mod ytm;
pub mod ytw;
pub mod z_spread;

pub use accrued::AccruedInterestCalculator;
pub use asw::{AssetSwapMarketCalculator, AssetSwapParCalculator};
pub use convexity::ConvexityCalculator;
pub use cs01::Cs01Calculator;
pub use dm::DiscountMarginCalculator;
pub use duration_macaulay::MacaulayDurationCalculator;
pub use duration_modified::ModifiedDurationCalculator;
pub use i_spread::ISpreadCalculator;
pub use oas::OasCalculator;
pub use prices::{CleanPriceCalculator, DirtyPriceCalculator};
pub use risk_bucketed_dv01::BucketedDv01Calculator;
pub use ytm::YtmCalculator;
pub use ytw::YtwCalculator;
pub use z_spread::ZSpreadCalculator;

/// Registers all bond metrics to a registry.
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::Accrued,
            Arc::new(AccruedInterestCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::DirtyPrice,
            Arc::new(DirtyPriceCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::CleanPrice,
            Arc::new(CleanPriceCalculator),
            &["Bond"],
        )
        .register_metric(MetricId::Ytm, Arc::new(YtmCalculator), &["Bond"])
        .register_metric(
            MetricId::DurationMac,
            Arc::new(MacaulayDurationCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::DurationMod,
            Arc::new(ModifiedDurationCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::Convexity,
            Arc::new(ConvexityCalculator),
            &["Bond"],
        )
        .register_metric(MetricId::Ytw, Arc::new(YtwCalculator), &["Bond"])
        .register_metric(MetricId::Oas, Arc::new(OasCalculator), &["Bond"])
        .register_metric(MetricId::ZSpread, Arc::new(ZSpreadCalculator), &["Bond"])
        .register_metric(MetricId::ISpread, Arc::new(ISpreadCalculator), &["Bond"])
        .register_metric(
            MetricId::DiscountMargin,
            Arc::new(DiscountMarginCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::ASWPar,
            Arc::new(AssetSwapParCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::ASWMarket,
            Arc::new(AssetSwapMarketCalculator),
            &["Bond"],
        )
        .register_metric(MetricId::Cs01, Arc::new(Cs01Calculator), &["Bond"])
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(BucketedDv01Calculator),
            &["Bond"],
        );
}
