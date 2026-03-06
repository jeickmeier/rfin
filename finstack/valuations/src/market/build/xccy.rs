//! Builders for cross-currency swap instruments from market quotes.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::xccy_swap::{LegSide, NotionalExchange, XccySwap, XccySwapLeg};
use crate::market::build::helpers::{resolve_calendar, resolve_spot_date};
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::ids::Pillar;
use crate::market::quotes::xccy::XccyQuote;
use crate::market::BuildCtx;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::Result;
use rust_decimal::Decimal;

/// Build a cross-currency swap instrument from an [`XccyQuote`].
pub fn build_xccy_instrument(quote: &XccyQuote, ctx: &BuildCtx) -> Result<Box<dyn Instrument>> {
    let registry = ConventionRegistry::try_global()?;

    match quote {
        XccyQuote::BasisSwap {
            id,
            convention,
            far_pillar,
            basis_spread_bp,
        } => {
            let conv = registry.require_xccy(convention)?;

            let domestic_discount = ctx
                .curve_id("domestic_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.quote_currency));
            let foreign_discount = ctx
                .curve_id("foreign_discount")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-OIS", conv.base_currency));
            let domestic_forward = ctx
                .curve_id("domestic_forward")
                .map(|s| s.to_string())
                .unwrap_or_else(|| conv.quote_index_id.to_string());
            let foreign_forward = ctx
                .curve_id("foreign_forward")
                .map(|s| s.to_string())
                .unwrap_or_else(|| conv.base_index_id.to_string());

            let spot = resolve_spot_date(
                ctx.as_of(),
                &conv.base_calendar_id,
                conv.spot_lag_days,
                conv.business_day_convention,
            )?;
            let far = resolve_far_date(
                spot,
                far_pillar,
                conv.business_day_convention,
                &conv.base_calendar_id,
            )?;

            let leg1 = XccySwapLeg {
                currency: conv.base_currency,
                notional: Money::new(ctx.notional(), conv.base_currency),
                side: LegSide::Receive,
                forward_curve_id: CurveId::new(foreign_forward),
                discount_curve_id: CurveId::new(foreign_discount),
                start: spot,
                end: far,
                frequency: conv.payment_frequency,
                day_count: conv.day_count,
                bdc: conv.business_day_convention,
                stub: finstack_core::dates::StubKind::ShortFront,
                spread_bp: Decimal::ZERO,
                payment_lag_days: 0,
                calendar_id: Some(conv.base_calendar_id.clone()),
                allow_calendar_fallback: false,
            };

            let leg2 = XccySwapLeg {
                currency: conv.quote_currency,
                notional: Money::new(ctx.notional(), conv.quote_currency),
                side: LegSide::Pay,
                forward_curve_id: CurveId::new(domestic_forward),
                discount_curve_id: CurveId::new(domestic_discount),
                start: spot,
                end: far,
                frequency: conv.payment_frequency,
                day_count: conv.day_count,
                bdc: conv.business_day_convention,
                stub: finstack_core::dates::StubKind::ShortFront,
                spread_bp: Decimal::from_f64_retain(*basis_spread_bp).unwrap_or_default(),
                payment_lag_days: 0,
                calendar_id: Some(conv.quote_calendar_id.clone()),
                allow_calendar_fallback: false,
            };

            let swap = XccySwap::new(id.as_str(), leg1, leg2, conv.quote_currency)
                .with_notional_exchange(NotionalExchange::InitialAndFinal);

            Ok(Box::new(swap))
        }
    }
}

fn resolve_far_date(
    spot: finstack_core::dates::Date,
    pillar: &Pillar,
    bdc: BusinessDayConvention,
    calendar_id: &str,
) -> Result<finstack_core::dates::Date> {
    let cal = resolve_calendar(calendar_id)?;
    match pillar {
        Pillar::Tenor(tenor) => {
            let raw = tenor.add_to_date(spot, None, BusinessDayConvention::Unadjusted)?;
            finstack_core::dates::adjust(raw, bdc, cal)
        }
        Pillar::Date(date) => finstack_core::dates::adjust(*date, bdc, cal),
    }
}
