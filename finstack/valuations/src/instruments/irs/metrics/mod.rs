//! Interest rate swap metrics and risk calculations.
//!
//! Provides specialized metric calculators for IRS instruments including:
//! - **Par Rate**: The fixed rate that makes the swap's NPV zero
//! - **Annuity**: Sum of discounted accrual factors (fixed leg)
//! - **DV01**: Dollar value of 1bp parallel curve shift
//! - **Bucketed DV01**: Key rate sensitivities at curve vertices
//! - **PV Fixed/Float**: Individual leg present values
//!
//! # Module Organization
//!
//! Each metric is implemented in its own file for clarity:
//! - [`par_rate`]: Par swap rate calculation
//! - [`annuity`]: Fixed leg annuity
//! - [`pv_fixed`]: Fixed leg PV
//! - [`pv_float`]: Floating leg PV
//!
//! DV01 metrics use the unified calculator framework from `crate::metrics`.
//!
//! # Examples
//!
//! ```ignore
//! use finstack_valuations::instruments::irs::InterestRateSwap;
//! use finstack_valuations::instruments::common::traits::Instrument;
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//! # use time::Month;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let irs = InterestRateSwap::example()?;
//! // Build market context with required curves
//! let mut context = MarketContext::new();
//! // ... add USD-OIS and USD-SOFR-3M curves ...
//!
//! let as_of = Date::from_calendar_date(2024, Month::January, 1)
//!     .map_err(|e| finstack_core::error::Error::Validation(format!("{}", e)))?;
//!
//! // Compute par rate and DV01
//! let metrics = vec![MetricId::ParRate, MetricId::Dv01];
//! let result = irs.price_with_metrics(&context, as_of, &metrics)?;
//!
//! let par_rate = result.measures.get(&MetricId::ParRate);
//! let dv01 = result.measures.get(&MetricId::Dv01);
//! # Ok(())
//! # }
//! ```

pub mod annuity;
pub mod par_rate;
pub mod pv_fixed;
pub mod pv_float;
// risk_bucketed_dv01, dv01, and theta now using generic implementations

/// Registers all IRS metrics into a provided registry.
pub fn register_irs_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateSwap",
        metrics: [
            (Annuity, annuity::AnnuityCalculator),
            (ParRate, par_rate::ParRateCalculator),

            // Theta is now registered universally in metrics::standard_registry()

            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InterestRateSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),

            // Pv01 is an alias for standard DV01 for IRS
            (Pv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InterestRateSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),

            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InterestRateSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),

            (PvFixed, pv_fixed::FixedLegPvCalculator),
            (PvFloat, pv_float::FloatLegPvCalculator),
        ]
    }
}
