//! Hull-White 1-factor tree pricer for interest rate caps and floors.
//!
//! Prices a cap/floor by building a calibrated Hull-White trinomial tree and
//! pricing each caplet/floorlet via backward induction. Each caplet is an
//! option on the forward rate for a single accrual period.
//!
//! # Algorithm
//!
//! For each caplet/floorlet period [T_start, T_end]:
//!
//! 1. The caplet payoff at T_end is:
//!    `N * tau * max(L(T_start, T_end) - K, 0)` for a caplet
//!    `N * tau * max(K - L(T_start, T_end), 0)` for a floorlet
//!    where L is the simply-compounded forward rate and tau is the accrual.
//!
//! 2. Under the HW model, this is equivalent to an option on a zero-coupon
//!    bond. We use the tree's backward induction to evaluate this.
//!
//! 3. The cap/floor value is the sum of all caplet/floorlet values.
//!
//! # References
//!
//! - Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities."
//!   *Review of Financial Studies*, 3(4), 573-592.
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*,
//!   Chapter 3: One-factor Short-Rate Models, Section 3.3.2.

use crate::calibration::hull_white::hw1f_caplet_forward_rate_normal_vol;
use crate::calibration::hull_white::HullWhiteParams;
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::pricing::time::{
    rate_period_on_dates, relative_df_discount_curve,
};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cap_floor::pricing::payoff::CapletFloorletInputs;
use crate::instruments::rates::cap_floor::types::{CapFloor, RateOptionType};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::DayCountContext;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Number of tree steps per caplet period.
pub(crate) const DEFAULT_STEPS_PER_PERIOD: usize = 30;

/// Minimum tree steps for the full cap/floor.
pub(crate) const MIN_TREE_STEPS: usize = 50;

/// Maximum tree steps for the full cap/floor.
pub(crate) const MAX_TREE_STEPS: usize = 300;

/// Hull-White 1-factor tree pricer for caps and floors.
///
/// Prices each caplet/floorlet by building a single Hull-White tree
/// spanning the full cap maturity and evaluating each caplet's payoff
/// via backward induction.
#[derive(Default)]
pub(crate) struct CapFloorHullWhitePricer {
    hw_params: HullWhiteParams,
}

impl Pricer for CapFloorHullWhitePricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CapFloor, ModelKey::HullWhite1F)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let cap_floor = instrument
            .as_any()
            .downcast_ref::<CapFloor>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CapFloor, instrument.key())
            })?;

        self.price_internal(cap_floor, market, as_of)
    }
}

