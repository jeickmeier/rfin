//! Builders for credit instruments from market quotes.

use crate::instruments::cds::{CDSConvention, CreditDefaultSwap};
use crate::instruments::common::parameters::legs::{PayReceive, PremiumLegSpec, ProtectionLegSpec};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::instruments::PricingOverrides;
use crate::market::build::context::BuildCtx;
use crate::market::conventions::defs::CdsConventions;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::Pillar;
use finstack_core::dates::{
    adjust, next_cds_date, CalendarRegistry, Date, DateExt, HolidayCalendar, StubKind,
};
use finstack_core::error::Error;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Build an instrument from a CdsQuote.
pub fn build_cds_instrument(quote: &CdsQuote, ctx: &BuildCtx) -> Result<Box<dyn Instrument>> {
    let registry = ConventionRegistry::global();

    // Extract common fields
    let (id, convention_key, entity, pillar, spread_bp, recovery_rate, upfront) = match quote {
        CdsQuote::CdsParSpread {
            id,
            entity,
            convention,
            pillar,
            spread_bp,
            recovery_rate,
        } => (
            id,
            convention,
            entity,
            pillar,
            *spread_bp,
            *recovery_rate,
            None,
        ),
        CdsQuote::CdsUpfront {
            id,
            entity,
            convention,
            pillar,
            running_spread_bp,
            upfront_pct,
            recovery_rate,
        } => (
            id,
            convention,
            entity,
            pillar,
            *running_spread_bp,
            *recovery_rate,
            Some(*upfront_pct),
        ),
    };

    let conv = registry.require_cds(convention_key)?;
    let spot = resolve_spot_date(ctx.as_of, conv)?;

    // Resolve calendar for tenor addition
    let cal_registry = CalendarRegistry::global();
    let cal = resolve_calendar(cal_registry, &conv.calendar_id)?;

    // CDS Start: Market standard is the prior CDS roll (20th of Mar/Jun/Sep/Dec).
    // Use the CDS IMM roll date on or before spot.
    let roll_anchor = spot.add_months(-3);
    let start = next_cds_date(roll_anchor);

    let maturity = match pillar {
        Pillar::Tenor(t) => {
            // Maturity is the CDS roll date on or after the tenor-adjusted date.
            let raw = t.add_to_date(start, Some(cal), conv.business_day_convention)?;
            next_cds_date(raw - time::Duration::days(1))
        }
        Pillar::Date(d) => *d,
    };

    let discount_id = ctx
        .curve_id("discount")
        .cloned()
        .unwrap_or_else(|| convention_key.currency.to_string());

    // Credit curve ID: usually defaulted to entity name if not mapped
    let credit_id = ctx
        .curve_id("credit")
        .cloned()
        .unwrap_or_else(|| entity.clone());

    // Calculate upfront amount if present
    // Amount = Notional * pct; Date = Spot (Settlement)
    let upfront_payment = upfront.map(|pct| {
        (
            spot,
            Money::new(ctx.notional * pct, convention_key.currency),
        )
    });

    // We use Custom convention to avoid enum mismatch, but fully specify legs
    let convention_enum = CDSConvention::Custom;

    let cds = CreditDefaultSwap {
        id: InstrumentId::new(id.as_str()),
        notional: Money::new(ctx.notional, convention_key.currency),
        side: PayReceive::PayFixed, // Standard: Quote implies we buy protection (pay premium/spread) ? Or we are pricing the contract?
        // Usually "Par Spread" implies the spread we pay.
        // Default to Buy Protection (Pay Premium).
        convention: convention_enum,
        premium: PremiumLegSpec {
            start,
            end: maturity,
            freq: conv.payment_frequency,
            stub: StubKind::None, // Default to None or derive?
            bdc: conv.business_day_convention,
            calendar_id: Some(conv.calendar_id.clone()),
            dc: conv.day_count,
            spread_bp,
            discount_curve_id: CurveId::new(discount_id),
        },
        protection: ProtectionLegSpec {
            credit_curve_id: CurveId::new(credit_id),
            recovery_rate,
            settlement_delay: conv.settlement_days as u16,
        },
        pricing_overrides: PricingOverrides::default(),
        upfront: upfront_payment,
        margin_spec: None,
        attributes: Attributes::new(),
    };

    Ok(Box::new(cds))
}

// Helpers (duplicated from rates.rs for now, could be shared)

fn resolve_calendar<'a>(
    registry: &'a CalendarRegistry,
    id: &str,
) -> Result<&'a dyn HolidayCalendar> {
    registry
        .resolve_str(id)
        .ok_or_else(|| Error::calendar_not_found_with_suggestions(id, &[]))
}

fn resolve_spot_date(as_of: Date, conv: &CdsConventions) -> Result<Date> {
    let cal = CalendarRegistry::global().resolve_str(&conv.calendar_id);
    if let Some(c) = cal {
        let spot = as_of.add_business_days(conv.settlement_days, c)?;
        adjust(spot, conv.business_day_convention, c)
    } else {
        Err(Error::calendar_not_found_with_suggestions(
            &conv.calendar_id,
            &[],
        ))
    }
}
