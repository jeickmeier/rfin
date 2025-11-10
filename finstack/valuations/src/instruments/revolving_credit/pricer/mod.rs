//! Pricing engines for revolving credit facilities.
//!
//! This module provides multiple pricing approaches:
//!
//! - [`RevolvingCreditPricer`]: Unified pricer that automatically selects method
//! - [`deterministic`]: Deterministic discounting for fixed utilization schedules
//! - [`stochastic`]: Monte Carlo simulation for stochastic utilization processes
//!
//! # Usage
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::revolving_credit::pricer;
//!
//! // Unified pricing (recommended)
//! let pv = pricer::RevolvingCreditPricer::price(&facility, &market, as_of)?;
//!
//! // Direct access to specific engines
//! let pv_det = pricer::deterministic::RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, as_of)?;
//! let result_mc = pricer::stochastic::RevolvingCreditMcPricer::price_stochastic(&facility, &market, as_of, None)?;
//! let pv_mc = result_mc.estimate.mean;
//! ```

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::types::RevolvingCredit;

pub mod deterministic;
pub mod stochastic;

/// Unified pricer that automatically selects the appropriate pricing method.
///
/// This pricer inspects the facility specification and routes to either
/// deterministic or Monte Carlo pricing as appropriate. It provides a
/// single entry point for all revolving credit pricing needs.
#[derive(Default)]
pub struct RevolvingCreditPricer;

impl RevolvingCreditPricer {
    /// Create a new unified revolving credit pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using the appropriate method.
    ///
    /// This method automatically determines whether to use deterministic or
    /// stochastic pricing based on the facility's draw/repay specification:
    ///
    /// - `DrawRepaySpec::Deterministic`: Uses deterministic discounting
    /// - `DrawRepaySpec::Stochastic`: Uses Monte Carlo simulation
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility to price
    /// * `market` - Market data context containing required curves
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value of the facility as seen by the lender.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required market data is missing
    /// - Facility specification is invalid
    /// - Pricing method fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Deterministic facility
    /// let det_facility = RevolvingCredit::builder()
    ///     .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
    ///     .build()?;
    ///
    /// // Stochastic facility
    /// let stoch_facility = RevolvingCredit::builder()
    ///     .draw_repay_spec(DrawRepaySpec::Stochastic(stoch_spec))
    ///     .build()?;
    ///
    /// // Price both with same interface
    /// let pv_det = RevolvingCreditPricer::price(&det_facility, &market, as_of)?;
    /// let pv_stoch = RevolvingCreditPricer::price(&stoch_facility, &market, as_of)?;
    /// ```
    pub fn price(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        match &facility.draw_repay_spec {
            super::types::DrawRepaySpec::Deterministic(_) => {
                deterministic::RevolvingCreditDiscountingPricer::price_deterministic(
                    facility, market, as_of,
                )
            }
            super::types::DrawRepaySpec::Stochastic(_) => {
                #[cfg(feature = "mc")]
                {
                    let result = stochastic::RevolvingCreditMcPricer::price_stochastic(facility, market, as_of, None)?;
                    Ok(result.estimate.mean)
                }
                #[cfg(not(feature = "mc"))]
                {
                    Err(finstack_core::error::InputError::Invalid.into())
                }
            }
        }
    }
}

impl Pricer for RevolvingCreditPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Extract valuation date from discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let as_of = disc.base_date();

        // Price using unified interface
        let pv = Self::price(facility, market, as_of)?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use time::Month;

    use super::super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};

    #[test]
    fn test_unified_pricer_key() {
        let pricer = RevolvingCreditPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
        );
    }

    #[test]
    fn test_unified_pricer_deterministic() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-UNIFIED-DET".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();

        let market = MarketContext::new().insert_discount(disc_curve);

        // Test unified pricing
        let pv_unified = RevolvingCreditPricer::price(&facility, &market, start).unwrap();

        // Test direct deterministic pricing for comparison
        let pv_direct = deterministic::RevolvingCreditDiscountingPricer::price_deterministic(
            &facility, &market, start,
        )
        .unwrap();

        assert_eq!(pv_unified.amount(), pv_direct.amount());
        assert!(pv_unified.currency() == Currency::USD);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_unified_pricer_stochastic() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-UNIFIED-STOCH".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 1.0,
                        volatility: 0.1,
                    },
                    num_paths: 100,
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();

        let market = MarketContext::new().insert_discount(disc_curve);

        // Test unified pricing
        let pv_unified = RevolvingCreditPricer::price(&facility, &market, start).unwrap();

        // Test direct stochastic pricing for comparison
        let result_direct = stochastic::RevolvingCreditMcPricer::price_stochastic(
            &facility, &market, start, None,
        )
        .unwrap();
        let pv_direct = result_direct.estimate.mean;

        assert_eq!(pv_unified.amount(), pv_direct.amount());
        assert!(pv_unified.currency() == Currency::USD);
    }
}
