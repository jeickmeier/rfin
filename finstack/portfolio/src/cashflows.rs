//! Portfolio-level cashflow aggregation.
//!
//! This module provides utilities to build a **cashflow ladder** across all
//! positions in a portfolio. Cashflows are aggregated by payment date and
//! currency using holder-view schedules from the underlying instruments.
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
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
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

/// Aggregated portfolio cashflows by date and currency, plus extraction warnings.
#[derive(Clone, Debug)]
pub struct PortfolioCashflows {
    /// Map from payment date to per-currency totals.
    pub by_date: IndexMap<Date, IndexMap<Currency, Money>>,

    /// Optional per-position cashflow schedules for drill-down.
    ///
    /// This is keyed by position ID and contains holder-view cashflows in
    /// the instrument's native currency, scaled by position quantity.
    pub by_position: IndexMap<PositionId, DatedFlows>,

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

/// Collect holder-view cashflows for a single instrument, if supported.
///
/// Uses the `Instrument::as_cashflow_provider()` trait method to obtain cashflow
/// schedules from instruments that implement `CashflowProvider`. This approach
/// automatically supports new instruments as they implement the trait.
fn instrument_holder_flows(
    instrument: &DynInstrument,
    market: &MarketContext,
    as_of: Date,
) -> std::result::Result<Option<DatedFlows>, finstack_core::Error> {
    if let Some(provider) = instrument.as_cashflow_provider() {
        return provider.build_dated_flows(market, as_of).map(Some);
    }

    // Instruments without a cashflow schedule interface (options, baskets, etc.)
    Ok(None)
}

/// Aggregate portfolio cashflows by payment date and currency.
///
/// This function:
/// 1. Collects holder-view cashflows for each position (when supported)
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
    let mut all_flows: Vec<DatedFlow> = Vec::new();
    let mut by_position: IndexMap<PositionId, DatedFlows> = IndexMap::new();
    let mut warnings = Vec::new();

    // Phase 1: collect and scale flows per position
    for position in &portfolio.positions {
        match instrument_holder_flows(position.instrument.as_ref(), market, portfolio.as_of) {
            Ok(Some(flows)) => {
                let mut scaled: DatedFlows = Vec::with_capacity(flows.len());

                for (date, money) in flows {
                    let scaled_money = position.scale_value(money);
                    all_flows.push((date, scaled_money));
                    scaled.push((date, scaled_money));
                }

                if !scaled.is_empty() {
                    by_position.insert(position.position_id.clone(), scaled);
                }
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(
                    position_id = %position.position_id,
                    instrument_id = %position.instrument.id(),
                    instrument_type = ?position.instrument.key(),
                    error = %err,
                    "Skipping position during portfolio cashflow aggregation because contractual cashflows could not be built"
                );
                warnings.push(CashflowWarning {
                    position_id: position.position_id.clone(),
                    instrument_id: position.instrument.id().to_string(),
                    instrument_type: format!("{:?}", position.instrument.key()),
                    message: err.to_string(),
                });
            }
        }
    }

    // Phase 2: aggregate by date and currency (sorted chronologically)
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
        warnings,
    })
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
            if *ccy == base_ccy {
                total = total.checked_add(*money).map_err(Error::Core)?;
            } else {
                let fx_matrix = market.fx().ok_or_else(|| {
                    Error::MissingMarketData("FX matrix not available".to_string())
                })?;

                let query = FxQuery::new(*ccy, base_ccy, *date);
                let rate_result = fx_matrix
                    .rate(query)
                    .map_err(|_| Error::FxConversionFailed {
                        from: *ccy,
                        to: base_ccy,
                    })?;

                if should_warn_far_future_fx_conversion(market, *date, *ccy, base_ccy)
                    && warned_pairs.insert((*ccy, base_ccy, *date))
                {
                    tracing::warn!(
                        from = %ccy,
                        to = %base_ccy,
                        payment_date = %date,
                        "Converting cashflow beyond market as-of + 30Y using spot-equivalent FX; prefer forward FX for long-dated reporting"
                    );
                }

                let converted = Money::new(money.amount() * rate_result.rate, base_ccy);
                total = total.checked_add(converted).map_err(Error::Core)?;
            }
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

    for (date, amount) in by_date_base {
        // Find the first period containing this date: [start, end]
        if let Some(period) = periods.iter().find(|p| date >= p.start && date <= p.end) {
            let entry = by_period
                .entry(period.id)
                .or_insert_with(|| Money::new(0.0, base_ccy));
            *entry = entry.checked_add(amount).map_err(Error::Core)?;
        }
    }

    Ok(PortfolioCashflowBuckets { by_period })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market_at;
    use crate::types::Entity;
    use finstack_core::market_data::term_structures::HazardCurve;
    use finstack_valuations::instruments::credit_derivatives::CDSIndex;
    use finstack_valuations::instruments::fixed_income::AgencyMbsPassthrough;
    use finstack_valuations::instruments::fixed_income::bond;
    use finstack_valuations::instruments::fx::ndf::Ndf;
    use std::sync::Arc;
    use time::macros::date;

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
        assert!(ladder.warnings.is_empty(), "expected no aggregation warnings");
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
            "POS_NDF",
            "ENTITY_A",
            "USDCNY-NDF-3M",
            Arc::new(Ndf::example()),
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

        assert!(ladder.by_date.is_empty(), "failed cashflows should be skipped");
        assert!(ladder.by_position.is_empty(), "failed position should not emit flows");
        assert_eq!(ladder.warnings.len(), 1, "expected one warning");
        assert_eq!(ladder.warnings[0].position_id.as_str(), "POS_NDF");
        assert!(
            ladder.warnings[0]
                .message
                .contains("cannot build a contractual cashflow schedule before fixing"),
            "unexpected warning message: {}",
            ladder.warnings[0].message
        );
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

        assert!(!ladder.by_date.is_empty(), "agency provider should emit flows");
        assert!(ladder.warnings.is_empty(), "agency provider should not warn");
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

        assert!(!ladder.by_date.is_empty(), "credit composite provider should emit flows");
        assert!(ladder.warnings.is_empty(), "credit composite provider should not warn");
    }
}
