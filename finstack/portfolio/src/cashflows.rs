//! Portfolio-level cashflow aggregation.
//!
//! This module provides utilities to build a **cashflow ladder** across all
//! positions in a portfolio. Cashflows are aggregated by payment date and
//! currency using signed canonical schedules from the underlying instruments.
//!
//! The aggregation is **currency-preserving**: no implicit FX conversion is
//! applied. Consumers can apply explicit FX policies on top if a base-currency
//! ladder is required.
//!
//! # FX Conversion Warning
//!
//! The convenience functions
//! [`collapse_cashflows_to_base_by_date`](crate::cashflows::collapse_cashflows_to_base_by_date)
//! and [`cashflows_to_base_by_period`](crate::cashflows::cashflows_to_base_by_period)
//! convert using the spot-equivalent rate from
//! the [`FxMatrix`](finstack_core::money::fx::FxMatrix) for every cashflow date.
//! This is **not** the same as discounting future foreign-currency cashflows at
//! the appropriate forward FX rate. For NPV-grade accuracy, derive forward FX
//! rates from the relevant discount curves instead.

use crate::error::{Error, Result};
use crate::portfolio::Portfolio;
use crate::types::PositionId;
use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::{CashFlowSchedule, CashflowRepresentation};
use finstack_valuations::cashflow::{DatedFlow, DatedFlows};
use finstack_valuations::instruments::DynInstrument;
use indexmap::IndexMap;
use std::collections::HashSet;

fn market_reference_date(market: &MarketContext) -> Option<Date> {
    let state: finstack_core::market_data::context::MarketContextState = market.into();
    state.curves.iter().find_map(|curve| match curve {
        finstack_core::market_data::context::CurveState::Discount(curve) => Some(curve.base_date()),
        finstack_core::market_data::context::CurveState::Forward(curve) => Some(curve.base_date()),
        finstack_core::market_data::context::CurveState::Hazard(curve) => Some(curve.base_date()),
        finstack_core::market_data::context::CurveState::Inflation(curve) => {
            Some(curve.base_date())
        }
        finstack_core::market_data::context::CurveState::Price(curve) => Some(curve.base_date()),
        finstack_core::market_data::context::CurveState::VolIndex(curve) => Some(curve.base_date()),
        finstack_core::market_data::context::CurveState::BaseCorrelation(_) => None,
    })
}

fn add_years_clamped(date: Date, years: i32) -> Date {
    let target_year = date.year() + years;
    let month = date.month();
    let mut day = date.day();
    loop {
        if let Ok(result) = Date::from_calendar_date(target_year, month, day) {
            return result;
        }
        day -= 1;
    }
}

fn should_warn_far_future_fx_conversion(
    market: &MarketContext,
    payment_date: Date,
    from_ccy: Currency,
    base_ccy: Currency,
) -> bool {
    if from_ccy == base_ccy {
        return false;
    }
    let Some(reference_date) = market_reference_date(market) else {
        return false;
    };
    payment_date > add_years_clamped(reference_date, 30)
}

/// Aggregated portfolio cashflows by date and currency.
///
/// The `by_date` map preserves chronological ordering of payment dates and
/// aggregates per-currency amounts for each date across all positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CashflowWarning {
    /// Position whose cashflow extraction failed.
    pub position_id: PositionId,
    /// Underlying instrument identifier.
    pub instrument_id: String,
    /// Underlying instrument type key.
    pub instrument_type: String,
    /// Human-readable failure detail.
    pub message: String,
}

/// Why a position did not contribute classified cashflows to a portfolio ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CashflowExtractionIssueKind {
    /// The instrument does not expose `CashflowProvider`.
    Unsupported,
    /// The instrument exposes `CashflowProvider`, but schedule construction failed.
    BuildFailed,
}

/// Structured issue captured while extracting full cashflow schedules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CashflowExtractionIssue {
    /// Position whose cashflow extraction was attempted.
    pub position_id: PositionId,
    /// Underlying instrument identifier.
    pub instrument_id: String,
    /// Underlying instrument type key.
    pub instrument_type: String,
    /// Failure category.
    pub kind: CashflowExtractionIssueKind,
    /// Human-readable failure detail.
    pub message: String,
}

/// Per-position cashflow summary, including empty-schedule intent metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioCashflowPositionSummary {
    /// Position identifier.
    pub position_id: PositionId,
    /// Underlying instrument identifier.
    pub instrument_id: String,
    /// Underlying instrument type key.
    pub instrument_type: String,
    /// Schedule representation carried by the instrument.
    pub representation: CashflowRepresentation,
    /// Number of emitted dated events after schedule construction.
    pub event_count: usize,
}

