use crate::cashflow::traits::CashflowProvider;
use crate::cashflow::{builder::CashFlowSchedule, primitives::CFKind};
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Asset Swap Spreads (Par and Market) using discount-curve annuity approximation.
///
/// Par ASW: spread s such that PV of fixed coupons at (df * (1 + s*alpha)) equals par.
/// Closed-form approximation: asw_par ≈ coupon - par_swap_rate.
///
/// Market ASW: spread s that equates PV of bond fixed leg to dirty market price.
/// Approximation: asw_mkt ≈ (dirty/Notional - price_pv/Notional)/annuity + coupon - par_rate.
pub struct AssetSwapParCalculator;
/// Asset swap spread calculator using market price
pub struct AssetSwapMarketCalculator;
/// Asset swap par spread calculator using forward method
pub struct AssetSwapParFwdCalculator;
/// Asset swap market spread calculator using forward method
pub struct AssetSwapMarketFwdCalculator;

fn fixed_leg_annuity(
    disc: &DiscountCurve,
    dc: finstack_core::dates::DayCount,
    schedule: &[finstack_core::dates::Date],
) -> f64 {
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

/// PV of coupon-only leg from a custom schedule (excludes amortization and principal).
fn pv_coupon_from_custom_schedule(
    disc: &DiscountCurve,
    schedule: &CashFlowSchedule,
    as_of: Date,
) -> f64 {
    let mut pv = 0.0;
    for cf in &schedule.flows {
        if cf.date <= as_of {
            continue;
        }
        match cf.kind {
            CFKind::Fixed | CFKind::Stub => {
                let df = disc.df_on_date_curve(cf.date);
                pv += cf.amount.amount() * df;
            }
            _ => {}
        }
    }
    pv
}

/// Compute Par ASW using a forward-based methodology with explicit parameters.
pub fn asw_par_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
) -> finstack_core::Result<f64> {
    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;
    let fwd = curves.get_forward_ref(fwd_curve_id)?;

    // Mirror the bond schedule via holder flows
    let flows = bond.build_schedule(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }

    let ann = fixed_leg_annuity(disc, bond.cashflow_spec.day_count(), &sched);
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let f_base = fwd.base_date();
    let f_dc = fwd.day_count();
    let spread = float_spread_bp * 1e-4;
    let mut pv_float = 0.0;
    let mut prev = sched[0];
    for &d in &sched[1..] {
        let t1 = f_dc.year_fraction(f_base, prev, finstack_core::dates::DayCountCtx::default())?;
        let t2 = f_dc.year_fraction(f_base, d, finstack_core::dates::DayCountCtx::default())?;
        let yf = f_dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
        let rate = fwd.rate_period(t1, t2) + spread;
        let coupon_flt = bond.notional.amount() * rate * yf;
        let df = disc.df_on_date_curve(d);
        pv_float += coupon_flt * df;
        prev = d;
    }
    let par_rate = pv_float / (bond.notional.amount() * ann);

    // Equivalent fixed rate from coupon-only PV
    let eq_coupon = if let Some(custom) = &bond.custom_cashflows {
        let pv_coupon = pv_coupon_from_custom_schedule(disc, custom, as_of);
        pv_coupon / (bond.notional.amount() * ann)
    } else {
        // Extract fixed coupon rate from cashflow_spec
        match &bond.cashflow_spec {
            super::super::CashflowSpec::Fixed(spec) => spec.rate,
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        }
    };
    Ok(eq_coupon - par_rate)
}

/// Compute Market ASW using forward-based methodology with explicit parameters.
pub fn asw_market_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
    dirty_price_ccy: Option<f64>,
) -> finstack_core::Result<f64> {
    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;
    let flows = bond.build_schedule(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }
    let ann = fixed_leg_annuity(disc, bond.cashflow_spec.day_count(), &sched);
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let par_asw = asw_par_with_forward(bond, curves, as_of, fwd_curve_id, float_spread_bp)?;
    let notional = bond.notional.amount();
    let price_pct = if let Some(dirty) = dirty_price_ccy {
        dirty / notional
    } else {
        1.0 // Assume par if no price provided
    };
    // Market ASW = Par ASW + (Market Price % - 100%) / Annuity
    Ok(par_asw + (price_pct - 1.0) / ann)
}

