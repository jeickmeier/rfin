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

use crate::calibration::hull_white::HullWhiteParams;
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cap_floor::types::{CapFloor, RateOptionType};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::DayCountContext;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
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

        // Get discount curve
        let disc = market
            .get_discount(cap_floor.discount_curve_id.as_str())
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
            model_config.tree_volatility.unwrap_or(self.hw_params.sigma),
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let tree_steps = model_config.tree_steps.unwrap_or_else(|| {
            (periods.len() * DEFAULT_STEPS_PER_PERIOD).clamp(MIN_TREE_STEPS, MAX_TREE_STEPS)
        });

        // Calibrate a single HW tree over the full maturity horizon
        let config = HullWhiteTreeConfig::new(hw_params.kappa, hw_params.sigma, tree_steps);
        let tree = HullWhiteTree::calibrate(config, disc.as_ref(), maturity_time).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Price each caplet/floorlet using the tree
        let mut total_pv = 0.0;

        for period in &periods {
            let t_start = cap_floor
                .day_count
                .year_fraction(as_of, period.accrual_start, ctx)
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?;
            let t_end = cap_floor
                .day_count
                .year_fraction(as_of, period.accrual_end, ctx)
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?;

            // Skip expired caplets
            if t_start <= 0.0 {
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

            // Price this caplet using the tree.
            //
            // A caplet paying N * tau * max(L - K, 0) at T_end where
            // L = (1/P(T_start,T_end) - 1) / tau is equivalent to
            // (1 + K*tau) * put on ZCB P(T_start, T_end) with strike X = 1/(1+K*tau).
            //
            // Similarly a floorlet is equivalent to (1+K*tau) * call on ZCB.
            //
            // We evaluate this via backward induction on the tree: the payoff
            // is evaluated at the fixing step (T_start) and then discounted
            // back to today.
            let caplet_pv = self.price_caplet(
                &tree,
                disc.as_ref(),
                t_start,
                t_end,
                tau,
                strike,
                notional,
                is_cap,
            );

            total_pv += caplet_pv;
        }

        Ok(ValuationResult::stamped(
            cap_floor.id.as_str(),
            as_of,
            Money::new(total_pv, cap_floor.notional.currency()),
        ))
    }

    /// Price a single caplet/floorlet using the HW tree.
    ///
    /// The caplet fixes at `t_start` and pays at `t_end`. The payoff at
    /// `t_end` is `N * tau * max(L - K, 0)` for a caplet, where
    /// `L = (1/P(t_start, t_end) - 1) / tau`.
    ///
    /// Equivalently, the caplet PV at `t_start` is:
    /// `N * max(1 - (1 + K*tau) * P(t_start, t_end), 0)` for a caplet
    /// `N * max((1 + K*tau) * P(t_start, t_end) - 1, 0)` for a floorlet
    ///
    /// We evaluate this at every node at the fixing step and then use
    /// backward induction to discount to today.
    #[allow(clippy::too_many_arguments)]
    fn price_caplet(
        &self,
        tree: &HullWhiteTree,
        disc: &dyn Discounting,
        t_start: f64,
        t_end: f64,
        tau: f64,
        strike: f64,
        notional: f64,
        is_cap: bool,
    ) -> f64 {
        let fixing_step = tree.time_to_step(t_start);
        let n = tree.num_steps();

        // Strike price for the ZCB option
        let zcb_strike = 1.0 / (1.0 + strike * tau);

        // Terminal values: zero
        let terminal: Vec<f64> = vec![0.0; tree.num_nodes(n)];

        // Backward induction with caplet payoff at fixing step
        tree.backward_induction(&terminal, |step, node_idx, continuation| {
            if step == fixing_step {
                // Compute ZCB price P(t_start, t_end) at this node
                let zcb = tree.bond_price(step, node_idx, t_end, disc);

                // Caplet: option to receive max(L - K, 0) * tau * N at T_end
                // At T_start, PV of caplet = N * max(1 - (1+K*tau)*P, 0) for cap
                //                          = N * max((1+K*tau)*P - 1, 0) for floor
                let payoff = if is_cap {
                    notional * (1.0 - zcb / zcb_strike).max(0.0)
                } else {
                    notional * (zcb / zcb_strike - 1.0).max(0.0)
                };

                continuation.max(payoff)
            } else {
                continuation
            }
        })
    }
}
