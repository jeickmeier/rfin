//! Builders for bond instruments from market quotes.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::fixed_income::bond::pricing::quote_conversions::{
    df_from_yield, YieldCompounding,
};
use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;
use crate::instruments::{Bond, DynInstrument, PricingOverrides};
use crate::market::quotes::bond::BondQuote;
use crate::market::BuildCtx;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCountContext;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::NeumaierAccumulator;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Build a bond instrument from a [`BondQuote`].
///
/// `market` is required for spread-based quotes (Z-spread, OAS) that need a
/// discount curve to normalize to clean price. Pass `None` for clean-price
/// and YTM quotes.
pub fn build_bond_instrument(
    quote: &BondQuote,
    ctx: &BuildCtx,
    market: Option<&MarketContext>,
) -> Result<Box<DynInstrument>> {
    tracing::debug!(quote_id = %quote.id(), "building bond instrument");
    match quote {
        BondQuote::FixedRateBulletCleanPrice {
            id,
            currency,
            issue_date,
            maturity,
            coupon_rate,
            convention,
            clean_price_pct,
        } => {
            let mut bond = build_bond_shell(
                id.as_str(),
                *currency,
                *coupon_rate,
                *issue_date,
                *maturity,
                convention,
                ctx,
            )?;
            bond.pricing_overrides =
                PricingOverrides::default().with_quoted_clean_price(*clean_price_pct);
            Ok(Box::new(bond))
        }
        BondQuote::FixedRateBulletYtm {
            id,
            currency,
            issue_date,
            maturity,
            coupon_rate,
            convention,
            ytm,
        } => {
            let mut bond = build_bond_shell(
                id.as_str(),
                *currency,
                *coupon_rate,
                *issue_date,
                *maturity,
                convention,
                ctx,
            )?;

            let empty_market = MarketContext::new();
            let market = market.unwrap_or(&empty_market);
            let quote_ctx = QuoteDateContext::new(&bond, market, ctx.as_of())?;
            let flows = <Bond as CashflowProvider>::dated_cashflows(&bond, market, ctx.as_of())?;
            let dirty_price_ccy =
                dirty_price_from_ytm_with_frequency_ctx(&bond, &flows, quote_ctx.quote_date, *ytm)?;
            if bond.notional.amount().abs() < crate::constants::numerical::ZERO_TOLERANCE {
                return Err(finstack_core::Error::Validation(
                    "bond notional must be non-zero for price conversion".to_string(),
                ));
            }
            let clean_price_pct = (dirty_price_ccy - quote_ctx.accrued_at_quote_date)
                / bond.notional.amount()
                * 100.0;

            bond.pricing_overrides =
                PricingOverrides::default().with_quoted_clean_price(clean_price_pct);
            Ok(Box::new(bond))
        }
        BondQuote::FixedRateBulletZSpread {
            id,
            currency,
            issue_date,
            maturity,
            coupon_rate,
            convention,
            z_spread,
        } => {
            let market = market.ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Z-spread bond quote requires a MarketContext with a discount curve"
                        .to_string(),
                )
            })?;
            let mut bond = build_bond_shell(
                id.as_str(),
                *currency,
                *coupon_rate,
                *issue_date,
                *maturity,
                convention,
                ctx,
            )?;
            let quote_ctx = QuoteDateContext::new(&bond, market, ctx.as_of())?;
            let dirty_price_ccy =
                crate::instruments::fixed_income::bond::pricing::quote_conversions::price_from_z_spread(
                    &bond,
                    market,
                    quote_ctx.quote_date,
                    *z_spread,
                )?;
            if bond.notional.amount().abs() < crate::constants::numerical::ZERO_TOLERANCE {
                return Err(finstack_core::Error::Validation(
                    "bond notional must be non-zero for price conversion".to_string(),
                ));
            }
            let clean_price_pct = (dirty_price_ccy - quote_ctx.accrued_at_quote_date)
                / bond.notional.amount()
                * 100.0;
            bond.pricing_overrides =
                PricingOverrides::default().with_quoted_clean_price(clean_price_pct);
            Ok(Box::new(bond))
        }
        BondQuote::FixedRateBulletOas {
            id,
            currency,
            issue_date,
            maturity,
            coupon_rate,
            convention,
            oas,
        } => {
            let market = market.ok_or_else(|| {
                finstack_core::Error::Validation(
                    "OAS bond quote requires a MarketContext with a discount curve".to_string(),
                )
            })?;
            let mut bond = build_bond_shell(
                id.as_str(),
                *currency,
                *coupon_rate,
                *issue_date,
                *maturity,
                convention,
                ctx,
            )?;
            let quote_ctx = QuoteDateContext::new(&bond, market, ctx.as_of())?;
            let dirty_price_ccy =
                crate::instruments::fixed_income::bond::pricing::quote_conversions::price_from_oas(
                    &bond,
                    market,
                    quote_ctx.quote_date,
                    *oas,
                )?;
            if bond.notional.amount().abs() < crate::constants::numerical::ZERO_TOLERANCE {
                return Err(finstack_core::Error::Validation(
                    "bond notional must be non-zero for price conversion".to_string(),
                ));
            }
            let clean_price_pct = (dirty_price_ccy - quote_ctx.accrued_at_quote_date)
                / bond.notional.amount()
                * 100.0;
            bond.pricing_overrides =
                PricingOverrides::default().with_quoted_clean_price(clean_price_pct);
            Ok(Box::new(bond))
        }
    }
}

