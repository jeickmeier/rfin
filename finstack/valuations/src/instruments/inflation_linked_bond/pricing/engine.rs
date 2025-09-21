//! Core ILB pricing engine.
//!
//! Provides:
//! - Inflation index ratio computation with lag and optional floors
//! - Schedule construction via shared cashflow builder
//! - PV via standard cashflow discounting path (using curve day-count basis)
//!
//! This engine centralizes pricing logic to keep `types.rs` ergonomic and
//! declarative.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use crate::instruments::inflation_linked_bond::types::{DeflationProtection, InflationLinkedBond};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use time::Duration;

/// Pricing engine for `InflationLinkedBond`.
pub struct InflationLinkedBondEngine;

impl InflationLinkedBondEngine {
    /// Compute the index ratio on a given date using the instrument's
    /// lag and deflation protection policy.
    pub fn index_ratio(
        ilb: &InflationLinkedBond,
        date: Date,
        inflation_index: &InflationIndex,
    ) -> Result<finstack_core::F> {
        // Validate interpolation policy vs indexation method for common standards
        match ilb.indexation_method {
            super::super::types::IndexationMethod::TIPS
            | super::super::types::IndexationMethod::Canadian => {
                if inflation_index.interpolation() != InflationInterpolation::Linear {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
            }
            super::super::types::IndexationMethod::UK => {
                if inflation_index.interpolation() != InflationInterpolation::Step {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
            }
            _ => {}
        }

        // Apply lag to obtain the reference date in index space
        let reference_date = match ilb.lag {
            InflationLag::Months(m) => finstack_core::dates::add_months(date, -(m as i32)),
            InflationLag::Days(d) => date - Duration::days(d as i64),
            InflationLag::None => date,
            _ => date,
        };

        // Value on reference date (interpolation policy controlled by index)
        let current_index = inflation_index.value_on(reference_date)?;

        // Ratio vs base
        if ilb.base_index <= 0.0 {
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
        }
        let ratio = current_index / ilb.base_index;

        // Apply deflation protection per instrument policy
        Ok(match ilb.deflation_protection {
            DeflationProtection::None => ratio,
            DeflationProtection::MaturityOnly => {
                if date == ilb.maturity {
                    ratio.max(1.0)
                } else {
                    ratio
                }
            }
            DeflationProtection::AllPayments => ratio.max(1.0),
        })
    }

    /// Build an inflation-adjusted coupon and principal schedule.
    pub fn build_schedule(
        ilb: &InflationLinkedBond,
        curves: &MarketContext,
        _as_of: Date,
    ) -> Result<DatedFlows> {
        let inflation_index = curves
            .inflation_index(ilb.inflation_id.as_str())
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_linked_bond_quote".to_string(),
                })
            })?;

        // Base coupon dates via shared builder
        let sched = crate::cashflow::builder::build_dates(
            ilb.issue,
            ilb.maturity,
            ilb.freq,
            ilb.stub,
            ilb.bdc,
            ilb.calendar_id,
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(vec![]);
        }

        let mut flows = Vec::with_capacity(dates.len());
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let year_frac = ilb
                .dc
                .year_fraction(prev, d, DayCountCtx::default())?
                .max(0.0);
            let base_amount = ilb.notional * ilb.real_coupon * year_frac;
            let ratio = Self::index_ratio(ilb, d, &inflation_index)?;
            flows.push((d, base_amount * ratio));
            prev = d;
        }

        // Principal repayment at maturity (inflation adjusted)
        let principal_ratio = Self::index_ratio(ilb, ilb.maturity, &inflation_index)?;
        flows.push((ilb.maturity, ilb.notional * principal_ratio));

        Ok(flows)
    }

    /// Present value using standard cashflow discounting.
    pub fn pv(ilb: &InflationLinkedBond, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let flows = Self::build_schedule(ilb, curves, as_of)?;
        let disc = curves.get_ref::<DiscountCurve>(ilb.disc_id.as_str())?;
        let base_date = disc.base_date();
        // Use curve basis for time mapping
        let dc = disc.day_count();
        flows.npv(disc, base_date, dc)
    }

    /// Solve real yield by matching the real dirty price to cashflows discounted by the yield.
    /// Expects `clean_price` in price-per-100 notional terms.
    pub fn real_yield(
        ilb: &InflationLinkedBond,
        clean_price: finstack_core::F,
        _curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::F> {
        use crate::instruments::bond::pricing::helpers::YieldCompounding;
        use crate::instruments::bond::pricing::ytm_solver::{solve_ytm, YtmPricingSpec};

        if !clean_price.is_finite() || clean_price <= 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        // Build real cashflows (already inflation-adjusted amounts at payment dates)
        // Note: Here we approximate by using the instrument schedule with ILB cash amounts;
        // accrued real interest is small relative to coupon accuracy. A future enhancement can
        // compute real accrued to convert clean→dirty precisely.
        let flows = Self::build_schedule(ilb, &_curves.clone(), as_of)?;
        if flows.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }

        // Convert clean price (per 100) to Money in instrument currency
        let target_price = Money::new(
            clean_price / 100.0 * ilb.notional.amount(),
            ilb.notional.currency(),
        );

        let spec = YtmPricingSpec {
            day_count: ilb.dc,
            notional: ilb.notional,
            coupon_rate: ilb.real_coupon,
            compounding: YieldCompounding::Street,
            frequency: ilb.freq,
        };
        // Solve yield that matches the target price to PV of flows on (as_of)
        let y = solve_ytm(&flows, as_of, target_price, spec)?;
        // Clamp extreme values to avoid explosive outputs
        Ok(y.clamp(-0.99, 2.0))
    }

    /// Real duration (modified) computed via central difference on price vs real yield.
    pub fn real_duration(
        ilb: &InflationLinkedBond,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::F> {
        // Determine a base clean price to center the bump around
        let base_clean = ilb.quoted_clean.unwrap_or(100.0);
        // Compute base yield
        let y0 = Self::real_yield(ilb, base_clean, curves, as_of)?;
        // Bump yield by 1bp in decimal terms
        let bp = 1e-4;
        // Price function from yield using helper
        use crate::instruments::bond::pricing::helpers::{
            price_from_ytm_compounded_params, YieldCompounding,
        };
        let flows = Self::build_schedule(ilb, curves, as_of)?;
        // Convert price from ytm helpers returns currency units; convert back to clean per-100 notionally
        let price_from_yield = |y: f64| -> finstack_core::F {
            price_from_ytm_compounded_params(
                ilb.dc,
                ilb.freq,
                &flows,
                as_of,
                y,
                YieldCompounding::Street,
            )
            .unwrap_or(0.0)
                / ilb.notional.amount()
                * 100.0
        };
        let p_up = price_from_yield(y0 + bp);
        let p_dn = price_from_yield(y0 - bp);
        let dp_dy = (p_up - p_dn) / (2.0 * bp);
        // Modified duration in years per 1 delta in yield: D = - (1/P) * dP/dy
        let p0 = base_clean.max(1e-6);
        Ok(-(dp_dy / p0))
    }
}

impl CashflowProvider for InflationLinkedBond {
    fn build_schedule(&self, curves: &MarketContext, as_of: Date) -> Result<DatedFlows> {
        InflationLinkedBondEngine::build_schedule(self, curves, as_of)
    }
}
