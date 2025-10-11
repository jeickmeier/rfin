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
pub mod ytm;
pub mod ytw;
pub mod z_spread;

pub use accrued::AccruedInterestCalculator;
pub use asw::{
    AssetSwapMarketCalculator, AssetSwapMarketFwdCalculator, AssetSwapParCalculator,
    AssetSwapParFwdCalculator,
};
pub use convexity::ConvexityCalculator;
pub use cs01::Cs01Calculator;
pub use dm::DiscountMarginCalculator;
pub use duration_macaulay::MacaulayDurationCalculator;
pub use duration_modified::ModifiedDurationCalculator;
pub use i_spread::ISpreadCalculator;
pub use oas::OasCalculator;
pub use prices::{CleanPriceCalculator, DirtyPriceCalculator};
pub use ytm::YtmCalculator;
pub use ytw::YtwCalculator;
pub use z_spread::ZSpreadCalculator;

/// Registers all bond metrics to a registry.
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Bond",
        metrics: [
            (Accrued, AccruedInterestCalculator),
            (DirtyPrice, DirtyPriceCalculator),
            (CleanPrice, CleanPriceCalculator),
            (Ytm, YtmCalculator),
            (DurationMac, MacaulayDurationCalculator),
            (DurationMod, ModifiedDurationCalculator),
            (Convexity, ConvexityCalculator),
            (Ytw, YtwCalculator),
            (Oas, OasCalculator),
            (ZSpread, ZSpreadCalculator),
            (ISpread, ISpreadCalculator),
            (DiscountMargin, DiscountMarginCalculator),
            (ASWPar, AssetSwapParCalculator),
            (ASWMarket, AssetSwapMarketCalculator),
            (ASWParFwd, AssetSwapParFwdCalculator),
            (ASWMarketFwd, AssetSwapMarketFwdCalculator),
            (Cs01, Cs01Calculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01::<
                crate::instruments::Bond,
            >::default()),
        ]
    };
}
