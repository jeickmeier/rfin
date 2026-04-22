//! Hull-White 1-factor tree pricer for European swaptions.
//!
//! Prices a European swaption by building a calibrated Hull-White trinomial
//! tree and performing backward induction with a single exercise date at
//! expiry. This is the short-rate analogue of the Black-76 pricer and is
//! particularly useful when consistent pricing with Bermudan swaptions
//! (which also use the HW tree) is required.
//!
//! # Algorithm
//!
//! 1. Calibrate a Hull-White tree to the discount curve over the swap
//!    maturity horizon.
//! 2. At the terminal step, compute continuation values of zero.
//! 3. During backward induction, at the tree step corresponding to the
//!    swaption expiry, compute the exercise value:
//!    - Payer: max(0, (S - K) * A * N)
//!    - Receiver: max(0, (K - S) * A * N)
//!    where S is the forward swap rate, A the annuity, N the notional.
//! 4. The root node value is the present value.
//!
//! # References
//!
//! - Hull, J. & White, A. (1994). "Numerical Procedures for Implementing
//!   Term Structure Models I: Single-Factor Models", *Journal of Derivatives*.
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models - Theory and
//!   Practice*, Chapter 4.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::swaption::types::Swaption;
use crate::instruments::rates::swaption::HullWhiteParams;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, DayCountContext, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Number of tree steps used for the HW tree pricing.
const DEFAULT_TREE_STEPS: usize = 100;

/// Hull-White 1-factor tree pricer for European swaptions.
///
/// Prices European swaptions via backward induction on a calibrated
/// Hull-White trinomial tree. The tree is calibrated to the initial
/// discount curve and exercise is evaluated at the single expiry date.
pub(crate) struct SwaptionHullWhitePricer {
    hw_params: HullWhiteParams,
    tree_steps: usize,
}

impl Default for SwaptionHullWhitePricer {
    fn default() -> Self {
        Self {
            hw_params: HullWhiteParams::default(),
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }
}

impl Pricer for SwaptionHullWhitePricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, ModelKey::HullWhite1F)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let swaption = instrument
            .as_any()
            .downcast_ref::<Swaption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::Swaption, instrument.key())
            })?;

        self.price_internal(swaption, market, as_of)
    }
}

impl SwaptionHullWhitePricer {
    /// Core pricing routine.
    fn price_internal(
        &self,
        swaption: &Swaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Single-curve requirement (same as Bermudan pricer)
        if swaption.forward_curve_id != swaption.discount_curve_id {
            return Err(PricingError::model_failure_with_context(
                "Hull-White tree pricing is currently single-curve only. \
                 Set forward_curve_id equal to discount_curve_id or use a multi-curve-capable engine."
                    .to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Get discount curve
        let disc = market
            .get_discount(swaption.discount_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Time to expiry
        let time_to_expiry =
            year_fraction(swaption.day_count, as_of, swaption.expiry).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if time_to_expiry <= 0.0 {
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Time horizon is swap end (need the tree to cover the full swap)
        let ctx = DayCountContext::default();
        let swap_end_time = swaption
            .day_count
            .year_fraction(as_of, swaption.swap_end, ctx)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if swap_end_time <= 0.0 {
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Build and calibrate HW tree
        let config =
            HullWhiteTreeConfig::new(self.hw_params.kappa, self.hw_params.sigma, self.tree_steps);
        let tree = HullWhiteTree::calibrate(config, disc.as_ref(), swap_end_time).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Build swap schedule for the underlying
        let calendar_id = swaption
            .calendar_id
            .as_deref()
            .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID);

        let sched = crate::cashflow::builder::build_dates(
            swaption.swap_start,
            swaption.swap_end,
            swaption.fixed_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing,
            false,
            0,
            calendar_id,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let dates = &sched.dates;
        if dates.len() < 2 {
            return Err(PricingError::model_failure_with_context(
                "Swap schedule has fewer than 2 dates".to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Compute payment times and accrual fractions
        let mut payment_times = Vec::with_capacity(dates.len() - 1);
        let mut accrual_fractions = Vec::with_capacity(dates.len() - 1);
        let mut prev = dates[0];
        for &d in dates.iter().skip(1) {
            let t = swaption
                .day_count
                .year_fraction(as_of, d, ctx)
                .map_err(|e| {
                    PricingError::model_failure_with_context(
                        e.to_string(),
                        PricingErrorContext::default(),
                    )
                })?;
            let accrual = year_fraction(swaption.day_count, prev, d).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
            payment_times.push(t);
            accrual_fractions.push(accrual);
            prev = d;
        }

        let swap_start_time = swaption
            .day_count
            .year_fraction(as_of, swaption.swap_start, ctx)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        let strike = swaption.strike_f64().map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let notional = swaption.notional.amount();

        // Map expiry to tree step
        let exercise_step = tree.time_to_step(time_to_expiry);

        let n = tree.num_steps();

        // Terminal values: zero (no exercise at terminal step for European)
        let terminal: Vec<f64> = vec![0.0; tree.num_nodes(n)];

        // Backward induction with exercise at expiry step only
        let pv = tree.backward_induction(&terminal, |step, node_idx, continuation| {
            if step == exercise_step {
                let t = tree.time_at_step(step);

                // Find remaining payments after this time
                let start_idx = payment_times.partition_point(|&pt| pt <= t);
                if start_idx >= payment_times.len() {
                    return continuation;
                }

                let remaining_payment_times = &payment_times[start_idx..];
                let remaining_accruals = &accrual_fractions[start_idx..];

                let swap_start = swap_start_time.max(t);
                let swap_rate = tree.forward_swap_rate(
                    step,
                    node_idx,
                    swap_start,
                    swap_end_time,
                    remaining_payment_times,
                    remaining_accruals,
                    disc.as_ref(),
                );

                let annuity = tree.annuity(
                    step,
                    node_idx,
                    remaining_payment_times,
                    remaining_accruals,
                    disc.as_ref(),
                );

                let intrinsic = match swaption.option_type {
                    OptionType::Call => (swap_rate - strike).max(0.0),
                    OptionType::Put => (strike - swap_rate).max(0.0),
                };

                let exercise_value = intrinsic * annuity * notional;

                // European: take max of continuation and exercise at the single
                // exercise date (for a well-calibrated tree these should be close,
                // but max handles numerical edge cases).
                continuation.max(exercise_value)
            } else {
                continuation
            }
        });

        Ok(ValuationResult::stamped(
            swaption.id.as_str(),
            as_of,
            Money::new(pv, swaption.notional.currency()),
        ))
    }
}
