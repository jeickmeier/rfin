//! Effective duration and convexity for bonds with embedded options.
//!
//! For callable/putable bonds, yield-based modified duration and convexity are
//! inappropriate because they assume fixed cashflows. Effective duration and
//! convexity measure price sensitivity by bumping the discount curve and
//! repricing through the tree, which accounts for changes in exercise behavior
//! as rates move.
//!
//! # Formulas
//!
//! ```text
//! D_eff = (P_down - P_up) / (2 * P_base * shock)
//! C_eff = (P_up + P_down - 2 * P_base) / (P_base * shock^2)
//! ```
//!
//! where `shock` is the parallel rate bump in decimal (e.g., 0.0025 for 25 bps).

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::Bond;
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::MarketBump;
use finstack_core::market_data::context::{BumpSpec, MarketContext};
use finstack_core::types::CurveId;
use finstack_core::Result;

const DEFAULT_SHOCK_BPS: f64 = 25.0;

/// Effective duration and convexity result.
#[derive(Debug, Clone)]
#[allow(dead_code)] // public API result struct
pub(crate) struct EffectiveDurationResult {
    pub duration: f64,
    pub convexity: f64,
    pub base_price: f64,
    pub price_up: f64,
    pub price_down: f64,
    pub shock_bps: f64,
}

/// Calculate effective duration for a bond using parallel curve bumps.
///
/// For bonds without embedded options, this produces results very close to
/// modified duration. For callable/putable bonds, the tree pricer captures
/// the change in exercise behavior as rates shift.
pub(crate) fn effective_duration(
    bond: &Bond,
    market: &MarketContext,
    as_of: Date,
    shock_bps: Option<f64>,
) -> Result<f64> {
    Ok(effective_duration_convexity(bond, market, as_of, shock_bps)?.duration)
}

/// Calculate effective convexity for a bond using parallel curve bumps.
pub(crate) fn effective_convexity(
    bond: &Bond,
    market: &MarketContext,
    as_of: Date,
    shock_bps: Option<f64>,
) -> Result<f64> {
    Ok(effective_duration_convexity(bond, market, as_of, shock_bps)?.convexity)
}

/// Calculate both effective duration and convexity in one pass (three pricings).
pub(crate) fn effective_duration_convexity(
    bond: &Bond,
    market: &MarketContext,
    as_of: Date,
    shock_bps: Option<f64>,
) -> Result<EffectiveDurationResult> {
    let shock_bps = shock_bps.unwrap_or(DEFAULT_SHOCK_BPS);
    let shock = shock_bps / 10_000.0;

    let (risk_bond, base_price) = option_risk_bond_and_base_price(bond, market, as_of)?;

    if base_price.abs() < 1e-10 {
        return Ok(EffectiveDurationResult {
            duration: 0.0,
            convexity: 0.0,
            base_price,
            price_up: 0.0,
            price_down: 0.0,
            shock_bps,
        });
    }

    let curve_id = option_risk_curve_id(&risk_bond);
    let market_up = market.bump([MarketBump::Curve {
        id: curve_id.clone(),
        spec: BumpSpec::parallel_bp(shock_bps),
    }])?;
    let market_down = market.bump([MarketBump::Curve {
        id: curve_id,
        spec: BumpSpec::parallel_bp(-shock_bps),
    }])?;

    let price_up = risk_bond.value(&market_up, as_of)?.amount();
    let price_down = risk_bond.value(&market_down, as_of)?.amount();

    let duration = (price_down - price_up) / (2.0 * base_price * shock);
    let convexity = (price_up + price_down - 2.0 * base_price) / (base_price * shock * shock);

    Ok(EffectiveDurationResult {
        duration,
        convexity,
        base_price,
        price_up,
        price_down,
        shock_bps,
    })
}

pub(crate) fn option_risk_bond_and_base_price(
    bond: &Bond,
    market: &MarketContext,
    as_of: Date,
) -> Result<(Bond, f64)> {
    use crate::instruments::fixed_income::bond::pricing::engine::tree::{
        bond_tree_config, TreePricer,
    };
    use crate::instruments::fixed_income::bond::pricing::quote_conversions::{
        clear_price_driving_overrides, price_from_quote_overrides,
    };
    use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;

    let mut risk_bond = bond.clone();
    let Some(base_price) = price_from_quote_overrides(bond, market, as_of)? else {
        return Ok((risk_bond.clone(), risk_bond.value(market, as_of)?.amount()));
    };

    if let Some(oas) = bond.pricing_overrides.market_quotes.quoted_oas {
        clear_price_driving_overrides(&mut risk_bond);
        risk_bond.pricing_overrides.market_quotes.quoted_oas = Some(oas);
        return Ok((risk_bond, base_price));
    }

    let quote_ctx = QuoteDateContext::new(bond, market, as_of)?;
    let clean_price_pct =
        (base_price - quote_ctx.accrued_at_quote_date) / bond.notional.amount() * 100.0;
    let oas_bp = TreePricer::with_config(bond_tree_config(bond)).calculate_oas(
        bond,
        market,
        as_of,
        clean_price_pct,
    )?;

    clear_price_driving_overrides(&mut risk_bond);
    risk_bond.pricing_overrides.market_quotes.quoted_oas = Some(oas_bp / 10_000.0);
    Ok((risk_bond, base_price))
}