/// One scaled portfolio cashflow event derived from an instrument schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioCashflowEvent {
    /// Position contributing the event.
    pub position_id: PositionId,
    /// Underlying instrument identifier.
    pub instrument_id: String,
    /// Underlying instrument type key.
    pub instrument_type: String,
    /// Payment date.
    pub date: Date,
    /// Position-scaled amount.
    pub amount: Money,
    /// Cashflow classification preserved from the instrument schedule.
    pub kind: CFKind,
    /// Optional reset date for floating coupons.
    pub reset_date: Option<Date>,
    /// Accrual factor used to compute the event when available.
    pub accrual_factor: f64,
    /// Effective rate used to compute the event when available.
    pub rate: Option<f64>,
}

/// Rich portfolio cashflow ladder preserving event classifications.
#[derive(Clone, Debug)]
pub struct PortfolioFullCashflows {
    /// Scaled cashflow events for all supported positions, sorted by payment date.
    pub events: Vec<PortfolioCashflowEvent>,

    /// Per-position event drill-down keyed by position ID.
    pub by_position: IndexMap<PositionId, Vec<PortfolioCashflowEvent>>,

    /// Aggregated totals by date, currency, and `CFKind`.
    pub by_date: IndexMap<Date, IndexMap<Currency, IndexMap<CFKind, Money>>>,

    /// Per-position schedule metadata, including placeholder/no-residual intent.
    pub position_summaries: IndexMap<PositionId, PortfolioCashflowPositionSummary>,

    /// Extraction issues for unsupported instruments and provider failures.
    pub issues: Vec<CashflowExtractionIssue>,
}

/// Aggregated portfolio cashflows by date and currency, plus extraction warnings.
#[derive(Clone, Debug)]
pub struct PortfolioCashflows {
    /// Map from payment date to per-currency totals.
    pub by_date: IndexMap<Date, IndexMap<Currency, Money>>,

    /// Optional per-position cashflow schedules for drill-down.
    ///
    /// This is keyed by position ID and contains instrument-economics-signed
    /// cashflows in the instrument's native currency, scaled by position quantity.
    pub by_position: IndexMap<PositionId, DatedFlows>,

    /// Per-position cashflow metadata, including empty-schedule visibility.
    pub position_summaries: IndexMap<PositionId, PortfolioCashflowPositionSummary>,

    /// Cashflow extraction warnings captured during aggregation.
    pub warnings: Vec<CashflowWarning>,
}

/// Aggregated portfolio cashflows by reporting period in base currency.
///
/// Each period total is expressed in a single reporting currency and is
/// suitable for liquidity analysis and cashflow ladder reporting.
#[derive(Clone, Debug)]
pub struct PortfolioCashflowBuckets {
    /// Map from period identifier to total cashflow in reporting currency.
    pub by_period: IndexMap<finstack_core::dates::PeriodId, Money>,
}

/// Aggregated base-currency cashflows bucketed by period and `CFKind`.
#[derive(Clone, Debug)]
pub struct PortfolioCashflowKindBuckets {
    /// Map from period identifier to base-currency totals per cashflow kind.
    pub by_period: IndexMap<finstack_core::dates::PeriodId, IndexMap<CFKind, Money>>,
}

/// Build the canonical signed schedule for a single instrument.
fn instrument_cashflow_schedule(
    instrument: &DynInstrument,
    market: &MarketContext,
    as_of: Date,
) -> std::result::Result<CashFlowSchedule, finstack_core::Error> {
    instrument.cashflow_schedule(market, as_of)
}

/// Aggregate full portfolio cashflows while preserving `CFKind` classification.
pub fn aggregate_full_cashflows(
    portfolio: &Portfolio,
    market: &MarketContext,
) -> Result<PortfolioFullCashflows> {
    let mut events = Vec::new();
    let mut by_position: IndexMap<PositionId, Vec<PortfolioCashflowEvent>> = IndexMap::new();
    let mut position_summaries: IndexMap<PositionId, PortfolioCashflowPositionSummary> =
        IndexMap::new();
    let mut issues = Vec::new();

    for position in &portfolio.positions {
        let instrument_id = position.instrument.id().to_string();
        let instrument_type = format!("{:?}", position.instrument.key());

        match instrument_cashflow_schedule(position.instrument.as_ref(), market, portfolio.as_of) {
            Ok(schedule) => {
                let event_count = schedule.flows.len();
                let representation = schedule.meta.representation;
                let mut position_events = Vec::with_capacity(schedule.flows.len());
                for flow in schedule.flows {
                    let scaled_amount = position.scale_value(flow.amount);
                    let event = PortfolioCashflowEvent {
                        position_id: position.position_id.clone(),
                        instrument_id: instrument_id.clone(),
                        instrument_type: instrument_type.clone(),
                        date: flow.date,
                        amount: scaled_amount,
                        kind: flow.kind,
                        reset_date: flow.reset_date,
                        accrual_factor: flow.accrual_factor,
                        rate: flow.rate,
                    };
                    events.push(event.clone());
                    position_events.push(event);
                }
                by_position.insert(position.position_id.clone(), position_events);
                position_summaries.insert(
                    position.position_id.clone(),
                    PortfolioCashflowPositionSummary {
                        position_id: position.position_id.clone(),
                        instrument_id: instrument_id.clone(),
                        instrument_type: instrument_type.clone(),
                        representation,
                        event_count,
                    },
                );
            }
            Err(err) => {
                tracing::warn!(
                    position_id = %position.position_id,
                    instrument_id = %position.instrument.id(),
                    instrument_type = ?position.instrument.key(),
                    error = %err,
                    "Skipping position during portfolio cashflow aggregation because contractual cashflows could not be built"
                );
                issues.push(CashflowExtractionIssue {
                    position_id: position.position_id.clone(),
                    instrument_id,
                    instrument_type,
                    kind: CashflowExtractionIssueKind::BuildFailed,
                    message: err.to_string(),
                });
            }
        }
    }

    events.sort_by_key(|event| event.date);

    let mut by_date: IndexMap<Date, IndexMap<Currency, IndexMap<CFKind, Money>>> = IndexMap::new();
    for event in &events {
        let per_ccy = by_date.entry(event.date).or_default();
        let per_kind = per_ccy.entry(event.amount.currency()).or_default();
        let entry = per_kind
            .entry(event.kind)
            .or_insert_with(|| Money::new(0.0, event.amount.currency()));
        *entry = entry.checked_add(event.amount).map_err(Error::Core)?;
    }

    Ok(PortfolioFullCashflows {
        events,
        by_position,
        by_date,
        position_summaries,
        issues,
    })
}

