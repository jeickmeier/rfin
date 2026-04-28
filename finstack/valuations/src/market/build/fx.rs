//! Builders for FX instruments from market quotes.

use crate::instruments::fx::fx_forward::FxForward;
use crate::instruments::fx::fx_option::FxOption;
use crate::instruments::fx::fx_swap::FxSwap;
use crate::instruments::DynInstrument;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::fx::FxQuote;
use crate::market::quotes::ids::Pillar;
use crate::market::BuildCtx;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Build an FX instrument from an [`FxQuote`].
pub fn build_fx_instrument(quote: &FxQuote, ctx: &BuildCtx) -> Result<Box<DynInstrument>> {
    tracing::debug!(quote_id = %quote.id(), "building FX instrument");
    let registry = ConventionRegistry::try_global()?;

    match quote {
        FxQuote::ForwardOutright {
            id,
            convention,
            pillar,
            forward_rate,
        } => {
            let conv = registry.require_fx(convention)?;
            let domestic_discount_curve_id = ctx
                .curve_id("domestic_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.quote_currency));
            let foreign_discount_curve_id = ctx
                .curve_id("foreign_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.base_currency));

            let maturity = resolve_maturity(
                ctx.as_of(),
                pillar,
                conv.spot_lag_days,
                conv.business_day_convention,
                &conv.base_calendar_id,
                &conv.quote_calendar_id,
            )?;

            let forward = FxForward::builder()
                .id(InstrumentId::new(id.as_str()))
                .base_currency(conv.base_currency)
                .quote_currency(conv.quote_currency)
                .maturity(maturity)
                .notional(Money::new(ctx.notional(), conv.base_currency))
                .contract_rate_opt(Some(*forward_rate))
                .domestic_discount_curve_id(CurveId::new(domestic_discount_curve_id))
                .foreign_discount_curve_id(CurveId::new(foreign_discount_curve_id))
                .base_calendar_id_opt(Some(conv.base_calendar_id.clone()))
                .quote_calendar_id_opt(Some(conv.quote_calendar_id.clone()))
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(forward))
        }
        FxQuote::SwapOutright {
            id,
            convention,
            far_pillar,
            near_rate,
            far_rate,
        } => {
            let conv = registry.require_fx(convention)?;
            let domestic_discount_curve_id = ctx
                .curve_id("domestic_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.quote_currency));
            let foreign_discount_curve_id = ctx
                .curve_id("foreign_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.base_currency));

            let near_date = resolve_spot_date(
                ctx.as_of(),
                conv.spot_lag_days,
                conv.business_day_convention,
                &conv.base_calendar_id,
                &conv.quote_calendar_id,
            )?;
            let far_date = resolve_maturity_from_spot(
                near_date,
                far_pillar,
                conv.business_day_convention,
                &conv.base_calendar_id,
                &conv.quote_calendar_id,
            )?;

            let swap = FxSwap::builder()
                .id(InstrumentId::new(id.as_str()))
                .base_currency(conv.base_currency)
                .quote_currency(conv.quote_currency)
                .near_date(near_date)
                .far_date(far_date)
                .base_notional(Money::new(ctx.notional(), conv.base_currency))
                .domestic_discount_curve_id(CurveId::new(domestic_discount_curve_id))
                .foreign_discount_curve_id(CurveId::new(foreign_discount_curve_id))
                .near_rate_opt(Some(*near_rate))
                .far_rate_opt(Some(*far_rate))
                .base_calendar_id_opt(Some(conv.base_calendar_id.clone()))
                .quote_calendar_id_opt(Some(conv.quote_calendar_id.clone()))
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(swap))
        }
        FxQuote::OptionVanilla {
            id,
            convention,
            expiry,
            strike,
            option_type,
            vol_surface_id,
        } => {
            let option_conv = registry.require_fx_option(convention)?;
            let conv = registry.require_fx(&option_conv.fx_convention_id)?;
            let domestic_discount_curve_id = ctx
                .curve_id("domestic_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.quote_currency));
            let foreign_discount_curve_id = ctx
                .curve_id("foreign_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.base_currency));

            let option = FxOption::builder()
                .id(InstrumentId::new(id.as_str()))
                .base_currency(conv.base_currency)
                .quote_currency(conv.quote_currency)
                .strike(*strike)
                .option_type(*option_type)
                .exercise_style(option_conv.exercise_style)
                .expiry(*expiry)
                .day_count(option_conv.day_count)
                .notional(Money::new(ctx.notional(), conv.base_currency))
                .settlement(option_conv.settlement)
                .domestic_discount_curve_id(CurveId::new(domestic_discount_curve_id))
                .foreign_discount_curve_id(CurveId::new(foreign_discount_curve_id))
                .vol_surface_id(vol_surface_id.clone())
                .pricing_overrides(Default::default())
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(option))
        }
    }
}

fn resolve_spot_date(
    as_of: finstack_core::dates::Date,
    spot_lag_days: i32,
    bdc: BusinessDayConvention,
    base_calendar_id: &str,
    quote_calendar_id: &str,
) -> Result<finstack_core::dates::Date> {
    use crate::instruments::common_impl::fx_dates::roll_spot_date;

    roll_spot_date(
        as_of,
        spot_lag_days as u32,
        bdc,
        Some(base_calendar_id),
        Some(quote_calendar_id),
    )
}

fn resolve_maturity(
    as_of: finstack_core::dates::Date,
    pillar: &Pillar,
    spot_lag_days: i32,
    bdc: BusinessDayConvention,
    base_calendar_id: &str,
    quote_calendar_id: &str,
) -> Result<finstack_core::dates::Date> {
    let spot_date = resolve_spot_date(
        as_of,
        spot_lag_days,
        bdc,
        base_calendar_id,
        quote_calendar_id,
    )?;
    resolve_maturity_from_spot(spot_date, pillar, bdc, base_calendar_id, quote_calendar_id)
}

fn resolve_maturity_from_spot(
    spot_date: finstack_core::dates::Date,
    pillar: &Pillar,
    bdc: BusinessDayConvention,
    base_calendar_id: &str,
    quote_calendar_id: &str,
) -> Result<finstack_core::dates::Date> {
    use crate::instruments::common_impl::fx_dates::adjust_joint_calendar;

    match pillar {
        Pillar::Tenor(tenor) => {
            let raw = tenor.add_to_date(spot_date, None, BusinessDayConvention::Unadjusted)?;
            adjust_joint_calendar(raw, bdc, Some(base_calendar_id), Some(quote_calendar_id))
        }
        Pillar::Date(date) => {
            adjust_joint_calendar(*date, bdc, Some(base_calendar_id), Some(quote_calendar_id))
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/market/build/fx.rs"]
mod builder_integration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::conventions::ids::FxConventionId;
    use crate::market::quotes::ids::QuoteId;
    use finstack_core::dates::Tenor;
    use time::Month;

    #[test]
    fn builder_defaults_fx_discount_curves_from_pair_when_ctx_roles_missing() {
        let as_of = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 10)
            .expect("valid date");
        let ctx = BuildCtx::new(as_of, 1_000_000.0, finstack_core::HashMap::default());

        let quote = FxQuote::ForwardOutright {
            id: QuoteId::new("EURUSD-FWD-3M"),
            convention: FxConventionId::new("EUR/USD"),
            pillar: Pillar::Tenor(Tenor::parse("3M").expect("valid tenor")),
            forward_rate: 1.1050,
        };

        let instrument = build_fx_instrument(&quote, &ctx).expect("build fx forward");
        let forward = instrument
            .as_any()
            .downcast_ref::<FxForward>()
            .expect("expected fx forward");

        assert_eq!(forward.domestic_discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(forward.foreign_discount_curve_id.as_str(), "EUR-OIS");
    }
}
