//! Builders for CDS Tranche instruments from market quotes.

use crate::cashflow::builder::ScheduleParams;
use crate::instruments::cds_tranche::parameters::CDSTrancheParams;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use crate::instruments::common::traits::Instrument;
use crate::market::build::context::BuildCtx;
use crate::market::conventions::defs::CdsConventions;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds_tranche::CdsTrancheQuote;
use finstack_core::dates::{
    adjust, BusinessDayConvention, CalendarRegistry, Date, DateExt, DayCount, HolidayCalendar,
    StubKind, Tenor,
};
use finstack_core::error::Error;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Overrides for CDS tranche schedule and index metadata.
#[derive(Clone, Debug)]
pub struct CdsTrancheBuildOverrides {
    /// Index series number.
    pub series: u16,
    /// Optional payment frequency override.
    pub payment_frequency: Option<Tenor>,
    /// Optional day count override.
    pub day_count: Option<DayCount>,
    /// Optional business day convention override.
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Optional calendar identifier override.
    pub calendar_id: Option<String>,
    /// Whether to use standard IMM dates for the schedule.
    pub use_imm_dates: bool,
}

impl CdsTrancheBuildOverrides {
    /// Create overrides with only the series specified.
    pub fn new(series: u16) -> Self {
        Self {
            series,
            payment_frequency: None,
            day_count: None,
            business_day_convention: None,
            calendar_id: None,
            use_imm_dates: false,
        }
    }
}

/// Build an instrument from a CdsTrancheQuote.
pub fn build_cds_tranche_instrument(
    quote: &CdsTrancheQuote,
    ctx: &BuildCtx,
    overrides: &CdsTrancheBuildOverrides,
) -> Result<Box<dyn Instrument>> {
    let registry = ConventionRegistry::global();

    // Extract fields
    let (
        id,
        convention_key,
        index,
        attachment,
        detachment,
        maturity,
        running_spread_bp,
        upfront_pct,
    ) = match quote {
        CdsTrancheQuote::CDSTranche {
            id,
            index,
            attachment,
            detachment,
            maturity,
            running_spread_bp,
            upfront_pct,
            convention,
            ..
        } => (
            id,
            convention,
            index,
            *attachment,
            *detachment,
            *maturity,
            *running_spread_bp,
            *upfront_pct,
        ),
    };

    let conv = registry.require_cds(convention_key)?;
    let spot = resolve_spot_date(ctx.as_of, conv)?;

    // Resolve calendar for tenor addition
    let cal_registry = CalendarRegistry::global();
    let _cal = resolve_calendar(cal_registry, &conv.calendar_id)?;

    let discount_id = ctx
        .curve_id("discount")
        .cloned()
        .unwrap_or_else(|| convention_key.currency.to_string());

    // Index curve ID: usually defaulted to index name if not mapped
    let credit_id = ctx
        .curve_id("credit")
        .cloned()
        .unwrap_or_else(|| index.clone());

    let normalization_factor = detachment - attachment;
    if !normalization_factor.is_finite() || normalization_factor <= 0.0 {
        return Err(Error::Validation(format!(
            "Invalid tranche width: attachment={} detachment={}",
            attachment, detachment
        )));
    }
    let notional_amt = ctx.notional * normalization_factor;

    // `upfront_pct` is expressed in percentage points (e.g. -5.0 means -5% of tranche notional).
    let upfront_payment = (upfront_pct.abs() > 0.0).then(|| {
        (
            spot,
            Money::new(notional_amt * upfront_pct * 0.01, convention_key.currency),
        )
    });

    // Construct Params
    let tranche_params = CDSTrancheParams {
        index_name: index.clone(),
        series: overrides.series,
        attach_pct: attachment * 100.0, // Params expect percent
        detach_pct: detachment * 100.0, // Params expect percent
        notional: Money::new(notional_amt, convention_key.currency),
        maturity,
        running_coupon_bp: running_spread_bp,
        accumulated_loss: 0.0,
    };

    let schedule_params = ScheduleParams {
        freq: overrides.payment_frequency.unwrap_or(conv.payment_frequency),
        dc: overrides.day_count.unwrap_or(conv.day_count),
        bdc: overrides
            .business_day_convention
            .unwrap_or(conv.business_day_convention),
        calendar_id: overrides
            .calendar_id
            .clone()
            .or_else(|| Some(conv.calendar_id.clone())),
        stub: StubKind::ShortFront, // Default?
    };

    // Side: Quote usually implies we are observing market price.
    // If we build instrument to price it, we usually align with "Buy Protection" logic (pay premium).
    let side = TrancheSide::BuyProtection;

    let mut instrument = CdsTranche::new(
        InstrumentId::new(id.as_str()),
        &tranche_params,
        &schedule_params,
        CurveId::new(discount_id),
        CurveId::new(credit_id),
        side,
    );
    instrument.standard_imm_dates = overrides.use_imm_dates;
    instrument.upfront = upfront_payment;

    Ok(Box::new(instrument))
}

// Helpers

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