/// Aggregate portfolio cashflows by payment date and currency.
///
/// This function:
/// 1. Collects signed canonical schedule cashflows for each position (when supported)
/// 2. Scales flows by position quantity
/// 3. Aggregates by date and currency across the entire portfolio
///
/// # Arguments
///
/// * `portfolio` - Portfolio whose positions will be traversed
/// * `market` - Market context providing curves/indexes required for schedules
///
/// # Returns
///
/// [`Result`] containing [`PortfolioCashflows`] with both portfolio-level and
/// per-position views.
///
/// # References
///
/// - Bond and cashflow conventions:
///   `docs/REFERENCES.md#icma-rule-book`
pub fn aggregate_cashflows(
    portfolio: &Portfolio,
    market: &MarketContext,
) -> Result<PortfolioCashflows> {
    let PortfolioFullCashflows {
        events,
        by_position,
        position_summaries,
        issues,
        ..
    } = aggregate_full_cashflows(portfolio, market)?;

    let mut all_flows: Vec<DatedFlow> = events
        .iter()
        .map(|event| (event.date, event.amount))
        .collect();
    let by_position: IndexMap<PositionId, DatedFlows> = by_position
        .iter()
        .map(|(position_id, events)| {
            (
                position_id.clone(),
                events
                    .iter()
                    .map(|event| (event.date, event.amount))
                    .collect(),
            )
        })
        .collect();
    let warnings = issues
        .into_iter()
        .filter(|issue| issue.kind == CashflowExtractionIssueKind::BuildFailed)
        .map(|issue| CashflowWarning {
            position_id: issue.position_id,
            instrument_id: issue.instrument_id,
            instrument_type: issue.instrument_type,
            message: issue.message,
        })
        .collect();
    all_flows.sort_by_key(|(d, _)| *d);

    let mut by_date: IndexMap<Date, IndexMap<Currency, Money>> = IndexMap::new();

    for (date, money) in all_flows {
        let per_ccy = by_date.entry(date).or_default();
        let ccy = money.currency();
        let entry = per_ccy.entry(ccy).or_insert_with(|| Money::new(0.0, ccy));
        *entry = entry.checked_add(money).map_err(Error::Core)?;
    }

    Ok(PortfolioCashflows {
        by_date,
        by_position,
        position_summaries,
        warnings,
    })
}

fn convert_money_to_base_on_date(
    money: Money,
    payment_date: Date,
    market: &MarketContext,
    base_ccy: Currency,
    warned_pairs: &mut HashSet<(Currency, Currency, Date)>,
) -> Result<Money> {
    let ccy = money.currency();
    if ccy == base_ccy {
        return Ok(money);
    }

    let fx_matrix = market
        .fx()
        .ok_or_else(|| Error::MissingMarketData("FX matrix not available".to_string()))?;

    let query = FxQuery::new(ccy, base_ccy, payment_date);
    let rate_result = fx_matrix
        .rate(query)
        .map_err(|_| Error::FxConversionFailed {
            from: ccy,
            to: base_ccy,
        })?;

    if should_warn_far_future_fx_conversion(market, payment_date, ccy, base_ccy)
        && warned_pairs.insert((ccy, base_ccy, payment_date))
    {
        tracing::warn!(
            from = %ccy,
            to = %base_ccy,
            payment_date = %payment_date,
            "Converting cashflow beyond market as-of + 30Y using spot-equivalent FX; prefer forward FX for long-dated reporting"
        );
    }

    Ok(Money::new(money.amount() * rate_result.rate, base_ccy))
}

