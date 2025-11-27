//! Portfolio-level cashflow aggregation.
//!
//! This module provides utilities to build a **cashflow ladder** across all
//! positions in a portfolio. Cashflows are aggregated by payment date and
//! currency using holder-view schedules from the underlying instruments.
//!
//! The aggregation is **currency-preserving**: no implicit FX conversion is
//! applied. Consumers can apply explicit FX policies on top if a base-currency
//! ladder is required.

use crate::error::{PortfolioError, Result};
use crate::portfolio::Portfolio;
use crate::types::PositionId;
use finstack_core::prelude::*;
use finstack_valuations::cashflow::{traits::CashflowProvider, DatedFlow, DatedFlows};
use finstack_valuations::instruments::common::traits::Instrument;
use indexmap::IndexMap;

/// Aggregated portfolio cashflows by date and currency.
///
/// The `by_date` map preserves chronological ordering of payment dates and
/// aggregates per-currency amounts for each date across all positions.
#[derive(Clone, Debug)]
pub struct PortfolioCashflows {
    /// Map from payment date to per-currency totals.
    pub by_date: IndexMap<Date, IndexMap<Currency, Money>>,

    /// Optional per-position cashflow schedules for drill-down.
    ///
    /// This is keyed by position ID and contains holder-view cashflows in
    /// the instrument's native currency, scaled by position quantity.
    pub by_position: IndexMap<PositionId, DatedFlows>,
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
/// This helper mirrors the downcasting approach used by the theta calculator
/// in `finstack-valuations` to obtain cashflow schedules from instruments that
/// implement `CashflowProvider` (or equivalent methods).
fn instrument_holder_flows(
    instrument: &dyn Instrument,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Option<DatedFlows>> {
    use finstack_valuations::instruments::*;

    let any_ref = instrument.as_any();

    // Try to downcast to known CashflowProvider implementors.
    let flows: Option<DatedFlows> =
        // Bonds
        if let Some(bond) = any_ref.downcast_ref::<bond::Bond>() {
            bond.build_schedule(market, as_of).ok()
        }
        // Interest Rate Swaps
        else if let Some(irs) = any_ref.downcast_ref::<irs::InterestRateSwap>() {
            irs.build_schedule(market, as_of).ok()
        }
        // Deposits
        else if let Some(deposit) = any_ref.downcast_ref::<deposit::Deposit>() {
            deposit.build_schedule(market, as_of).ok()
        }
        // FRAs
        else if let Some(fra) = any_ref.downcast_ref::<fra::ForwardRateAgreement>() {
            fra.build_schedule(market, as_of).ok()
        }
        // IR Futures
        else if let Some(ir_fut) = any_ref.downcast_ref::<ir_future::InterestRateFuture>() {
            ir_fut.build_schedule(market, as_of).ok()
        }
        // Equities with discrete dividends
        else if let Some(eq) = any_ref.downcast_ref::<equity::Equity>() {
            eq.build_schedule(market, as_of).ok()
        }
        // FX Spot
        else if let Some(fx) = any_ref.downcast_ref::<fx_spot::FxSpot>() {
            fx.build_schedule(market, as_of).ok()
        }
        // Inflation-linked bonds
        else if let Some(ilb) =
            any_ref.downcast_ref::<inflation_linked_bond::InflationLinkedBond>()
        {
            ilb.build_schedule(market, as_of).ok()
        }
        // Repos
        else if let Some(repo) = any_ref.downcast_ref::<repo::Repo>() {
            repo.build_schedule(market, as_of).ok()
        }
        // Structured credit (pool/tranches)
        else if let Some(sc) = any_ref.downcast_ref::<structured_credit::StructuredCredit>() {
            sc.build_schedule(market, as_of).ok()
        }
        // Total return swaps
        else if let Some(eq_trs) = any_ref.downcast_ref::<trs::EquityTotalReturnSwap>() {
            eq_trs.build_schedule(market, as_of).ok()
        } else if let Some(fi_trs) = any_ref.downcast_ref::<trs::FIIndexTotalReturnSwap>() {
            fi_trs.build_schedule(market, as_of).ok()
        }
        // Private markets fund
        else if let Some(pmf) =
            any_ref.downcast_ref::<private_markets_fund::PrivateMarketsFund>()
        {
            pmf.build_schedule(market, as_of).ok()
        }
        // Variance swap
        else if let Some(var_swap) = any_ref.downcast_ref::<variance_swap::VarianceSwap>() {
            var_swap.build_schedule(market, as_of).ok()
        }
        // CDS – use premium schedule as holder-view cashflows
        else if let Some(cds) = any_ref.downcast_ref::<cds::CreditDefaultSwap>() {
            cds.build_premium_schedule(market, as_of).ok()
        }
        // FX Swap – settlements are handled at trade level; no interim schedule here
        else if any_ref.is::<fx_swap::FxSwap>() {
            None
        } else {
            // Instruments without a cashflow schedule interface (options, baskets, etc.)
            None
        };

    Ok(flows)
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
pub fn aggregate_cashflows(
    portfolio: &Portfolio,
    market: &MarketContext,
) -> Result<PortfolioCashflows> {
    let mut all_flows: Vec<DatedFlow> = Vec::new();
    let mut by_position: IndexMap<PositionId, DatedFlows> = IndexMap::new();

    // Phase 1: collect and scale flows per position
    for position in &portfolio.positions {
        if let Some(flows) =
            instrument_holder_flows(position.instrument.as_ref(), market, portfolio.as_of)?
        {
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
    }

    // Phase 2: aggregate by date and currency (sorted chronologically)
    all_flows.sort_by_key(|(d, _)| *d);

    let mut by_date: IndexMap<Date, IndexMap<Currency, Money>> = IndexMap::new();

    for (date, money) in all_flows {
        let per_ccy = by_date.entry(date).or_default();
        let ccy = money.currency();
        let entry = per_ccy.entry(ccy).or_insert_with(|| Money::new(0.0, ccy));
        *entry = entry.checked_add(money).map_err(PortfolioError::Core)?;
    }

    Ok(PortfolioCashflows {
        by_date,
        by_position,
    })
}

/// Collapse a multi-currency cashflow ladder into base currency by date.
///
/// This helper applies an explicit FX policy using the cashflow date as the
/// FX fixing date. It requires an `FxMatrix` in the market context.
pub fn collapse_cashflows_to_base_by_date(
    ladder: &PortfolioCashflows,
    market: &MarketContext,
    base_ccy: Currency,
) -> Result<IndexMap<Date, Money>> {
    let mut by_date_base: IndexMap<Date, Money> = IndexMap::new();

    for (date, per_ccy) in &ladder.by_date {
        let mut total = Money::new(0.0, base_ccy);

        for (ccy, money) in per_ccy {
            if *ccy == base_ccy {
                total = total.checked_add(*money).map_err(PortfolioError::Core)?;
            } else {
                let fx_matrix = market.fx.as_ref().ok_or_else(|| {
                    PortfolioError::MissingMarketData("FX matrix not available".to_string())
                })?;

                let query = FxQuery::new(*ccy, base_ccy, *date);
                let rate_result =
                    fx_matrix
                        .rate(query)
                        .map_err(|_| PortfolioError::FxConversionFailed {
                            from: *ccy,
                            to: base_ccy,
                        })?;

                let converted = Money::new(money.amount() * rate_result.rate, base_ccy);
                total = total.checked_add(converted).map_err(PortfolioError::Core)?;
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
/// base currency via [`collapse_cashflows_to_base_by_date`].
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
            *entry = entry.checked_add(amount).map_err(PortfolioError::Core)?;
        }
    }

    Ok(PortfolioCashflowBuckets { by_period })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::types::Entity;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::bond;
    use std::sync::Arc;
    use time::macros::date;

    fn build_test_market(as_of: Date) -> MarketContext {
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic()
            .build()
            .expect("test should succeed");

        MarketContext::new().insert_discount(curve)
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
        );

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

        let market = build_test_market(as_of);

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
        );

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

        let market = build_test_market(as_of);

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
}