impl MetricCalculator for AssetSwapParCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // If the bond has custom cashflows, compute ASW using a forward-based
        // custom-swap constructed on the same schedule. Requires a float spec.
        if bond.custom_cashflows.is_some() {
            match &bond.cashflow_spec {
                super::super::CashflowSpec::Floating(spec) => {
                    return asw_par_with_forward(
                        bond,
                        &context.curves,
                        context.as_of,
                        spec.rate_spec.index_id.as_str(),
                        spec.rate_spec.spread_bp,
                    );
                }
                _ => {
                    return Err(finstack_core::error::InputError::NotFound {
                        id: "bond.cashflow_spec.floating".to_string(),
                    }
                    .into());
                }
            }
        }

        let discount_curve_id = bond.discount_curve_id.to_owned();
        let maturity = bond.maturity;
        let dc = bond.cashflow_spec.day_count();
        let disc = context.curves.get_discount_ref(&discount_curve_id)?;

        // Extract schedule params from cashflow_spec
        let (freq, bdc, calendar_id, stub) = match &bond.cashflow_spec {
            super::super::CashflowSpec::Fixed(spec) => {
                (spec.freq, spec.bdc, spec.calendar_id.as_deref(), spec.stub)
            }
            super::super::CashflowSpec::Floating(spec) => (
                spec.freq,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
                spec.stub,
            ),
            super::super::CashflowSpec::Amortizing { base, .. } => match &**base {
                super::super::CashflowSpec::Fixed(spec) => {
                    (spec.freq, spec.bdc, spec.calendar_id.as_deref(), spec.stub)
                }
                super::super::CashflowSpec::Floating(spec) => (
                    spec.freq,
                    spec.rate_spec.bdc,
                    spec.rate_spec.calendar_id.as_deref(),
                    spec.stub,
                ),
                _ => return Err(finstack_core::error::InputError::Invalid.into()),
            },
        };

        // Market standard: Par swap rate via discount ratio on bond's actual payment schedule
        let sched = crate::instruments::bond::pricing::schedule_helpers::build_bond_schedule(
            context.as_of,
            maturity,
            freq,
            stub,
            bdc,
            calendar_id,
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
        // Use stated coupon for non-custom bonds; for custom bonds, this branch is not reached
        let coupon = match &bond.cashflow_spec {
            super::super::CashflowSpec::Fixed(spec) => spec.rate,
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        };
        Ok(coupon - par_rate)
    }
}

