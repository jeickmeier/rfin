//! Bond-specific metric calculators split into per-metric modules.

/// Accrued interest calculator
pub mod accrued;
/// Convexity calculator
pub mod convexity;
/// Macaulay duration calculator
pub mod duration_macaulay;
/// Modified duration calculator
pub mod duration_modified;
/// Price, yield, and spread metrics
pub mod price_yield_spread;

pub use accrued::AccruedInterestCalculator;
pub use convexity::ConvexityCalculator;
pub use duration_macaulay::MacaulayDurationCalculator;
pub use duration_modified::ModifiedDurationCalculator;
pub use price_yield_spread::{
    AssetSwapMarketCalculator, AssetSwapMarketFwdCalculator, AssetSwapParCalculator,
    AssetSwapParFwdCalculator, CleanPriceCalculator, DirtyPriceCalculator,
    DiscountMarginCalculator, ISpreadCalculator, OasCalculator, YtmCalculator, YtwCalculator,
    ZSpreadCalculator,
};

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
            (ZSpread, ZSpreadCalculator::default()),
            (ISpread, ISpreadCalculator::default()),
            (DiscountMargin, DiscountMarginCalculator::default()),
            (ASWPar, AssetSwapParCalculator::default()),
            (ASWMarket, AssetSwapMarketCalculator::default()),
            (ASWParFwd, AssetSwapParFwdCalculator),
            (ASWMarketFwd, AssetSwapMarketFwdCalculator),
            (Cs01, crate::metrics::GenericParallelCs01::<
                crate::instruments::Bond,
            >::default()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    };
}
