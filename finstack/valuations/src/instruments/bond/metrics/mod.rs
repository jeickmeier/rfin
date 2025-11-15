//! Bond-specific metric calculators split into per-metric modules.

/// Accrued interest calculator
pub mod accrued;
/// Asset swap spread calculators (par, market, forward)
pub mod asw;
/// Convexity calculator
pub mod convexity;
/// Credit spread DV01 (CS01) calculator
pub mod cs01;
/// Discount margin calculator
pub mod dm;
/// Macaulay duration calculator
pub mod duration_macaulay;
/// Modified duration calculator
pub mod duration_modified;
/// I-spread (interpolated spread) calculator
pub mod i_spread;
/// Option-adjusted spread (OAS) calculator
pub mod oas;
/// Price calculators (clean and dirty)
pub mod prices;
/// Yield-to-maturity (YTM) calculator
pub mod ytm;
/// Yield-to-worst (YTW) calculator
pub mod ytw;
/// Z-spread (zero-volatility spread) calculator
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
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Pv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())), // Alias for DV01 (credit convention)
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    };
}
