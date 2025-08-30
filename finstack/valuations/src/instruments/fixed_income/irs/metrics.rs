//! Interest rate swap specific metric calculators.
//!
//! Provides metric calculators for interest rate swaps including annuity factors,
//! par rates, DV01, and present values for both fixed and floating legs.
//! These metrics are essential for swap valuation and risk management.

use super::PayReceive;
use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates annuity (sum of discounted year fractions) for IRS.
///
/// Computes the sum of discounted year fractions for the fixed leg,
/// which is used in par rate calculations and other swap metrics.
///
/// See unit tests and `examples/` for usage.
pub struct AnnuityCalculator;

impl MetricCalculator for AnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let base = disc.base_date();

        // Build fixed leg schedule
        let sched = crate::cashflow::builder::build_dates(
            irs.fixed.start,
            irs.fixed.end,
            irs.fixed.freq,
            irs.fixed.stub,
            irs.fixed.bdc,
            irs.fixed.calendar_id,
        );
        let schedule: Vec<Date> = sched.dates;

        if schedule.len() < 2 {
            return Ok(0.0);
        }

        // Compute annuity as sum(yf * df)
        let mut annuity = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, irs.fixed.dc);
            let df = DiscountCurve::df_on(&*disc, base, d, irs.fixed.dc);
            annuity += yf * df;
            prev = d;
        }

        Ok(annuity)
    }
}

/// Calculates par rate for IRS.
///
/// Computes the fixed rate that makes the swap worth zero at inception.
/// Uses the formula: PV(float_leg) / (notional * annuity).
///
/// # Dependencies
/// Requires `Annuity` metric to be computed first.
///
/// See unit tests and `examples/` for usage.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let fwd = context.curves.forecast(irs.float.fwd_id)?;
        let base_d = disc.base_date();

        // Get annuity from computed metrics
        let annuity = context
            .computed
            .get(&MetricId::Annuity)
            .copied()
            .unwrap_or(0.0);
        if annuity == 0.0 {
            return Ok(0.0);
        }

        // Compute PV of float leg
        let fs = crate::cashflow::builder::build_dates(
            irs.float.start,
            irs.float.end,
            irs.float.freq,
            irs.float.stub,
            irs.float.bdc,
            irs.float.calendar_id,
        );
        let float_schedule: Vec<Date> = fs.dates;

        if float_schedule.len() < 2 {
            return Ok(0.0);
        }

        let mut float_pv = 0.0;
        let mut prev = float_schedule[0];
        for &d in &float_schedule[1..] {
            let t1 = DiscountCurve::year_fraction(base_d, prev, irs.float.dc);
            let t2 = DiscountCurve::year_fraction(base_d, d, irs.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, irs.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;
            let df = DiscountCurve::df_on(&*disc, base_d, d, irs.float.dc);
            float_pv += coupon * df;
            prev = d;
        }

        // Par rate = float_pv / (notional * annuity)
        Ok(float_pv / irs.notional.amount() / annuity)
    }
}

/// Calculates DV01 (dollar value of 1 basis point) for IRS.
///
/// Computes the change in present value for a 1 basis point parallel shift
/// in interest rates. The sign depends on whether the swap is pay-fixed
/// (negative DV01) or receive-fixed (positive DV01).
///
/// # Dependencies
/// Requires `Annuity` metric to be computed first.
///
/// See unit tests and `examples/` for usage.
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        // Get annuity from computed metrics
        let annuity = context
            .computed
            .get(&MetricId::Annuity)
            .copied()
            .unwrap_or(0.0);

        // DV01 = annuity * notional * 1bp, with sign based on pay/receive
        let dv01_magnitude = annuity * irs.notional.amount() * 1e-4;

        let dv01 = match irs.side {
            PayReceive::ReceiveFixed => dv01_magnitude,
            PayReceive::PayFixed => -dv01_magnitude,
        };

        Ok(dv01)
    }
}

/// Calculates PV of fixed leg for IRS.
///
/// Computes the present value of the fixed-rate leg by discounting
/// all fixed coupon payments using the swap's discount curve.
///
/// See unit tests and `examples/` for usage.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let disc = context.curves.discount(irs.fixed.disc_id)?;
        let base = disc.base_date();

        // Build fixed leg schedule and compute PV
        let sched = crate::cashflow::builder::build_dates(
            irs.fixed.start,
            irs.fixed.end,
            irs.fixed.freq,
            irs.fixed.stub,
            irs.fixed.bdc,
            irs.fixed.calendar_id,
        );
        let schedule: Vec<Date> = sched.dates;

        if schedule.len() < 2 {
            return Ok(0.0);
        }

        let mut pv = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, irs.fixed.dc);
            let coupon = irs.notional.amount() * irs.fixed.rate * yf;
            let df = DiscountCurve::df_on(&*disc, base, d, irs.fixed.dc);
            pv += coupon * df;
            prev = d;
        }

        Ok(pv)
    }
}

/// Calculates PV of floating leg for IRS.
///
/// Computes the present value of the floating-rate leg by discounting
/// all floating coupon payments. Forward rates are used to project
/// future floating rates, and the spread is added to each rate.
///
/// See unit tests and `examples/` for usage.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs = match &*context.instrument {
            Instrument::IRS(irs) => irs,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let disc = context.curves.discount(irs.float.disc_id)?;
        let fwd = context.curves.forecast(irs.float.fwd_id)?;
        let base = disc.base_date();

        // Build float leg schedule and compute PV
        let sched = crate::cashflow::builder::build_dates(
            irs.float.start,
            irs.float.end,
            irs.float.freq,
            irs.float.stub,
            irs.float.bdc,
            irs.float.calendar_id,
        );
        let schedule: Vec<Date> = sched.dates;

        if schedule.len() < 2 {
            return Ok(0.0);
        }

        let mut pv = 0.0;
        let mut prev = schedule[0];
        for &d in &schedule[1..] {
            let t1 = DiscountCurve::year_fraction(base, prev, irs.float.dc);
            let t2 = DiscountCurve::year_fraction(base, d, irs.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, irs.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;
            let df = DiscountCurve::df_on(&*disc, base, d, irs.float.dc);
            pv += coupon * df;
            prev = d;
        }

        Ok(pv)
    }
}

/// Registers all IRS metrics to a registry.
///
/// This function adds all IRS-specific metrics to the provided metric
/// registry. Each metric is registered with the "IRS" instrument type
/// to ensure proper applicability filtering.
///
/// # Arguments
/// * `registry` - Metric registry to add IRS metrics to
///
/// See unit tests and `examples/` for usage.
pub fn register_irs_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::Annuity,
            Arc::new(AnnuityCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::ParRate,
            Arc::new(ParRateCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::Dv01,
            Arc::new(Dv01Calculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::PvFixed,
            Arc::new(FixedLegPvCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::PvFloat,
            Arc::new(FloatLegPvCalculator),
            &["InterestRateSwap"],
        );
}
