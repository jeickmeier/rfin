//! Bond pricing helpers (moved from bond/helpers.rs)

use super::super::types::Bond;
use crate::cashflow::traits::CashflowProvider;
use crate::metrics::MetricContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldCompounding {
    Simple,
    Annual,
    Periodic(u32),
    Continuous,
    Street,
}

#[inline]
pub fn periods_per_year(
    freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<finstack_core::F> {
    match freq {
        finstack_core::dates::Frequency::Months(m) => {
            if m == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(12.0 / (m as finstack_core::F))
        }
        finstack_core::dates::Frequency::Days(d) => {
            if d == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(365.0 / (d as finstack_core::F))
        }
        _ => Err(finstack_core::error::InputError::Invalid.into()),
    }
}

#[inline]
pub fn df_from_yield(
    ytm: finstack_core::F,
    t: finstack_core::F,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<finstack_core::F> {
    if t <= 0.0 {
        return Ok(1.0);
    }
    Ok(match comp {
        YieldCompounding::Simple => 1.0 / (1.0 + ytm * t),
        YieldCompounding::Annual => (1.0 + ytm).powf(-t),
        YieldCompounding::Periodic(m) => {
            let m = m as finstack_core::F;
            (1.0 + ytm / m).powf(-m * t)
        }
        YieldCompounding::Continuous => (-ytm * t).exp(),
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq)?.max(1.0);
            (1.0 + ytm / m).powf(-m * t)
        }
    })
}

#[inline]
pub fn df_and_derivative_from_yield(
    ytm: finstack_core::F,
    t: finstack_core::F,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<(finstack_core::F, finstack_core::F)> {
    let df = df_from_yield(ytm, t, comp, bond_freq)?;
    if t <= 0.0 {
        return Ok((df, 0.0));
    }
    let ddf_dy = match comp {
        YieldCompounding::Simple => {
            let denom = 1.0 + ytm * t;
            -t / (denom * denom)
        }
        YieldCompounding::Annual => -t * df / (1.0 + ytm),
        YieldCompounding::Periodic(m) => {
            let m = m as finstack_core::F;
            -t * df / (1.0 + ytm / m)
        }
        YieldCompounding::Continuous => -t * df,
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq)?.max(1.0);
            -t * df / (1.0 + ytm / m)
        }
    };
    Ok((df, ddf_dy))
}

pub fn flows_from_context_or_build(
    context: &mut MetricContext,
    bond: &Bond,
) -> finstack_core::Result<Vec<(finstack_core::dates::Date, finstack_core::money::Money)>> {
    if let Some(flows) = &context.cashflows {
        return Ok(flows.clone());
    }
    let flows = bond.build_schedule(&context.curves, context.as_of)?;
    context.discount_curve_id = Some(bond.disc_id.clone());
    context.day_count = Some(bond.dc);
    context.cashflows = Some(flows.clone());
    Ok(flows)
}

pub fn price_from_ytm(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: finstack_core::F,
) -> finstack_core::Result<finstack_core::F> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
}

pub fn price_from_ytm_compounded(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: finstack_core::F,
    comp: YieldCompounding,
) -> finstack_core::Result<finstack_core::F> {
    let mut pv = 0.0;
    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = bond
            .dc
            .year_fraction(as_of, date, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        if t > 0.0 {
            let df = df_from_yield(ytm, t, comp, bond.freq)?;
            pv += amount.amount() * df;
        }
    }
    Ok(pv)
}