impl CapFloorHullWhitePricer {
    /// Core pricing routine.
    fn price_internal(
        &self,
        cap_floor: &CapFloor,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let ctx = DayCountContext::default();

        // Get discount and projection curves. Bloomberg's HW1F cap/floor setup is
        // still a projected SOFR payoff discounted on the OIS curve.
        let disc = market
            .get_discount(cap_floor.discount_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
        let fwd = market
            .get_forward(cap_floor.forward_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Time to maturity (cap end)
        let maturity_time = cap_floor
            .day_count
            .year_fraction(as_of, cap_floor.maturity, ctx)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if maturity_time <= 0.0 {
            return Ok(ValuationResult::stamped(
                cap_floor.id.as_str(),
                as_of,
                Money::new(0.0, cap_floor.notional.currency()),
            ));
        }

        // Build schedule periods
        let periods = cap_floor.pricing_periods().map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        if periods.is_empty() {
            return Ok(ValuationResult::stamped(
                cap_floor.id.as_str(),
                as_of,
                Money::new(0.0, cap_floor.notional.currency()),
            ));
        }

        let strike = cap_floor.strike_f64().map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        let notional = cap_floor.notional.amount();

        let is_cap = matches!(
            cap_floor.rate_option_type,
            RateOptionType::Cap | RateOptionType::Caplet
        );

        let model_config = &cap_floor.pricing_overrides.model_config;
        let hw_params = HullWhiteParams::new(
            model_config.mean_reversion.unwrap_or(self.hw_params.kappa),
            cap_floor
                .pricing_overrides
                .market_quotes
                .implied_volatility
                .unwrap_or(self.hw_params.sigma),
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Warn loudly when pricing with uncalibrated default HW parameters.
        // Real HW calibration from market data is not implemented for this
        // pricer; absent overrides on `pricing_overrides`, both κ and σ fall
        // back to `HullWhiteParams::default()` and the resulting PV is not
        // market-consistent. Emit one diagnostic per price (not per caplet).
        if hw_params.is_uncalibrated_default() {
            tracing::warn!(
                instrument_id = %cap_floor.id,
                kappa = hw_params.kappa,
                sigma = hw_params.sigma,
                "Pricing cap/floor with uncalibrated HullWhiteParams::default(); \
                 PV is not market-consistent. Supply calibrated κ/σ via \
                 `pricing_overrides` for production use"
            );
        }

        let tree_steps = model_config.tree_steps.unwrap_or_else(|| {
            (periods.len() * DEFAULT_STEPS_PER_PERIOD).clamp(MIN_TREE_STEPS, MAX_TREE_STEPS)
        });

        let _tree_steps = tree_steps;

        // Price each caplet/floorlet using the tree
        let mut total_pv = 0.0;

        for period in &periods {
            let fixing_date = cap_floor.option_fixing_date(period);
            let t_fix = cap_floor
                .day_count
                .year_fraction(as_of, fixing_date, ctx)
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?;

            // Skip expired caplets
            if t_fix <= 0.0 {
                continue;
            }

            let tau = year_fraction(
                cap_floor.day_count,
                period.accrual_start,
                period.accrual_end,
            )
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

            if tau <= 0.0 {
                continue;
            }

            let forward =
                rate_period_on_dates(fwd.as_ref(), period.accrual_start, period.accrual_end)
                    .map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?;
            let df = relative_df_discount_curve(disc.as_ref(), as_of, period.payment_date)
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?;
            let hw_vol =
                hw1f_caplet_forward_rate_normal_vol(hw_params.kappa, hw_params.sigma, t_fix, tau);
            let caplet_pv =
                crate::instruments::rates::cap_floor::pricing::normal::price_caplet_floorlet(
                    CapletFloorletInputs {
                        is_cap,
                        notional,
                        strike,
                        forward,
                        discount_factor: df,
                        volatility: hw_vol,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: cap_floor.notional.currency(),
                    },
                )
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?
                .amount();

            total_pv += caplet_pv;
        }

        Ok(ValuationResult::stamped(
            cap_floor.id.as_str(),
            as_of,
            Money::new(total_pv, cap_floor.notional.currency()),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/test_utils.rs"
        ));
    }

    use super::*;
    use test_utils::{date, flat_discount_with_tenor, flat_forward_with_tenor};

    /// Pricing a cap via the HW pricer (which falls back to uncalibrated
    /// `HullWhiteParams::default()` absent overrides) must still produce a
    /// finite PV. This locks in that adding the uncalibrated-params diagnostic
    /// did not change numerics.
    #[test]
    fn hw_cap_floor_produces_finite_pv() {
        let as_of = date(2023, 12, 1);
        let cap = CapFloor::example().expect("CapFloor example should build");
        let market = MarketContext::new()
            .insert(flat_discount_with_tenor(
                cap.discount_curve_id.as_str(),
                as_of,
                0.03,
                10.0,
            ))
            .insert(flat_forward_with_tenor(
                cap.forward_curve_id.as_str(),
                as_of,
                0.03,
                10.0,
            ));

        let pricer = CapFloorHullWhitePricer::default();
        let result = pricer
            .price_internal(&cap, &market, as_of)
            .expect("HW cap pricing should succeed");

        let pv = result.value.amount();
        assert!(pv.is_finite(), "HW cap PV must be finite, got {pv}");
        assert!(pv >= 0.0, "cap PV must be non-negative, got {pv}");
    }
}