pub(crate) fn option_risk_curve_id(bond: &Bond) -> CurveId {
    bond.pricing_overrides
        .model_config
        .tree_discount_curve_id
        .clone()
        .unwrap_or_else(|| bond.discount_curve_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::{Bond, CallPut, CallPutSchedule, CashflowSpec};
    use crate::instruments::PricingOverrides;
    use crate::metrics::{standard_registry, MetricContext, MetricId};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Tenor};
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use std::sync::Arc;
    use time::Month;

    fn test_market(as_of: finstack_core::dates::Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (3.0, 0.88),
                (5.0, 0.80),
                (10.0, 0.65),
            ])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("valid curve");
        MarketContext::new().insert(disc)
    }

    fn bullet_bond(as_of: finstack_core::dates::Date) -> Bond {
        let maturity = as_of + time::Duration::days(5 * 365);
        Bond::builder()
            .id("BULLET".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue_date(as_of)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Tenor::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Default::default())
            .build()
            .expect("valid bond")
    }

    fn callable_bond(as_of: finstack_core::dates::Date) -> Bond {
        let maturity = as_of + time::Duration::days(5 * 365);
        let call_date = as_of + time::Duration::days(2 * 365);
        let mut bond = Bond::builder()
            .id("CALLABLE".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue_date(as_of)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Tenor::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Default::default())
            .build()
            .expect("valid bond");

        let mut schedule = CallPutSchedule::default();
        schedule.calls.push(CallPut {
            start_date: call_date,
            end_date: maturity,
            price_pct_of_par: 100.0,
            make_whole: None,
        });
        bond.call_put = Some(schedule);
        bond
    }

    #[test]
    fn bullet_effective_duration_matches_modified() {
        let as_of =
            finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).expect("ok");
        let market = test_market(as_of);
        let bond = bullet_bond(as_of);

        let eff = effective_duration_convexity(&bond, &market, as_of, Some(25.0))
            .expect("effective calc");

        // Compute modified duration via the metrics registry
        let base_pv = bond.value(&market, as_of).expect("value");
        let instrument_arc: Arc<dyn Instrument> = Arc::new(bond);
        let curves_arc = Arc::new(market);
        let registry = standard_registry();
        let mut ctx = MetricContext::new(
            instrument_arc,
            curves_arc,
            as_of,
            base_pv,
            MetricContext::default_config(),
        );
        registry
            .compute(
                &[
                    MetricId::Accrued,
                    MetricId::Ytm,
                    MetricId::DurationMac,
                    MetricId::DurationMod,
                ],
                &mut ctx,
            )
            .expect("metrics");

        let d_mod = ctx
            .computed
            .get(&MetricId::DurationMod)
            .copied()
            .expect("DurationMod metric should be computed");

        // For a bullet bond, effective duration ≈ modified duration (within ~0.5 due to bump size)
        assert!(
            (eff.duration - d_mod).abs() < 0.5,
            "Effective duration ({:.4}) should be close to modified duration ({:.4})",
            eff.duration,
            d_mod,
        );
    }

    #[test]
    fn callable_effective_duration_lower_than_bullet() {
        let as_of =
            finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).expect("ok");
        let market = test_market(as_of);

        let bullet = bullet_bond(as_of);
        let callable = callable_bond(as_of);

        let eff_bullet =
            effective_duration(&bullet, &market, as_of, Some(25.0)).expect("bullet eff dur");
        let eff_callable =
            effective_duration(&callable, &market, as_of, Some(25.0)).expect("callable eff dur");

        // Callable bond effective duration <= bullet (call caps upside)
        assert!(
            eff_callable <= eff_bullet + 0.01,
            "Callable effective duration ({:.4}) should be <= bullet ({:.4})",
            eff_callable,
            eff_bullet,
        );
    }

    #[test]
    fn callable_effective_convexity_lower_than_bullet() {
        let as_of =
            finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).expect("ok");
        let market = test_market(as_of);

        let bullet = bullet_bond(as_of);
        let callable = callable_bond(as_of);

        let eff_bullet =
            effective_duration_convexity(&bullet, &market, as_of, Some(25.0)).expect("bullet");
        let eff_callable =
            effective_duration_convexity(&callable, &market, as_of, Some(25.0)).expect("callable");

        // Callable convexity should be lower (possibly negative) relative to bullet
        assert!(
            eff_callable.convexity <= eff_bullet.convexity + 1.0,
            "Callable effective convexity ({:.4}) should be <= bullet ({:.4})",
            eff_callable.convexity,
            eff_bullet.convexity,
        );
    }
}
