//! Builders for credit instruments from market quotes.

use crate::instruments::cds::{CDSConvention, CreditDefaultSwap};
use crate::instruments::common::parameters::legs::{PayReceive, PremiumLegSpec, ProtectionLegSpec};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::instruments::PricingOverrides;
use crate::market::BuildCtx;
use crate::market::build::helpers::{resolve_calendar, resolve_spot_date};
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::Pillar;
use finstack_core::dates::{adjust, next_cds_date, DateExt, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::{Error, InputError, Result};
use rust_decimal::Decimal;

/// Build a Credit Default Swap instrument from a [`CdsQuote`].
///
/// This function resolves CDS conventions, calculates IMM roll dates, and constructs a CDS
/// instrument with premium and protection legs configured according to market standards.
/// Supports both par spread quotes and upfront + running spread quotes.
///
/// # Arguments
///
/// * `quote` - The CDS market quote (either par spread or upfront + running)
/// * `ctx` - Build context with valuation date, notional, and curve mappings
///
/// # Returns
///
/// `Ok(Box<dyn Instrument>)` with the constructed CDS instrument, or `Err` if:
/// - Convention lookup fails (missing CDS convention key)
/// - Calendar resolution fails
/// - Date calculations fail (invalid pillar, IMM roll date resolution)
/// - Instrument construction fails (invalid parameters)
///
/// # CDS Date Conventions
///
/// CDS instruments use IMM roll dates (20th of March, June, September, December) for
/// start dates. The start date is set to the IMM date on or before spot, and maturity
/// is adjusted to the next IMM date after the pillar date.
///
/// # Examples
///
/// Building from a par spread quote:
/// ```rust
/// use finstack_valuations::market::BuildCtx;
/// use finstack_valuations::market::build_cds_instrument;
/// use finstack_valuations::market::quotes::cds::CdsQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::dates::Date;
/// use finstack_core::currency::Currency;
/// use finstack_core::HashMap;
///
/// # fn example() -> finstack_core::Result<()> {
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     10_000_000.0,
///     HashMap::default(),
/// );
///
/// let quote = CdsQuote::CdsParSpread {
///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
///     entity: "ABC Corp".to_string(),
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
///     pillar: Pillar::Tenor("5Y".parse().unwrap()),
///     spread_bp: 150.0,
///     recovery_rate: 0.40,
/// };
///
/// let instrument = build_cds_instrument(&quote, &ctx)?;
/// # Ok(())
/// # }
/// ```
///
/// Building from an upfront quote:
/// ```rust
/// use finstack_valuations::market::BuildCtx;
/// use finstack_valuations::market::build_cds_instrument;
/// use finstack_valuations::market::quotes::cds::CdsQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::dates::Date;
/// use finstack_core::currency::Currency;
/// use finstack_core::HashMap;
///
/// # fn example() -> finstack_core::Result<()> {
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     10_000_000.0,
///     HashMap::default(),
/// );
///
/// let quote = CdsQuote::CdsUpfront {
///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
///     entity: "ABC Corp".to_string(),
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
///     pillar: Pillar::Tenor("5Y".parse().unwrap()),
///     running_spread_bp: 500.0,
///     upfront_pct: 0.02, // 2% upfront
///     recovery_rate: 0.40,
/// };
///
/// let instrument = build_cds_instrument(&quote, &ctx)?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`CdsQuote`](crate::market::quotes::cds::CdsQuote) for supported quote types
/// - [`BuildCtx`](crate::market::BuildCtx) for build context configuration
pub fn build_cds_instrument(quote: &CdsQuote, ctx: &BuildCtx) -> Result<Box<dyn Instrument>> {
    let registry = ConventionRegistry::try_global()?;
    let missing_role = |role: &str| {
        Error::Input(InputError::NotFound {
            id: format!("curve role '{}'", role),
        })
    };

    // Extract common fields
    let (id, convention_key, _entity, pillar, spread_bp, recovery_rate, upfront) = match quote {
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
    let spot = resolve_spot_date(
        ctx.as_of(),
        &conv.calendar_id,
        conv.settlement_days,
        conv.business_day_convention,
    )?;

    // Resolve calendar for tenor addition
    let cal = resolve_calendar(&conv.calendar_id)?;

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
        Pillar::Date(d) => {
            // Enforce IMM alignment even for explicit dates.
            let adjusted = adjust(*d, conv.business_day_convention, cal)?;
            next_cds_date(adjusted - time::Duration::days(1))
        }
    };

    let discount_id = ctx
        .curve_id("discount")
        .cloned()
        .ok_or_else(|| missing_role("discount"))?;

    // Credit curve ID: usually defaulted to entity name if not mapped
    let credit_id = ctx
        .curve_id("credit")
        .cloned()
        .ok_or_else(|| missing_role("credit"))?;

    // Calculate upfront amount if present
    // Amount = Notional * pct; Date = Spot (Settlement)
    let upfront_payment = upfront.map(|pct| {
        (
            spot,
            Money::new(ctx.notional() * pct, convention_key.currency),
        )
    });

    // We use Custom convention to avoid enum mismatch, but fully specify legs
    let convention_enum = CDSConvention::Custom;

    let cds = CreditDefaultSwap {
        id: InstrumentId::new(id.as_str()),
        notional: Money::new(ctx.notional(), convention_key.currency),
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
            spread_bp: Decimal::try_from(spread_bp).map_err(|e| {
                finstack_core::Error::Validation(format!(
                    "spread_bp {} cannot be represented as Decimal: {}",
                    spread_bp, e
                ))
            })?,
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

// Helpers moved to build::helpers
