//! Builders for CDS Tranche instruments from market quotes.

use crate::cashflow::builder::ScheduleParams;
use crate::instruments::cds_tranche::parameters::CDSTrancheParams;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use crate::instruments::common::traits::Instrument;
use crate::market::build::context::BuildCtx;
use crate::market::build::helpers::{resolve_calendar, resolve_spot_date};
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds_tranche::CdsTrancheQuote;
use finstack_core::dates::{
    adjust, next_cds_date, BusinessDayConvention, DateExt, DayCount, StubKind, Tenor,
};
use finstack_core::error::Error;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Overrides for CDS tranche schedule and index metadata.
///
/// Allows customization of schedule parameters and index series when building CDS tranche
/// instruments from quotes. Fields default to convention values if not specified.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::build::cds_tranche::CdsTrancheBuildOverrides;
///
/// // Use default overrides with only series specified
/// let overrides = CdsTrancheBuildOverrides::new(42);
///
/// // Customize payment frequency and day count
/// let mut overrides = CdsTrancheBuildOverrides::new(42);
/// overrides.payment_frequency = Some("3M".parse().unwrap());
/// overrides.day_count = Some(finstack_core::dates::DayCount::Act360);
/// ```
#[derive(Clone, Debug)]
pub struct CdsTrancheBuildOverrides {
    /// Index series number.
    ///
    /// The series number identifies which version of the CDS index this tranche references
    /// (e.g., CDX.NA.IG Series 42).
    pub series: u16,
    /// Optional payment frequency override.
    ///
    /// If `None`, uses the payment frequency from the CDS convention.
    pub payment_frequency: Option<Tenor>,
    /// Optional day count override.
    ///
    /// If `None`, uses the day count from the CDS convention.
    pub day_count: Option<DayCount>,
    /// Optional business day convention override.
    ///
    /// If `None`, uses the business day convention from the CDS convention.
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Optional calendar identifier override.
    ///
    /// If `None`, uses the calendar ID from the CDS convention.
    pub calendar_id: Option<String>,
    /// Whether to use standard IMM dates for the schedule.
    ///
    /// When `true`, payment dates are aligned to IMM dates (20th of Mar/Jun/Sep/Dec).
    /// When `false`, payment dates follow the standard schedule calculation.
    pub use_imm_dates: bool,
}

impl CdsTrancheBuildOverrides {
    /// Create overrides with only the series specified.
    ///
    /// All other fields default to `None` or `false`, meaning convention values will be used.
    ///
    /// # Arguments
    ///
    /// * `series` - The CDS index series number
    ///
    /// # Returns
    ///
    /// A new `CdsTrancheBuildOverrides` with default values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::build::cds_tranche::CdsTrancheBuildOverrides;
    ///
    /// let overrides = CdsTrancheBuildOverrides::new(42);
    /// assert_eq!(overrides.series, 42);
    /// assert_eq!(overrides.payment_frequency, None);
    /// ```
    pub fn new(series: u16) -> Self {
        Self {
            series,
            payment_frequency: None,
            day_count: None,
            business_day_convention: None,
            calendar_id: None,
            use_imm_dates: true,
        }
    }
}