impl MetricCalculator for AssetSwapMarketCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let (discount_curve_id, maturity, dc, notional_amt, quoted_clean, is_custom, coupon) = {
            let b: &Bond = context.instrument_as()?;
            let coupon_rate = match &b.cashflow_spec {
                super::super::CashflowSpec::Fixed(spec) => spec.rate,
                _ => 0.0, // Will be handled later if needed
            };
            (
                b.discount_curve_id.to_owned(),
                b.maturity,
                b.cashflow_spec.day_count(),
                b.notional.amount(),
                b.pricing_overrides.quoted_clean_price,
                b.custom_cashflows.is_some(),
                coupon_rate,
            )
        };
        let disc = context.curves.get_discount_ref(&discount_curve_id)?;

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

        // If the bond has custom cashflows, compute forward-based ASW using the
        // bond's float spec on the same (custom) schedule. Requires a float spec.
        if is_custom {
            let bond: &Bond = context.instrument_as()?;
            match &bond.cashflow_spec {
                super::super::CashflowSpec::Floating(spec) => {
                    return asw_market_with_forward(
                        bond,
                        &context.curves,
                        context.as_of,
                        spec.rate_spec.index_id.as_str(),
                        spec.rate_spec.spread_bp,
                        Some(dirty_ccy),
                    );
                }
                _ => {
                    return Err(finstack_core::error::InputError::NotFound {
                        id: "bond.cashflow_spec.floating".to_string(),
                    }
                    .into());
                }
            }
        }

        // Fixed coupon-leg PV (exclude principal) at zero spread
        if context.cashflows.is_none() {
            let (disc_id_capture, dc_capture, built) = {
                let b: &Bond = context.instrument_as()?;
                (
                    b.discount_curve_id.to_owned(),
                    b.cashflow_spec.day_count(),
                    b.build_schedule(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(disc_id_capture);
            context.day_count = Some(dc_capture);
        }
        let _flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "cashflows".to_string(),
            })
        })?;
        let _pv_coupon_only = if let Some(custom) =
            &context.instrument_as::<Bond>()?.custom_cashflows
        {
            pv_coupon_from_custom_schedule(disc, custom, context.as_of)
        } else {
            // For standard bonds, coupon PV uses bond's actual payment schedule
            let bond: &Bond = context.instrument_as()?;
            let (freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
                super::super::CashflowSpec::Fixed(spec) => {
                    (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
                }
                super::super::CashflowSpec::Floating(spec) => (
                    spec.freq,
                    spec.stub,
                    spec.rate_spec.bdc,
                    spec.rate_spec.calendar_id.as_deref(),
                ),
                super::super::CashflowSpec::Amortizing { base, .. } => match &**base {
                    super::super::CashflowSpec::Fixed(spec) => {
                        (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
                    }
                    super::super::CashflowSpec::Floating(spec) => (
                        spec.freq,
                        spec.stub,
                        spec.rate_spec.bdc,
                        spec.rate_spec.calendar_id.as_deref(),
                    ),
                    _ => return Err(finstack_core::error::InputError::Invalid.into()),
                },
            };
            let sched = crate::instruments::bond::pricing::schedule_helpers::build_bond_schedule(
                context.as_of,
                maturity,
                freq,
                stub,
                bdc,
                calendar_id,
            );
            let ann = fixed_leg_annuity(disc, dc, &sched);
            notional_amt * coupon * ann
        };

        // Forward-based path when configured for non-custom bonds is available
        // via explicit helper methods (ASW*Fwd calculators). Here we keep fallback-only.

        // Market standard: discount-ratio using bond's payment schedule
        let bond: &Bond = context.instrument_as()?;
        let (freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
            super::super::CashflowSpec::Fixed(spec) => {
                (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
            }
            super::super::CashflowSpec::Floating(spec) => (
                spec.freq,
                spec.stub,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
            ),
            super::super::CashflowSpec::Amortizing { base, .. } => match &**base {
                super::super::CashflowSpec::Fixed(spec) => {
                    (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
                }
                super::super::CashflowSpec::Floating(spec) => (
                    spec.freq,
                    spec.stub,
                    spec.rate_spec.bdc,
                    spec.rate_spec.calendar_id.as_deref(),
                ),
                _ => return Err(finstack_core::error::InputError::Invalid.into()),
            },
        };
        let sched = crate::instruments::bond::pricing::schedule_helpers::build_bond_schedule(
            context.as_of,
            maturity,
            freq,
            stub,
            bdc,
            calendar_id,
        );
        let ann = fixed_leg_annuity(disc, dc, &sched);
        if ann == 0.0 || notional_amt == 0.0 {
            return Ok(0.0);
        }
        let p0 = disc.df_on_date_curve(sched[0]);
        let pn = disc.df_on_date_curve(*sched.last().unwrap());
        let par_rate = (p0 - pn) / ann;
        // Equivalent coupon from coupon PV only for custom bonds; otherwise stated coupon
        let eq_coupon = if let Some(custom) = &context.instrument_as::<Bond>()?.custom_cashflows {
            let pv_coupon = pv_coupon_from_custom_schedule(disc, custom, context.as_of);
            pv_coupon / (notional_amt * ann)
        } else {
            coupon
        };
        // Market ASW = Par ASW + (Market Price % - 100%) / Annuity
        let price_pct = dirty_ccy / notional_amt;
        let asw_mkt = (eq_coupon - par_rate) + (price_pct - 1.0) / ann;
        Ok(asw_mkt)
    }
}

impl MetricCalculator for AssetSwapParFwdCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount_ref(bond.discount_curve_id.as_str())?;
        let as_of = disc.base_date();
        match &bond.cashflow_spec {
            super::super::CashflowSpec::Floating(spec) => asw_par_with_forward(
                bond,
                &context.curves,
                as_of,
                spec.rate_spec.index_id.as_str(),
                spec.rate_spec.spread_bp,
            ),
            _ => Err(finstack_core::error::InputError::NotFound {
                id: "bond.cashflow_spec.floating".to_string(),
            }
            .into()),
        }
    }
}

impl MetricCalculator for AssetSwapMarketFwdCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount_ref(bond.discount_curve_id.as_str())?;
        let as_of = disc.base_date();
        match &bond.cashflow_spec {
            super::super::CashflowSpec::Floating(spec) => {
                let dirty = if let Some(clean) = bond.pricing_overrides.quoted_clean_price {
                    let accrued = *context.computed.get(&MetricId::Accrued).unwrap_or(&0.0);
                    Some(clean * bond.notional.amount() / 100.0 + accrued)
                } else {
                    Some(context.base_value.amount())
                };
                asw_market_with_forward(
                    bond,
                    &context.curves,
                    as_of,
                    spec.rate_spec.index_id.as_str(),
                    spec.rate_spec.spread_bp,
                    dirty,
                )
            }
            _ => Err(finstack_core::error::InputError::NotFound {
                id: "bond.cashflow_spec.floating".to_string(),
            }
            .into()),
        }
    }
}
