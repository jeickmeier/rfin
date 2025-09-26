use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Asset Swap Spreads (Par and Market) using discount-curve annuity approximation.
///
/// Par ASW: spread s such that PV of fixed coupons at (df * (1 + s*alpha)) equals par.
/// Closed-form approximation: asw_par ≈ coupon - par_swap_rate.
///
/// Market ASW: spread s that equates PV of bond fixed leg to dirty market price.
/// Approximation: asw_mkt ≈ (dirty/Notional - price_pv/Notional)/annuity + coupon - par_rate.
pub struct AssetSwapParCalculator;
pub struct AssetSwapMarketCalculator;

fn fixed_leg_annuity(
    disc: &DiscountCurve,
    dc: finstack_core::dates::DayCount,
    schedule: &[finstack_core::dates::Date],
) -> F {
    if schedule.len() < 2 {
        return 0.0;
    }
    let mut ann = 0.0;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let alpha = dc
            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let p = disc.df_on_date_curve(d);
        ann += alpha * p;
        prev = d;
    }
    ann
}

fn build_future_dates_from_flows(
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
) -> Vec<finstack_core::dates::Date> {
    use finstack_core::dates::Date;
    use std::collections::BTreeSet;
    let mut set: BTreeSet<Date> = BTreeSet::new();
    for (d, _amt) in flows {
        if *d > as_of {
            set.insert(*d);
        }
    }
    let mut dates: Vec<Date> = Vec::with_capacity(set.len() + 1);
    dates.push(as_of);
    dates.extend(set);
    dates
}

fn pv_fixed_from_flows(
    disc: &DiscountCurve,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
) -> F {
    let mut pv = 0.0;
    for (date, amt) in flows {
        if *date <= as_of {
            continue;
        }
        let df = disc.df_on_date_curve(*date);
        pv += amt.amount() * df;
    }
    pv
}

/// Compute Par ASW using a forward-based methodology with explicit parameters.
pub fn asw_par_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: F,
) -> finstack_core::Result<F> {
    let disc = curves.get_discount_ref(bond.disc_id.clone())?;
    let fwd = curves.get_forward_ref(fwd_curve_id)?;

    // Mirror the bond schedule via holder flows
    let flows = bond.build_schedule(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }

    let ann = fixed_leg_annuity(disc, bond.dc, &sched);
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let f_base = fwd.base_date();
    let f_dc = fwd.day_count();
    let spread = float_spread_bp * 1e-4;
    let mut pv_float = 0.0;
    let mut prev = sched[0];
    for &d in &sched[1..] {
        let t1 = f_dc
            .year_fraction(f_base, prev, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let t2 = f_dc
            .year_fraction(f_base, d, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let yf = f_dc
            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let rate = fwd.rate_period(t1, t2) + spread;
        let coupon_flt = bond.notional.amount() * rate * yf;
        let df = disc.df_on_date_curve(d);
        pv_float += coupon_flt * df;
        prev = d;
    }
    let par_rate = pv_float / (bond.notional.amount() * ann);

    // Equivalent fixed rate from actual fixed flows
    let eq_coupon = pv_fixed_from_flows(disc, &flows, as_of) / (bond.notional.amount() * ann);
    Ok(eq_coupon - par_rate)
}

/// Compute Market ASW using forward-based methodology with explicit parameters.
pub fn asw_market_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: F,
    dirty_price_ccy: Option<F>,
) -> finstack_core::Result<F> {
    let disc = curves.get_discount_ref(bond.disc_id.clone())?;
    let flows = bond.build_schedule(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }
    let ann = fixed_leg_annuity(disc, bond.dc, &sched);
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let par_asw = asw_par_with_forward(bond, curves, as_of, fwd_curve_id, float_spread_bp)?;
    let pv_fixed = pv_fixed_from_flows(disc, &flows, as_of);
    let notional = bond.notional.amount();
    let price_pct = if let Some(dirty) = dirty_price_ccy {
        dirty / notional
    } else {
        1.0 * (pv_fixed / notional)
    };
    Ok(par_asw + (price_pct - pv_fixed / notional) / ann)
}

impl MetricCalculator for AssetSwapParCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;
        let disc_id = bond.disc_id.clone();
        let maturity = bond.maturity;
        let dc = bond.dc;
        let coupon = bond.coupon;
        let disc = context.curves.get_discount_ref(disc_id.clone())?;

        // Forward-based path intentionally not invoked here; use the explicit helpers instead

        // Fallback: Par swap rate via discount ratio on annual schedule
        let sched = crate::instruments::bond::pricing::schedule_helpers::build_annual_schedule(
            context.as_of,
            maturity,
        );
        if sched.len() < 2 {
            return Ok(0.0);
        }
        let p0 = disc.df_on_date_curve(sched[0]);
        let pn = disc.df_on_date_curve(*sched.last().unwrap());
        let num = p0 - pn;
        let ann = fixed_leg_annuity(disc, dc, &sched);
        if ann == 0.0 {
            return Ok(0.0);
        }
        let par_rate = num / ann;
        Ok(coupon - par_rate)
    }
}

impl MetricCalculator for AssetSwapMarketCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let (disc_id, maturity, dc, coupon, notional_amt, quoted_clean) = {
            let b: &Bond = context.instrument_as()?;
            (
                b.disc_id.clone(),
                b.maturity,
                b.dc,
                b.coupon,
                b.notional.amount(),
                b.pricing_overrides.quoted_clean_price,
            )
        };
        let disc = context.curves.get_discount_ref(disc_id.clone())?;

        // Dirty market value in currency
        let dirty_ccy = if let Some(clean_px) = quoted_clean {
            let accrued = context
                .computed
                .get(&MetricId::Accrued)
                .copied()
                .unwrap_or(0.0);
            clean_px * notional_amt / 100.0 + accrued
        } else {
            context.base_value.amount()
        };

        // Fixed leg PV at zero spread
        if context.cashflows.is_none() {
            let (disc_id_capture, dc_capture, built) = {
                let b: &Bond = context.instrument_as()?;
                (
                    b.disc_id.clone(),
                    b.dc,
                    b.build_schedule(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(disc_id_capture);
            context.day_count = Some(dc_capture);
        }
        let flows = context.cashflows.as_ref().unwrap();

        let mut pv_fixed = 0.0;
        for (date, amt) in flows {
            if *date <= context.as_of {
                continue;
            }
            let p = disc.df_on_date_curve(*date);
            pv_fixed += amt.amount() * p;
        }

        // Forward-based path when configured
        // Forward-based is available via asw_market_with_forward helper; here we keep fallback-only

        // Fallback: discount-ratio
        let sched = crate::instruments::bond::pricing::schedule_helpers::build_annual_schedule(
            context.as_of,
            maturity,
        );
        let ann = fixed_leg_annuity(disc, dc, &sched);
        if ann == 0.0 || notional_amt == 0.0 {
            return Ok(0.0);
        }
        let p0 = disc.df_on_date_curve(sched[0]);
        let pn = disc.df_on_date_curve(*sched.last().unwrap());
        let par_rate = (p0 - pn) / ann;
        let price_pct = dirty_ccy / notional_amt;
        let asw_mkt = (price_pct - pv_fixed / notional_amt) / ann + (coupon - par_rate);
        Ok(asw_mkt)
    }
}