/// Build a CDS Tranche instrument from a [`CdsTrancheQuote`].
///
/// This function resolves CDS conventions, calculates tranche notional based on attachment
/// and detachment points, and constructs a CDS tranche instrument with upfront and running
/// spread payments.
///
/// # Arguments
///
/// * `quote` - The CDS tranche market quote with attachment/detachment and pricing
/// * `ctx` - Build context with valuation date, notional, and curve mappings
/// * `overrides` - Overrides for schedule parameters and index series
///
/// # Returns
///
/// `Ok(Box<dyn Instrument>)` with the constructed CDS tranche instrument, or `Err` if:
/// - Convention lookup fails (missing CDS convention key)
/// - Calendar resolution fails
/// - Invalid tranche width (detachment <= attachment or non-finite values)
/// - Instrument construction fails (invalid parameters)
///
/// # Tranche Notional Calculation
///
/// The tranche notional is calculated as:
/// ```text
/// tranche_notional = base_notional * (detachment - attachment)
/// ```
///
/// The upfront payment (if present) is calculated as:
/// ```text
/// upfront_amount = tranche_notional * upfront_pct * 0.01
/// ```
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::build::context::BuildCtx;
/// use finstack_valuations::market::build::cds_tranche::{build_cds_tranche_instrument, CdsTrancheBuildOverrides};
/// use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
/// use finstack_valuations::market::quotes::ids::QuoteId;
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::dates::Date;
/// use finstack_core::currency::Currency;
/// use finstack_core::collections::HashMap;
///
/// # fn example() -> finstack_core::Result<()> {
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     100_000_000.0, // Base notional
///     HashMap::default(),
/// );
///
/// let quote = CdsTrancheQuote::CDSTranche {
///     id: QuoteId::new("CDX-IG-3-7"),
///     index: "CDX.NA.IG".to_string(),
///     attachment: 0.03,  // 3%
///     detachment: 0.07,   // 7%
///     maturity: Date::from_calendar_date(2029, time::Month::June, 20).unwrap(),
///     upfront_pct: -2.5, // -2.5% upfront
///     running_spread_bp: 500.0,
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
/// };
///
/// let overrides = CdsTrancheBuildOverrides::new(42);
/// let instrument = build_cds_tranche_instrument(&quote, &ctx, &overrides)?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`CdsTrancheQuote`](crate::market::quotes::cds_tranche::CdsTrancheQuote) for quote structure
/// - [`BuildCtx`](crate::market::build::context::BuildCtx) for build context configuration
/// - [`CdsTrancheBuildOverrides`](CdsTrancheBuildOverrides) for override options
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
    let spot = resolve_spot_date(
        ctx.as_of,
        &conv.calendar_id,
        conv.settlement_days,
        conv.business_day_convention,
    )?;

    // Resolve calendar for tenor addition
    let cal = resolve_calendar(&conv.calendar_id)?;

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

    let (effective_date, maturity_date, standard_imm_dates) = if overrides.use_imm_dates {
        // CDS-style effective date (prior IMM) and IMM-aligned maturity.
        let roll_anchor = spot.add_months(-3);
        let effective_date = next_cds_date(roll_anchor);
        let maturity_imm = {
            let adjusted = adjust(maturity, conv.business_day_convention, cal)?;
            next_cds_date(adjusted - time::Duration::days(1))
        };
        (effective_date, maturity_imm, true)
    } else {
        let maturity_adj = adjust(maturity, conv.business_day_convention, cal)?;
        (spot, maturity_adj, false)
    };

    // Construct Params
    let tranche_params = CDSTrancheParams {
        index_name: index.clone(),
        series: overrides.series,
        attach_pct: attachment * 100.0, // Params expect percent
        detach_pct: detachment * 100.0, // Params expect percent
        notional: Money::new(notional_amt, convention_key.currency),
        maturity: maturity_date,
        running_coupon_bp: running_spread_bp,
        accumulated_loss: 0.0,
    };

    let schedule_params = ScheduleParams {
        freq: overrides
            .payment_frequency
            .unwrap_or(conv.payment_frequency),
        dc: overrides.day_count.unwrap_or(conv.day_count),
        bdc: overrides
            .business_day_convention
            .unwrap_or(conv.business_day_convention),
        calendar_id: overrides
            .calendar_id
            .clone()
            .or_else(|| Some(conv.calendar_id.clone())),
        stub: StubKind::ShortFront,
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
    instrument.standard_imm_dates = standard_imm_dates;
    instrument.effective_date = Some(effective_date);
    instrument.upfront = upfront_payment;

    Ok(Box::new(instrument))
}

// Helpers moved to build::helpers

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    use crate::market::quotes::cds_tranche::CdsTrancheQuote;
    use crate::market::quotes::ids::QuoteId;
    use finstack_core::collections::HashMap;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{adjust, Date};
    use time::Month;

    #[test]
    fn build_non_imm_tranche_allows_custom_schedule() {
        let as_of = Date::from_calendar_date(2024, Month::January, 2).expect("valid date");
        let ctx = BuildCtx::new(as_of, 1_000_000.0, HashMap::default());

        let convention_key = CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::IsdaNa,
        };
        let maturity = Date::from_calendar_date(2029, Month::January, 15).expect("valid date");

        let quote = CdsTrancheQuote::CDSTranche {
            id: QuoteId::new("CDX-IG-3-7"),
            index: "CDX.NA.IG".to_string(),
            attachment: 0.03,
            detachment: 0.07,
            maturity,
            upfront_pct: -2.5,
            running_spread_bp: 500.0,
            convention: convention_key.clone(),
        };

        let mut overrides = CdsTrancheBuildOverrides::new(42);
        overrides.use_imm_dates = false;

        let instrument = build_cds_tranche_instrument(&quote, &ctx, &overrides)
            .expect("non-IMM tranche build should succeed");
        let tranche = instrument
            .as_any()
            .downcast_ref::<CdsTranche>()
            .expect("should be CdsTranche");

        assert!(!tranche.standard_imm_dates);

        let conv = ConventionRegistry::global()
            .require_cds(&convention_key)
            .expect("convention should exist");
        let spot = resolve_spot_date(
            as_of,
            &conv.calendar_id,
            conv.settlement_days,
            conv.business_day_convention,
        )
        .expect("spot date");
        let cal = resolve_calendar(&conv.calendar_id).expect("calendar");
        let maturity_adj =
            adjust(maturity, conv.business_day_convention, cal).expect("maturity adjustment");

        assert_eq!(tranche.effective_date, Some(spot));
        assert_eq!(tranche.maturity, maturity_adj);
    }
}