fn build_bond_shell(
    id: &str,
    currency: Currency,
    coupon_rate: f64,
    issue_date: finstack_core::dates::Date,
    maturity: finstack_core::dates::Date,
    convention: &crate::market::conventions::ids::BondConventionId,
    ctx: &BuildCtx,
) -> Result<Bond> {
    let registry = crate::market::conventions::ConventionRegistry::try_global()?;
    let convention_data = registry.require_bond(convention)?;
    if currency != convention_data.currency {
        return Err(finstack_core::Error::Validation(format!(
            "Bond quote currency {} does not match convention currency {}",
            currency, convention_data.currency
        )));
    }
    let discount_curve_id = ctx
        .curve_id("discount")
        .map(|s| s.to_string())
        .unwrap_or_else(|| convention_data.default_discount_curve_id.clone());

    Bond::with_convention(
        id,
        Money::new(ctx.notional(), currency),
        finstack_core::types::Rate::from_decimal(coupon_rate),
        issue_date,
        maturity,
        convention_data.market_convention,
        CurveId::new(discount_curve_id),
    )
}

fn dirty_price_from_ytm_with_frequency_ctx(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
) -> Result<f64> {
    let day_count = bond.cashflow_spec.day_count();
    let freq = bond.cashflow_spec.frequency();
    let mut pv = NeumaierAccumulator::new();

    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = day_count.year_fraction(
            as_of,
            date,
            DayCountContext {
                frequency: Some(freq),
                ..Default::default()
            },
        )?;
        if t > 0.0 {
            let df = df_from_yield(ytm, t, YieldCompounding::Street, freq)?;
            pv.add(amount.amount() * df);
        }
    }

    Ok(pv.total())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::market::conventions::ids::BondConventionId;
    use crate::market::quotes::ids::QuoteId;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;

    #[test]
    fn builder_defaults_discount_curve_from_bond_convention_when_ctx_role_missing() {
        let as_of = Date::from_calendar_date(2025, time::Month::January, 10).expect("valid date");
        let ctx = BuildCtx::new(as_of, 1_000_000.0, finstack_core::HashMap::default());

        let quote = BondQuote::FixedRateBulletCleanPrice {
            id: QuoteId::new("BOND-CORP-5Y"),
            currency: Currency::USD,
            issue_date: Date::from_calendar_date(2025, time::Month::January, 15).expect("issue"),
            maturity: Date::from_calendar_date(2030, time::Month::January, 15).expect("maturity"),
            coupon_rate: 0.05,
            convention: BondConventionId::new("USD-CORP"),
            clean_price_pct: 100.0,
        };

        let instrument = build_bond_instrument(&quote, &ctx, None).expect("build bond");
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .expect("expected bond");

        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
    }
}
