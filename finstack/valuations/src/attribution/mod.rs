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
//! ```rust,no_run
//! use finstack_valuations::attribution::attribute_pnl_parallel;
//! use finstack_valuations::instruments::rates::deposit::Deposit;
//! use finstack_core::config::FinstackConfig;
//! use finstack_core::currency::Currency;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let as_of_t0 = date!(2025-01-15);
//! let as_of_t1 = date!(2025-01-16);
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! let config = FinstackConfig::default();
//!
//! let instrument = Arc::new(
//!     Deposit::builder()
//!         .id("DEP-1D".into())
//!         .notional(Money::new(1_000_000.0, Currency::USD))
//!         .start(as_of_t0)
//!         .end(as_of_t1)
//!         .day_count(finstack_core::dates::DayCount::Act360)
//!         .discount_curve_id("USD-OIS".into())
//!         .build()
//!         .expect("deposit builder should succeed"),
//! ) as Arc<dyn finstack_valuations::instruments::Instrument>;
//!
//! let attribution = attribute_pnl_parallel(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     as_of_t0,
//!     as_of_t1,
//!     &config,
//!     None,
//! )?;
//!
//! println!("Total P&L: {}", attribution.total_pnl);
//! println!("Carry: {}", attribution.carry);
//! println!("Rates: {}", attribution.rates_curves_pnl);
//! println!("Residual: {} ({:.2}%)",
//!     attribution.residual,
//!     attribution.meta.residual_pct
//! );
//! # Ok(())
//! # }
//! ```
//!
//! ## Waterfall Attribution
//!
//! ```rust,no_run
//! use finstack_valuations::attribution::{
//!     attribute_pnl_waterfall, AttributionFactor
//! };
//! use finstack_valuations::instruments::rates::deposit::Deposit;
//! use finstack_core::config::FinstackConfig;
//! use finstack_core::currency::Currency;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let factor_order = vec![
//!     AttributionFactor::Carry,
//!     AttributionFactor::RatesCurves,
//!     AttributionFactor::CreditCurves,
//!     AttributionFactor::Fx,
//! ];
//!
//! let as_of_t0 = date!(2025-01-15);
//! let as_of_t1 = date!(2025-01-16);
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! let config = FinstackConfig::default();
//! let instrument = Arc::new(
//!     Deposit::builder()
//!         .id("DEP-1D".into())
//!         .notional(Money::new(1_000_000.0, Currency::USD))
//!         .start(as_of_t0)
//!         .end(as_of_t1)
//!         .day_count(finstack_core::dates::DayCount::Act360)
//!         .discount_curve_id("USD-OIS".into())
//!         .build()
//!         .expect("deposit builder should succeed"),
//! ) as Arc<dyn finstack_valuations::instruments::Instrument>;
//!
//! let attribution = attribute_pnl_waterfall(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     as_of_t0,
//!     as_of_t1,
//!     &config,
//!     factor_order,
//!     false, // strict validation
//!     None,
//! )?;
//!
//! // Residual should be minimal (< 0.01%)
//! assert!(attribution.residual_within_tolerance(0.01, 1.0));
//! # Ok(())
//! # }
//! ```
//!
//! ## Metrics-Based Attribution
//!
//! ```rust,no_run
//! use finstack_valuations::attribution::attribute_pnl_metrics_based;
//! use finstack_valuations::attribution::default_attribution_metrics;
//! use finstack_valuations::instruments::rates::deposit::Deposit;
//! use finstack_core::currency::Currency;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! // Requires pre-computed valuations with metrics
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let as_of_t0 = date!(2025-01-15);
//! let as_of_t1 = date!(2025-01-16);
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! let metrics = default_attribution_metrics();
//!
//! let instrument = Arc::new(
//!     Deposit::builder()
//!         .id("DEP-1D".into())
//!         .notional(Money::new(1_000_000.0, Currency::USD))
//!         .start(as_of_t0)
//!         .end(as_of_t1)
//!         .day_count(finstack_core::dates::DayCount::Act360)
//!         .discount_curve_id("USD-OIS".into())
//!         .build()
//!         .expect("deposit builder should succeed"),
//! ) as Arc<dyn finstack_valuations::instruments::Instrument>;
//!
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
//! # let _ = attribution;
//! # Ok(())
//! # }
//! ```
//!
//! # Per-Tenor Curve Attribution
//!
//! ```rust,no_run
//! use finstack_valuations::attribution::attribute_pnl_parallel;
//! use finstack_valuations::instruments::rates::deposit::Deposit;
//! use finstack_core::config::FinstackConfig;
//! use finstack_core::currency::Currency;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let as_of_t0 = date!(2025-01-15);
//! let as_of_t1 = date!(2025-01-16);
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! let config = FinstackConfig::default();
//!
//! let instrument = Arc::new(
//!     Deposit::builder()
//!         .id("DEP-1D".into())
//!         .notional(Money::new(1_000_000.0, Currency::USD))
//!         .start(as_of_t0)
//!         .end(as_of_t1)
//!         .day_count(finstack_core::dates::DayCount::Act360)
//!         .discount_curve_id("USD-OIS".into())
//!         .build()
//!         .expect("deposit builder should succeed"),
//! ) as Arc<dyn finstack_valuations::instruments::Instrument>;
//!
//! let attribution = attribute_pnl_parallel(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     as_of_t0,
//!     as_of_t1,
//!     &config,
//!     None,
//! )?;
//!
//! if let Some(rates_detail) = &attribution.rates_detail {
//!     for ((curve_id, tenor), pnl) in &rates_detail.by_tenor {
//!         println!("{} {}: {}", curve_id, tenor, pnl);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub(crate) mod dataframe;
pub(crate) mod factors;
pub(crate) mod helpers;
pub(crate) mod metrics_based;
pub(crate) mod model_params;
pub(crate) mod parallel;
pub(crate) mod spec;
pub(crate) mod types;
pub(crate) mod waterfall;

// Re-export core types
pub use types::{
    AttributionFactor, AttributionInput, AttributionMeta, AttributionMethod,
    CorrelationsAttribution, CreditCurvesAttribution, FxAttribution, InflationCurvesAttribution,
    JsonEnvelope, ModelParamsAttribution, PnlAttribution, RatesCurvesAttribution,
    ScalarsAttribution, VolAttribution,
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
// Market snapshot helpers - exported for test usage
pub use factors::{
    extract, restore_scalars, CreditCurvesSnapshot, CurveRestoreFlags, InflationCurvesSnapshot,
    MarketExtractable, MarketSnapshot, RatesCurvesSnapshot, ScalarsSnapshot,
};
pub use helpers::{compute_pnl, compute_pnl_with_fx, convert_currency, reprice_instrument};