/// Collapse classified multi-currency cashflows into base currency by date and `CFKind`.
pub fn collapse_full_cashflows_to_base_by_date_kind(
    ladder: &PortfolioFullCashflows,
    market: &MarketContext,
    base_ccy: Currency,
) -> Result<IndexMap<Date, IndexMap<CFKind, Money>>> {
    let mut by_date_base: IndexMap<Date, IndexMap<CFKind, Money>> = IndexMap::new();
    let mut warned_pairs: HashSet<(Currency, Currency, Date)> = HashSet::new();

    for (date, per_ccy) in &ladder.by_date {
        let mut per_kind_base: IndexMap<CFKind, Money> = IndexMap::new();

        for per_kind in per_ccy.values() {
            for (kind, money) in per_kind {
                let converted = convert_money_to_base_on_date(
                    *money,
                    *date,
                    market,
                    base_ccy,
                    &mut warned_pairs,
                )?;
                let entry = per_kind_base
                    .entry(*kind)
                    .or_insert_with(|| Money::new(0.0, base_ccy));
                *entry = entry.checked_add(converted).map_err(Error::Core)?;
            }
        }

        if !per_kind_base.is_empty() {
            by_date_base.insert(*date, per_kind_base);
        }
    }

    Ok(by_date_base)
}

/// Collapse a multi-currency cashflow ladder into base currency by date.
///
/// This helper applies an explicit FX policy using the cashflow date as the
/// FX fixing date. It requires an `FxMatrix` in the market context.
///
/// # FX Conversion Note
///
/// The conversion uses spot-equivalent rates from
/// [`FxMatrix`](finstack_core::money::fx::FxMatrix) for **all**
/// cashflow dates, including future dates. In practice, the FX matrix typically
/// stores today's spot rate and may not account for the forward basis (interest
/// rate differential between currencies). For precise NPV-of-cashflows analysis
/// where the forward FX curve matters, convert future cashflows using forward
/// FX rates derived from the appropriate discount curves instead.
///
/// # Arguments
///
/// * `ladder` - Multi-currency cashflow ladder to convert.
/// * `market` - Market context providing FX rates.
/// * `base_ccy` - Reporting currency to convert into.
///
/// # Returns
///
/// Base-currency cashflow totals keyed by payment date.
///
/// # Errors
///
/// Returns an error when FX rates needed for conversion are unavailable.
pub fn collapse_cashflows_to_base_by_date(
    ladder: &PortfolioCashflows,
    market: &MarketContext,
    base_ccy: Currency,
) -> Result<IndexMap<Date, Money>> {
    let mut by_date_base: IndexMap<Date, Money> = IndexMap::new();
    let mut warned_pairs: HashSet<(Currency, Currency, Date)> = HashSet::new();

    for (date, per_ccy) in &ladder.by_date {
        let mut total = Money::new(0.0, base_ccy);

        for (ccy, money) in per_ccy {
            debug_assert_eq!(*ccy, money.currency());
            let converted =
                convert_money_to_base_on_date(*money, *date, market, base_ccy, &mut warned_pairs)?;
            total = total.checked_add(converted).map_err(Error::Core)?;
        }

        if !total.amount().is_nan() {
            by_date_base.insert(*date, total);
        }
    }

    Ok(by_date_base)
}

/// Bucket base-currency cashflows by reporting period.
///
/// This function assumes its input ladder has already been converted to
/// base currency via
/// [`collapse_cashflows_to_base_by_date`].
///
/// See that function's documentation for important notes on the use of
/// spot-equivalent FX rates for future cashflow conversion.
///
/// # Arguments
///
/// * `ladder` - Multi-currency cashflow ladder to bucket.
/// * `market` - Market context providing FX rates for conversion.
/// * `base_ccy` - Reporting currency for the bucket totals.
/// * `periods` - Reporting periods used to bucket converted cashflows.
///
/// # Returns
///
/// Base-currency cashflows bucketed by reporting period.
///
/// # Errors
///
/// Returns an error when FX conversion or monetary aggregation fails.
pub fn cashflows_to_base_by_period(
    ladder: &PortfolioCashflows,
    market: &MarketContext,
    base_ccy: Currency,
    periods: &[finstack_core::dates::Period],
) -> Result<PortfolioCashflowBuckets> {
    let by_date_base = collapse_cashflows_to_base_by_date(ladder, market, base_ccy)?;

    let mut by_period: IndexMap<finstack_core::dates::PeriodId, Money> = IndexMap::new();

    let mut unbucketed_count: usize = 0;
    let mut unbucketed_total = 0.0_f64;

    for (date, amount) in by_date_base {
        if let Some(period) = periods.iter().find(|p| date >= p.start && date <= p.end) {
            let entry = by_period
                .entry(period.id)
                .or_insert_with(|| Money::new(0.0, base_ccy));
            *entry = entry.checked_add(amount).map_err(Error::Core)?;
        } else {
            unbucketed_count += 1;
            unbucketed_total += amount.amount().abs();
        }
    }

    if unbucketed_count > 0 {
        tracing::warn!(
            count = unbucketed_count,
            total_abs = unbucketed_total,
            "cashflows_to_base_by_period: {unbucketed_count} cashflows fell outside all period boundaries and were dropped"
        );
    }

    Ok(PortfolioCashflowBuckets { by_period })
}

