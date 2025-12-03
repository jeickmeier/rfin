//! Multi-period P&L attribution for financial instruments.
//!
//! This module provides comprehensive P&L attribution capabilities to decompose
//! daily MTM changes into constituent factors: carry, curve shifts, credit spreads,
//! FX, volatility, model parameters, and market scalars.
//!
//! # Overview
//!
//! P&L attribution answers the question: "Why did my position's value change from
//! T₀ to T₁?" by isolating the impact of each market factor and model parameter.
//!
//! # Methodologies
//!
//! Three attribution methodologies are supported:
//!
//! - **Parallel Attribution**: Independent factor isolation. Each factor is analyzed
//!   in isolation by restoring T₀ values while keeping all other factors at T₁.
//!   Residual captures cross-effects and non-linearities.
//!
//! - **Waterfall Attribution**: Sequential factor application. Factors are applied
//!   one-by-one in a specified order, with each factor's P&L computed after applying
//!   all previous factors. Guarantees sum = total P&L (minimal residual by construction).
//!
//! - **Metrics-Based Attribution**: Linear approximation using existing metrics
//!   (Theta, DV01, CS01, etc.). Fast but less accurate for large market moves.
//!
//! # Attribution Factors
//!
//! The following factors are supported:
//!
//! - **Carry**: Time decay (theta) and accruals
//! - **RatesCurves**: Discount and forward curve shifts (IR risk)
//! - **CreditCurves**: Hazard curve shifts (credit spread risk)
//! - **InflationCurves**: Inflation curve shifts
//! - **Correlations**: Base correlation curve changes (structured credit)
//! - **Fx**: FX rate changes
//! - **Volatility**: Implied volatility changes
//! - **ModelParameters**: Model-specific parameters (prepayment, default, recovery, conversion)
//! - **MarketScalars**: Dividends, equity/commodity prices, inflation indices
//!
//! # Examples
//!
//! ## Basic Parallel Attribution
//!
//! ```rust,ignore
//! use finstack_valuations::attribution::attribute_pnl_parallel;
//!
//! let attribution = attribute_pnl_parallel(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     as_of_t0,
//!     as_of_t1,
//!     &config,
//! )?;
//!
//! println!("Total P&L: {}", attribution.total_pnl);
//! println!("Carry: {}", attribution.carry);
//! println!("Rates: {}", attribution.rates_curves_pnl);
//! println!("Residual: {} ({:.2}%)",
//!     attribution.residual,
//!     attribution.meta.residual_pct
//! );
//! ```
//!
//! ## Waterfall Attribution
//!
//! ```rust,ignore
//! use finstack_valuations::attribution::{
//!     attribute_pnl_waterfall, AttributionFactor
//! };
//!
//! let factor_order = vec![
//!     AttributionFactor::Carry,
//!     AttributionFactor::RatesCurves,
//!     AttributionFactor::CreditCurves,
//!     AttributionFactor::Fx,
//! ];
//!
//! let attribution = attribute_pnl_waterfall(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     as_of_t0,
//!     as_of_t1,
//!     &config,
//!     factor_order,
//! )?;
//!
//! // Residual should be minimal (< 0.01%)
//! assert!(attribution.residual_within_tolerance(0.01, 1.0));
//! ```
//!
//! ## Metrics-Based Attribution
//!
//! ```rust,ignore
//! use finstack_valuations::attribution::attribute_pnl_metrics_based;
//!
//! // Requires pre-computed valuations with metrics
//! let val_t0 = instrument.price_with_metrics(&market_t0, as_of_t0, &metrics)?;
//! let val_t1 = instrument.price_with_metrics(&market_t1, as_of_t1, &metrics)?;
//!
//! let attribution = attribute_pnl_metrics_based(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     &val_t0,
//!     &val_t1,
//!     as_of_t0,
//!     as_of_t1,
//! )?;
//! ```
//!
//! # Per-Tenor Curve Attribution
//!
//! ```rust,ignore
//! if let Some(rates_detail) = &attribution.rates_detail {
//!     for ((curve_id, tenor), pnl) in &rates_detail.by_tenor {
//!         println!("{} {}: {}", curve_id, tenor, pnl);
//!     }
//! }
//! ```

pub mod dataframe;
pub mod factors;
pub mod helpers;
pub mod metrics_based;
pub mod model_params;
pub mod parallel;
pub mod spec;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod types;
pub mod waterfall;

// Re-export core types
pub use types::{
    AttributionFactor, AttributionMeta, AttributionMethod, CorrelationsAttribution,
    CreditCurvesAttribution, FxAttribution, InflationCurvesAttribution, ModelParamsAttribution,
    PnlAttribution, RatesCurvesAttribution, ScalarsAttribution, VolAttribution,
};

// Re-export attribution functions
pub use metrics_based::attribute_pnl_metrics_based;
pub use model_params::{
    extract_model_params, measure_conversion_shift, measure_default_shift,
    measure_prepayment_shift, measure_recovery_shift, with_model_params, ModelParamsSnapshot,
};
pub use parallel::attribute_pnl_parallel;
pub use spec::{
    default_attribution_metrics, AttributionConfig, AttributionEnvelope, AttributionResult,
    AttributionResultEnvelope, AttributionSpec, ATTRIBUTION_SCHEMA_V1,
};
pub use waterfall::{attribute_pnl_waterfall, default_waterfall_order};
