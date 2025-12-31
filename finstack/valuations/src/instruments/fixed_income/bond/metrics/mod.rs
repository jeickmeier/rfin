//! Bond-specific metric calculators split into per-metric modules.
//!
//! This module provides metric calculators for bond-specific risk and valuation metrics.
//!
//! # Available Metrics
//!
//! ## Price and Yield Metrics
//! - **Accrued Interest**: Interest accrued since last coupon payment
//! - **Clean Price**: Quoted price excluding accrued interest
//! - **Dirty Price**: Clean price plus accrued interest
//! - **Yield to Maturity (YTM)**: Internal rate of return
//! - **Yield to Worst (YTW)**: Minimum yield across call/put/maturity paths
//!
//! ## Risk Metrics
//! - **Macaulay Duration**: Weighted average time to cashflows
//! - **Modified Duration**: Interest rate sensitivity measure
//! - **Convexity**: Curvature of price/yield relationship
//! - **DV01**: Dollar value of 1bp rate change
//! - **CS01**: Credit spread sensitivity
//!
//! ## Spread Metrics
//! - **Z-Spread**: Zero-volatility spread over discount curve
//! - **OAS**: Option-adjusted spread (for callable/putable bonds)
//! - **I-Spread**: Interpolated spread (YTM - par swap rate)
//! - **Discount Margin**: Spread measure for floating-rate notes
//! - **Asset Swap Spreads**: Par and market asset swap spreads
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_valuations::instruments::fixed_income::bond::metrics::register_bond_metrics;
//! use finstack_valuations::metrics::{MetricRegistry, MetricId};
//!
//! let mut registry = MetricRegistry::new();
//! register_bond_metrics(&mut registry);
//!
//! // Use registry to compute metrics for bonds
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - [`register_bond_metrics`] for registering all bond metrics
//! - [`crate::metrics`] for the metrics framework

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
    AssetSwapMarketCalculator, AssetSwapParCalculator, CleanPriceCalculator, DirtyPriceCalculator,
    DiscountMarginCalculator, EmbeddedOptionValueCalculator, ISpreadCalculator, OasCalculator,
    YtmCalculator, YtwCalculator, ZSpreadCalculator,
};

/// Registers all bond metrics to a registry.
///
/// This function registers all bond-specific metric calculators to the provided
/// metric registry, enabling computation of price, yield, duration, convexity,
/// and spread metrics for bonds.
///
/// # Arguments
///
/// * `registry` - The metric registry to register bond metrics into
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::metrics::register_bond_metrics;
/// use finstack_valuations::metrics::MetricRegistry;
///
/// let mut registry = MetricRegistry::new();
/// register_bond_metrics(&mut registry);
/// ```
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::Bond,
        metrics: [
            (Accrued, AccruedInterestCalculator),
            (DirtyPrice, DirtyPriceCalculator),
            (CleanPrice, CleanPriceCalculator),
            (Ytm, YtmCalculator),
            (Ytw, YtwCalculator),

            (DurationMac, MacaulayDurationCalculator),
            (DurationMod, ModifiedDurationCalculator),
            (Convexity, ConvexityCalculator),

            (Oas, OasCalculator),
            (EmbeddedOptionValue, EmbeddedOptionValueCalculator::default()),
            (ZSpread, ZSpreadCalculator::default()),
            (ISpread, ISpreadCalculator::default()),
            (DiscountMargin, DiscountMarginCalculator::default()),
            (ASWPar, AssetSwapParCalculator::default()),
            (ASWMarket, AssetSwapMarketCalculator::default()),

            // Theta is now registered universally in metrics::standard_registry()

            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Bond,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),

            (Cs01, crate::metrics::GenericParallelCs01::<
                crate::instruments::Bond,
            >::default()),
            (BucketedCs01, crate::metrics::GenericBucketedCs01::<
                crate::instruments::Bond,
            >::default()),


        ]
    };
}