/// Bucket classified full cashflows by reporting period in base currency.
pub fn cashflows_to_base_by_period_kind(
    ladder: &PortfolioFullCashflows,
    market: &MarketContext,
    base_ccy: Currency,
    periods: &[finstack_core::dates::Period],
) -> Result<PortfolioCashflowKindBuckets> {
    let by_date_base = collapse_full_cashflows_to_base_by_date_kind(ladder, market, base_ccy)?;

    let mut by_period: IndexMap<finstack_core::dates::PeriodId, IndexMap<CFKind, Money>> =
        IndexMap::new();

    let mut unbucketed_count: usize = 0;
    let mut unbucketed_total = 0.0_f64;

    for (date, per_kind) in by_date_base {
        if let Some(period) = periods.iter().find(|p| date >= p.start && date <= p.end) {
            let period_entry = by_period.entry(period.id).or_default();
            for (kind, amount) in per_kind {
                let entry = period_entry
                    .entry(kind)
                    .or_insert_with(|| Money::new(0.0, base_ccy));
                *entry = entry.checked_add(amount).map_err(Error::Core)?;
            }
        } else {
            unbucketed_count += 1;
            unbucketed_total += per_kind.values().map(|m| m.amount().abs()).sum::<f64>();
        }
    }

    if unbucketed_count > 0 {
        tracing::warn!(
            count = unbucketed_count,
            total_abs = unbucketed_total,
            "cashflows_to_base_by_period_kind: {unbucketed_count} cashflow dates fell outside all period boundaries and were dropped"
        );
    }

    Ok(PortfolioCashflowKindBuckets { by_period })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market_at;
    use crate::types::Entity;
    use finstack_core::cashflow::CFKind;
    use finstack_core::market_data::term_structures::HazardCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_core::types::Attributes;
    use finstack_valuations::instruments::commodity::commodity_swap::CommoditySwap;
    use finstack_valuations::instruments::credit_derivatives::CDSIndex;
    use finstack_valuations::instruments::fixed_income::bond;
    use finstack_valuations::instruments::fixed_income::AgencyMbsPassthrough;
    use finstack_valuations::instruments::internal::InstrumentExt as InternalInstrument;
    use finstack_valuations::instruments::rates::Swaption;
    use finstack_valuations::pricer::InstrumentType;
    use std::any::Any;
    use std::sync::Arc;
    use std::sync::OnceLock;
    use time::macros::date;

    #[derive(Clone)]
    struct UnsupportedInstrument;

    impl finstack_valuations::cashflow::CashflowProvider for UnsupportedInstrument {
        fn cashflow_schedule(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<CashFlowSchedule> {
            Err(finstack_core::Error::Validation(
                "unsupported test instrument".to_string(),
            ))
        }
    }

    impl InternalInstrument for UnsupportedInstrument {
        fn id(&self) -> &str {
            "UNSUPPORTED"
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Swaption
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
            Ok(Money::new(0.0, Currency::USD))
        }

        fn attributes(&self) -> &Attributes {
            static ATTRS: OnceLock<Attributes> = OnceLock::new();
            ATTRS.get_or_init(Attributes::default)
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            unreachable!("test dummy should not mutate attributes")
        }

        fn clone_box(&self) -> Box<dyn InternalInstrument> {
            Box::new(self.clone())
        }
    }

    fn market_with_eurusd_fx(as_of: Date, eurusd: f64) -> MarketContext {
        let provider = Arc::new(SimpleFxProvider::new());
        provider
            .set_quote(Currency::EUR, Currency::USD, eurusd)
            .expect("test FX quote should be valid");
        build_test_market_at(as_of).insert_fx(FxMatrix::new(provider))
    }

    fn full_cashflow_ladder_fixture() -> PortfolioFullCashflows {
        let mut by_date: IndexMap<Date, IndexMap<Currency, IndexMap<CFKind, Money>>> =
            IndexMap::new();

        by_date.insert(
            date!(2025 - 03 - 15),
            IndexMap::from([
                (
                    Currency::EUR,
                    IndexMap::from([
                        (CFKind::Fixed, Money::new(100.0, Currency::EUR)),
                        (CFKind::Notional, Money::new(200.0, Currency::EUR)),
                    ]),
                ),
                (
                    Currency::USD,
                    IndexMap::from([(CFKind::Fee, Money::new(-10.0, Currency::USD))]),
                ),
            ]),
        );

        by_date.insert(
            date!(2025 - 08 - 01),
            IndexMap::from([
                (
                    Currency::USD,
                    IndexMap::from([(CFKind::Fixed, Money::new(50.0, Currency::USD))]),
                ),
                (
                    Currency::EUR,
                    IndexMap::from([(CFKind::Fee, Money::new(-5.0, Currency::EUR))]),
                ),
            ]),
        );

        by_date.insert(
            date!(2026 - 02 - 01),
            IndexMap::from([(
                Currency::EUR,
                IndexMap::from([(CFKind::Fixed, Money::new(25.0, Currency::EUR))]),
            )]),
        );

        PortfolioFullCashflows {
            events: Vec::new(),
            by_position: IndexMap::new(),
            by_date,
            position_summaries: IndexMap::new(),
            issues: Vec::new(),
        }
    }

    #[test]
    fn test_aggregate_cashflows_basic() {
        let as_of = date!(2025 - 01 - 01);

        // Simple fixed-rate bond with annual coupons
        let issue = as_of;
        let maturity = date!(2027 - 01 - 01);

        let bond = bond::Bond::fixed(
            "BOND_001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "BOND_001",
            Arc::new(bond),
            1.0,
            PositionUnit::FaceValue,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market_at(as_of);

        let ladder = aggregate_cashflows(&portfolio, &market).expect("cashflow aggregation");

        // There should be at least one payment date with USD flows
        assert!(
            !ladder.by_date.is_empty(),
            "Expected non-empty cashflow ladder"
        );

        let mut has_usd = false;
        for map in ladder.by_date.values() {
            if map.contains_key(&Currency::USD) {
                has_usd = true;
                break;
            }
        }
        assert!(has_usd, "Expected at least one USD cashflow");

        // Position-level drill-down should have exactly one entry
        assert_eq!(ladder.by_position.len(), 1);
        assert!(ladder.by_position.contains_key("POS_001"));
        assert_eq!(ladder.position_summaries.len(), 1);
        assert_eq!(
            ladder.position_summaries["POS_001"].representation,
            CashflowRepresentation::Contractual
        );
        assert!(
            ladder.warnings.is_empty(),
            "expected no aggregation warnings"
        );
    }

    #[test]
    fn test_cashflows_to_base_by_period() {
        let as_of = date!(2025 - 01 - 01);

        // Reuse the USD bond setup
        let issue = as_of;
        let maturity = date!(2027 - 01 - 01);

        let bond = bond::Bond::fixed(
            "BOND_001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "BOND_001",
            Arc::new(bond),
            1.0,
            PositionUnit::FaceValue,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market_at(as_of);

        let ladder = aggregate_cashflows(&portfolio, &market).expect("cashflow aggregation");

        // Define a simple annual period covering the bond horizon
        let periods = vec![finstack_core::dates::Period {
            id: finstack_core::dates::PeriodId::annual(2025),
            start: as_of,
            end: date!(2026 - 01 - 01),
            is_actual: true,
        }];

        let buckets = cashflows_to_base_by_period(&ladder, &market, Currency::USD, &periods)
            .expect("bucketed cashflows");

        // There should be at most one entry for the defined period
        assert!(buckets.by_period.len() <= 1);
        if let Some(total) = buckets
            .by_period
            .get(&finstack_core::dates::PeriodId::annual(2025))
        {
            // Total should be in USD
            assert_eq!(total.currency(), Currency::USD);
        }
    }

    #[test]
    fn far_future_fx_conversions_are_flagged_relative_to_market_reference_date() {
        let as_of = date!(2025 - 01 - 01);
        let market = build_test_market_at(as_of);
        let payment_date = date!(2055 - 01 - 02);

        assert!(should_warn_far_future_fx_conversion(
            &market,
            payment_date,
            Currency::EUR,
            Currency::USD
        ));
    }

    #[test]
    fn aggregate_cashflows_surfaces_provider_failures_as_warnings() {
        let as_of = date!(2025 - 01 - 01);
        let position = Position::new(
            "POS_SWAP",
            "ENTITY_A",
            "NG-SWAP-2025",
            Arc::new(CommoditySwap::example()),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("WARNINGS")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let ladder = aggregate_cashflows(&portfolio, &MarketContext::new())
            .expect("aggregation should succeed with warnings");

        assert!(
            ladder.by_date.is_empty(),
            "failed cashflows should be skipped"
        );
        assert!(
            ladder.by_position.is_empty(),
            "failed position should not emit flows"
        );
        assert_eq!(ladder.warnings.len(), 1, "expected one warning");
        assert_eq!(ladder.warnings[0].position_id.as_str(), "POS_SWAP");
        assert!(
            ladder.warnings[0].message.contains("NG-SPOT-AVG"),
            "unexpected warning message: {}",
            ladder.warnings[0].message
        );
    }

    #[test]
    fn aggregate_cashflows_preserves_empty_placeholder_position_summaries() {
        let as_of = date!(2025 - 01 - 01);
        let position = Position::new(
            "POS_SWAPTION",
            "ENTITY_A",
            "SWAPTION_001",
            Arc::new(Swaption::example()),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("PLACEHOLDER")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let ladder = aggregate_cashflows(&portfolio, &build_test_market_at(as_of))
            .expect("placeholder aggregation");

        assert!(
            ladder.by_date.is_empty(),
            "empty placeholder schedule has no events"
        );
        assert!(ladder.by_position["POS_SWAPTION"].is_empty());
        assert_eq!(
            ladder.position_summaries["POS_SWAPTION"].representation,
            CashflowRepresentation::Placeholder
        );
        assert_eq!(ladder.position_summaries["POS_SWAPTION"].event_count, 0);
        assert!(
            ladder.warnings.is_empty(),
            "placeholder schedules should not warn"
        );
    }

    #[test]
    fn collapsed_and_bucketed_views_project_from_same_canonical_event_set() {
        let as_of = date!(2025 - 01 - 01);
        let bond = bond::Bond::fixed(
            "BOND_001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2027 - 01 - 01),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");
        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "BOND_001",
            Arc::new(bond),
            1.0,
            PositionUnit::FaceValue,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");
        let market = build_test_market_at(as_of);
        let ladder = aggregate_cashflows(&portfolio, &market).expect("cashflow aggregation");

        let collapsed = collapse_cashflows_to_base_by_date(&ladder, &market, Currency::USD)
            .expect("collapse to base");
        let periods = vec![
            finstack_core::dates::Period {
                id: finstack_core::dates::PeriodId::annual(2025),
                start: date!(2025 - 01 - 01),
                end: date!(2026 - 01 - 01),
                is_actual: true,
            },
            finstack_core::dates::Period {
                id: finstack_core::dates::PeriodId::annual(2026),
                start: date!(2026 - 01 - 01),
                end: date!(2027 - 01 - 01),
                is_actual: true,
            },
            finstack_core::dates::Period {
                id: finstack_core::dates::PeriodId::annual(2027),
                start: date!(2027 - 01 - 01),
                end: date!(2028 - 01 - 01),
                is_actual: true,
            },
        ];
        let buckets = cashflows_to_base_by_period(&ladder, &market, Currency::USD, &periods)
            .expect("bucketed cashflows");

        let collapsed_total: f64 = collapsed.values().map(Money::amount).sum();
        let bucket_total: f64 = buckets.by_period.values().map(Money::amount).sum();
        assert_eq!(collapsed.len(), ladder.by_date.len());
        assert_eq!(bucket_total, collapsed_total);
    }

    #[test]
    fn aggregate_cashflows_includes_deferred_agency_provider() {
        let as_of = date!(2025 - 01 - 01);
        let position = Position::new(
            "POS_MBS",
            "ENTITY_A",
            "FN-MA1234",
            Arc::new(AgencyMbsPassthrough::example().expect("agency mbs example")),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("AGENCY")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let ladder = aggregate_cashflows(&portfolio, &build_test_market_at(as_of))
            .expect("agency cashflow aggregation");

        assert!(
            !ladder.by_date.is_empty(),
            "agency provider should emit flows"
        );
        assert!(
            ladder.warnings.is_empty(),
            "agency provider should not warn"
        );
    }

    #[test]
    fn aggregate_cashflows_includes_deferred_credit_composite_provider() {
        let as_of = date!(2025 - 01 - 01);
        let position = Position::new(
            "POS_CDX",
            "ENTITY_A",
            "CDX-IG-42",
            Arc::new(CDSIndex::example()),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("CDX")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");
        let market = build_test_market_at(as_of).insert(
            HazardCurve::builder("CDX.NA.IG.HAZARD")
                .base_date(as_of)
                .currency(Currency::USD)
                .recovery_rate(0.40)
                .knots([(0.0, 0.02), (5.0, 0.02)])
                .build()
                .expect("hazard curve should build"),
        );

        let ladder = aggregate_cashflows(&portfolio, &market).expect("cdx cashflow aggregation");

        assert!(
            !ladder.by_date.is_empty(),
            "credit composite provider should emit flows"
        );
        assert!(
            ladder.warnings.is_empty(),
            "credit composite provider should not warn"
        );
    }

    #[test]
    fn aggregate_full_cashflows_preserves_kinds_and_position_detail() {
        let as_of = date!(2025 - 01 - 01);
        let issue = as_of;
        let maturity = date!(2027 - 01 - 01);
        let bond = bond::Bond::fixed(
            "BOND_FULL",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");
        let position = Position::new(
            "POS_FULL",
            "ENTITY_A",
            "BOND_FULL",
            Arc::new(bond),
            1.0,
            PositionUnit::FaceValue,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("FULL")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let full = aggregate_full_cashflows(&portfolio, &build_test_market_at(as_of))
            .expect("full cashflow aggregation");

        assert!(
            !full.events.is_empty(),
            "expected classified cashflow events"
        );
        assert!(full.issues.is_empty(), "expected no extraction issues");
        assert_eq!(
            full.by_position.len(),
            1,
            "expected one position drill-down"
        );
        assert!(
            full.events
                .iter()
                .any(|event| matches!(event.kind, CFKind::Fixed | CFKind::Notional)),
            "expected coupon or principal classifications"
        );

        let has_kind_bucket = full.by_date.values().any(|per_ccy| {
            per_ccy.values().any(|per_kind| {
                per_kind.contains_key(&CFKind::Fixed) || per_kind.contains_key(&CFKind::Notional)
            })
        });
        assert!(
            has_kind_bucket,
            "expected date aggregation to preserve CFKind buckets"
        );
    }

    #[test]
    fn aggregate_full_cashflows_records_unsupported_instruments() {
        let as_of = date!(2025 - 01 - 01);
        let position = Position::new(
            "POS_UNSUPPORTED",
            "ENTITY_A",
            "UNSUPPORTED",
            Arc::new(UnsupportedInstrument),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("UNSUPPORTED")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let full = aggregate_full_cashflows(&portfolio, &build_test_market_at(as_of))
            .expect("unsupported instruments should produce issues, not fail the aggregation");

        assert!(
            full.events.is_empty(),
            "unsupported instrument should not emit events"
        );
        assert_eq!(full.issues.len(), 1, "expected one unsupported issue");
        assert_eq!(
            full.issues[0].kind,
            CashflowExtractionIssueKind::BuildFailed
        );
    }

    #[test]
    fn aggregate_cashflows_matches_full_cashflow_projection() {
        let as_of = date!(2025 - 01 - 01);
        let issue = as_of;
        let maturity = date!(2027 - 01 - 01);
        let bond = bond::Bond::fixed(
            "BOND_COMPARE",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");
        let position = Position::new(
            "POS_COMPARE",
            "ENTITY_A",
            "BOND_COMPARE",
            Arc::new(bond),
            1.0,
            PositionUnit::FaceValue,
        )
        .expect("test should succeed");
        let portfolio = PortfolioBuilder::new("COMPARE")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");
        let market = build_test_market_at(as_of);

        let ladder = aggregate_cashflows(&portfolio, &market).expect("dated ladder");
        let full = aggregate_full_cashflows(&portfolio, &market).expect("full ladder");

        let mut collapsed: IndexMap<Date, IndexMap<Currency, Money>> = IndexMap::new();
        for event in &full.events {
            let per_ccy = collapsed.entry(event.date).or_default();
            let entry = per_ccy
                .entry(event.amount.currency())
                .or_insert_with(|| Money::new(0.0, event.amount.currency()));
            *entry = entry
                .checked_add(event.amount)
                .expect("single-currency sum");
        }

        assert_eq!(
            ladder.by_date, collapsed,
            "dated ladder should derive from full events"
        );
    }

    #[test]
    fn collapse_full_cashflows_to_base_by_date_kind_preserves_cfkind() {
        let as_of = date!(2025 - 01 - 01);
        let full = full_cashflow_ladder_fixture();
        let market = market_with_eurusd_fx(as_of, 1.20);

        let by_date_kind =
            collapse_full_cashflows_to_base_by_date_kind(&full, &market, Currency::USD)
                .expect("base currency conversion by kind");

        let march = by_date_kind
            .get(&date!(2025 - 03 - 15))
            .expect("march bucket should exist");
        assert_eq!(march[&CFKind::Fixed], Money::new(120.0, Currency::USD));
        assert_eq!(march[&CFKind::Notional], Money::new(240.0, Currency::USD));
        assert_eq!(march[&CFKind::Fee], Money::new(-10.0, Currency::USD));

        let august = by_date_kind
            .get(&date!(2025 - 08 - 01))
            .expect("august bucket should exist");
        assert_eq!(august[&CFKind::Fixed], Money::new(50.0, Currency::USD));
        assert_eq!(august[&CFKind::Fee], Money::new(-6.0, Currency::USD));
    }

    #[test]
    fn cashflows_to_base_by_period_kind_aggregates_each_kind() {
        let as_of = date!(2025 - 01 - 01);
        let full = full_cashflow_ladder_fixture();
        let market = market_with_eurusd_fx(as_of, 1.20);
        let periods = vec![
            finstack_core::dates::Period {
                id: finstack_core::dates::PeriodId::annual(2025),
                start: as_of,
                end: date!(2026 - 01 - 01),
                is_actual: true,
            },
            finstack_core::dates::Period {
                id: finstack_core::dates::PeriodId::annual(2026),
                start: date!(2026 - 01 - 01),
                end: date!(2027 - 01 - 01),
                is_actual: true,
            },
        ];

        let buckets = cashflows_to_base_by_period_kind(&full, &market, Currency::USD, &periods)
            .expect("period bucketing by kind");

        let y2025 = buckets
            .by_period
            .get(&finstack_core::dates::PeriodId::annual(2025))
            .expect("2025 bucket should exist");
        assert_eq!(y2025[&CFKind::Fixed], Money::new(170.0, Currency::USD));
        assert_eq!(y2025[&CFKind::Notional], Money::new(240.0, Currency::USD));
        assert_eq!(y2025[&CFKind::Fee], Money::new(-16.0, Currency::USD));

        let y2026 = buckets
            .by_period
            .get(&finstack_core::dates::PeriodId::annual(2026))
            .expect("2026 bucket should exist");
        assert_eq!(y2026[&CFKind::Fixed], Money::new(30.0, Currency::USD));
    }
}
