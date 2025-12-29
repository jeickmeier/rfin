//! Comprehensive portfolio example demonstrating:
//! - Entity-based and standalone instrument positions
//! - Portfolio valuation and aggregation
//! - Cross-currency FX conversion
//! - Attribute-based grouping
//! - Scenario application and re-valuation
//! - DataFrame exports

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::money::Money;
use finstack_portfolio::{self, *};
use finstack_scenarios::spec::{CurveKind, OperationSpec, ScenarioSpec};
use finstack_valuations::constants::isda;
use finstack_valuations::instruments;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::cap_floor::{parameters::*, InterestRateOption};
use finstack_valuations::instruments::cds::*;
use finstack_valuations::instruments::cds_index::{parameters::*, CDSIndex};
use finstack_valuations::instruments::cds_option::{parameters::*, CdsOption};
use finstack_valuations::instruments::cds_tranche::{parameters::*, CdsTranche, TrancheSide};
use finstack_valuations::instruments::common::parameters::legs::*;
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::instruments::equity::Equity;
use finstack_valuations::instruments::equity_option::{parameters::*, EquityOption};
use finstack_valuations::instruments::fx_option::{parameters::*, FxOption};
use finstack_valuations::instruments::fx_spot::FxSpot;
use finstack_valuations::instruments::fx_swap::FxSwap;
use finstack_valuations::instruments::inflation_linked_bond::{parameters::*, InflationLinkedBond};
use finstack_valuations::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_valuations::instruments::irs::*;
use finstack_valuations::instruments::structured_credit::StructuredCredit;
use finstack_valuations::instruments::structured_credit::{
    DealType, Pool, Seniority, TrancheBuilder, TrancheCoupon, TrancheStructure,
};
use finstack_valuations::instruments::swaption::{parameters::*, Swaption};
use rust_decimal_macros::dec;
use std::sync::Arc;
use time::macros::date;

fn main() -> finstack_portfolio::Result<()> {
    println!("=== Finstack Portfolio Example ===\n");

    // 1. Create market data
    let as_of = date!(2024 - 01 - 01);
    let market = build_market_data(as_of);
    let config = FinstackConfig::default();

    // 2. Build portfolio with mixed instruments
    let portfolio = build_sample_portfolio(as_of)?;
    println!("Created portfolio '{}' with:", portfolio.id);
    println!("  - {} entities", portfolio.entities.len());
    println!("  - {} positions", portfolio.positions.len());
    println!("  - Base currency: {}\n", portfolio.base_ccy);

    // 3. Value the portfolio
    println!("--- Portfolio Valuation ---");
    let valuation = value_portfolio(&portfolio, &market, &config)?;
    println!("Total value: {}", valuation.total_base_ccy);
    println!("\nBy entity:");
    for (entity_id, total) in &valuation.by_entity {
        println!("  {}: {}", entity_id, total);
    }

    // 4. Aggregate metrics
    println!("\n--- Metrics Aggregation ---");
    let metrics = aggregate_metrics(&valuation)?;
    println!("Aggregated {} metrics", metrics.aggregated.len());
    println!(
        "Position-level metrics: {} positions",
        metrics.by_position.len()
    );

    // Show aggregated metrics
    if !metrics.aggregated.is_empty() {
        println!("\nAggregated metrics:");
        for (metric_id, agg_metric) in &metrics.aggregated {
            println!("  {}: {:.2}", metric_id, agg_metric.total);
            println!("    By entity:");
            for (entity_id, value) in &agg_metric.by_entity {
                println!("      {}: {:.2}", entity_id, value);
            }
        }
    }

    // Show position-level metrics
    if !metrics.by_position.is_empty() {
        println!("\nPosition-level metrics:");
        for (position_id, position_metrics) in metrics.by_position.iter().take(25) {
            println!("  {}:", position_id);
            for (metric_id, value) in position_metrics {
                println!("    {}: {:.4}", metric_id, value);
            }
        }
        if metrics.by_position.len() > 25 {
            println!(
                "  ... and {} more positions",
                metrics.by_position.len() - 25
            );
        }
    }

    // 5. Group by attributes
    println!("\n--- Attribute-Based Grouping ---");
    let by_rating = aggregate_by_attribute(
        &valuation,
        &portfolio.positions,
        "rating",
        portfolio.base_ccy,
    )?;
    println!("By rating:");
    for (rating, total) in &by_rating {
        println!("  {}: {}", rating, total);
    }

    let by_instrument_type = aggregate_by_attribute(
        &valuation,
        &portfolio.positions,
        "instrument_type",
        portfolio.base_ccy,
    )?;
    println!("\nBy instrument type:");
    for (inst_type, total) in &by_instrument_type {
        println!("  {}: {}", inst_type, total);
    }

    // 6. Apply scenario and re-value
    println!("\n--- Scenario Application ---");
    let scenario = ScenarioSpec {
        id: "stress_test".to_string(),
        name: Some("Rate Stress (+50bp)".to_string()),
        description: Some("Parallel 50bp shift to USD discount curve".to_string()),
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD".to_string(),
            bp: 50.0,
        }],
        priority: 0,
    };

    println!("Scenario: {}", scenario.name.as_ref().unwrap());
    println!("Description: {}", scenario.description.as_ref().unwrap());
    println!("Operations:");
    for (i, op) in scenario.operations.iter().enumerate() {
        match op {
            OperationSpec::CurveParallelBp {
                curve_kind,
                curve_id,
                bp,
            } => {
                println!(
                    "  {}. {:?} curve '{}': {:+.0}bp parallel shift",
                    i + 1,
                    curve_kind,
                    curve_id,
                    bp
                );
            }
            _ => println!("  {}. {:?}", i + 1, op),
        }
    }

    let (stressed_valuation, report) = apply_and_revalue(&portfolio, &scenario, &market, &config)?;

    println!("\nScenario Results:");
    println!("  Operations applied: {}", report.operations_applied);
    if !report.warnings.is_empty() {
        println!("  Warnings: {}", report.warnings.len());
        for warning in &report.warnings {
            println!("    - {}", warning);
        }
    }

    println!("\nValue Impact:");
    println!("  Base case value:    {}", valuation.total_base_ccy);
    println!(
        "  Stressed value:     {}",
        stressed_valuation.total_base_ccy
    );
    let delta = stressed_valuation
        .total_base_ccy
        .checked_sub(valuation.total_base_ccy)?;
    let pct_change = if valuation.total_base_ccy.amount() != 0.0 {
        (delta.amount() / valuation.total_base_ccy.amount().abs()) * 100.0
    } else {
        0.0
    };
    println!("  Change:             {} ({:+.2}%)", delta, pct_change);

    // Show per-entity impact
    println!("\nPer-Entity Impact:");
    for (entity_id, base_value) in &valuation.by_entity {
        if let Some(stressed_value) = stressed_valuation.by_entity.get(entity_id) {
            let entity_delta = stressed_value.checked_sub(*base_value)?;
            println!(
                "  {}: {} → {} (change: {})",
                entity_id, base_value, stressed_value, entity_delta
            );
        }
    }

    // Show metrics impact if available
    let stressed_metrics = aggregate_metrics(&stressed_valuation)?;
    if !metrics.aggregated.is_empty() && !stressed_metrics.aggregated.is_empty() {
        println!("\nMetrics Impact:");
        for (metric_id, base_agg) in &metrics.aggregated {
            if let Some(stressed_agg) = stressed_metrics.aggregated.get(metric_id) {
                let metric_delta = stressed_agg.total - base_agg.total;
                println!(
                    "  {}: {:.2} → {:.2} (change: {:+.2})",
                    metric_id, base_agg.total, stressed_agg.total, metric_delta
                );
            }
        }
    }

    // 7. Export to DataFrames
    println!("\n--- DataFrame Exports ---");
    let df_positions = finstack_portfolio::dataframe::positions_to_dataframe(&valuation)?;
    println!(
        "Positions DataFrame: {} rows x {} columns",
        df_positions.height(),
        df_positions.width()
    );

    let df_entities = finstack_portfolio::dataframe::entities_to_dataframe(&valuation)?;
    println!(
        "Entities DataFrame:  {} rows x {} columns",
        df_entities.height(),
        df_entities.width()
    );

    println!("\n=== Portfolio Example Complete ===");
    Ok(())
}

fn build_market_data(as_of: Date) -> MarketContext {
    // Create USD discount curve with realistic rates (~5% rate)
    let usd_curve = DiscountCurve::builder("USD")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0 / 365.0, 0.999863),  // 1 day: ~5% rate
            (7.0 / 365.0, 0.999042),  // 1 week
            (30.0 / 365.0, 0.995890), // 1 month
            (0.25, 0.9875),           // 3 months: ~5% rate
            (0.5, 0.975),             // 6 months
            (1.0, 0.95),              // 1 year: ~5.13% rate
            (2.0, 0.90),              // 2 years
            (5.0, 0.80),              // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create EUR discount curve with realistic rates (~3% rate)
    let eur_curve = DiscountCurve::builder("EUR")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0 / 365.0, 0.999918),  // 1 day: ~3% rate
            (7.0 / 365.0, 0.999426),  // 1 week
            (30.0 / 365.0, 0.997534), // 1 month
            (0.25, 0.9925),           // 3 months: ~3% rate
            (0.5, 0.985),             // 6 months
            (1.0, 0.97),              // 1 year: ~3.05% rate
            (2.0, 0.94),              // 2 years
            (5.0, 0.86),              // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create forward curve for SOFR (needed for IRS)
    let usd_sofr_fwd = ForwardCurve::builder("USD_SOFR_3M", 0.25)
        .base_date(as_of)
        .knots(vec![(0.0, 0.05), (1.0, 0.051), (2.0, 0.053), (5.0, 0.055)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create hazard curve for credit instruments
    let hazard_curve = HazardCurve::builder("CORP_BB")
        .base_date(as_of)
        .knots(vec![(0.0, 0.02), (1.0, 0.025), (5.0, 0.03), (10.0, 0.035)])
        .recovery_rate(0.4)
        .day_count(DayCount::Act365F)
        .build()
        .unwrap();

    // Create separate hazard curve for credit index data
    let index_hazard_curve = HazardCurve::builder("CDX_NA_IG_42")
        .base_date(as_of)
        .knots(vec![
            (0.0, 0.015), // Lower spread for IG index
            (1.0, 0.018),
            (5.0, 0.022),
            (10.0, 0.025),
        ])
        .recovery_rate(0.4)
        .day_count(DayCount::Act365F)
        .build()
        .unwrap();

    // Create volatility surface for CDS options
    let vol_surface = VolSurface::builder("CDS_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0]) // 3M, 6M, 1Y, 2Y, 5Y
        .strikes(&[50.0, 100.0, 150.0, 200.0, 300.0]) // Strike spreads in bp
        .row(&[0.35, 0.32, 0.30, 0.28, 0.25]) // 3M expiry vols
        .row(&[0.33, 0.30, 0.28, 0.26, 0.23]) // 6M expiry vols
        .row(&[0.30, 0.28, 0.26, 0.24, 0.21]) // 1Y expiry vols
        .row(&[0.28, 0.26, 0.24, 0.22, 0.19]) // 2Y expiry vols
        .row(&[0.25, 0.23, 0.21, 0.19, 0.16]) // 5Y expiry vols
        .build()
        .unwrap();

    // Create volatility surface for interest rate options (caps/floors/swaptions)
    let ir_vol_surface = VolSurface::builder("IR_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0]) // 3M, 6M, 1Y, 2Y, 5Y
        .strikes(&[0.02, 0.03, 0.04, 0.05, 0.06]) // Strike rates (2%, 3%, 4%, 5%, 6%)
        .row(&[0.25, 0.23, 0.21, 0.19, 0.17]) // 3M expiry vols
        .row(&[0.24, 0.22, 0.20, 0.18, 0.16]) // 6M expiry vols
        .row(&[0.23, 0.21, 0.19, 0.17, 0.15]) // 1Y expiry vols
        .row(&[0.22, 0.20, 0.18, 0.16, 0.14]) // 2Y expiry vols
        .row(&[0.20, 0.18, 0.16, 0.14, 0.12]) // 5Y expiry vols
        .build()
        .unwrap();

    // Create volatility surface for equity options
    let equity_vol_surface = VolSurface::builder("EQ_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0]) // 3M, 6M, 1Y, 2Y, 5Y
        .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0]) // Strike prices (80, 90, 100, 110, 120)
        .row(&[0.35, 0.33, 0.31, 0.29, 0.27]) // 3M expiry vols
        .row(&[0.33, 0.31, 0.29, 0.27, 0.25]) // 6M expiry vols
        .row(&[0.30, 0.28, 0.26, 0.24, 0.22]) // 1Y expiry vols
        .row(&[0.28, 0.26, 0.24, 0.22, 0.20]) // 2Y expiry vols
        .row(&[0.25, 0.23, 0.21, 0.19, 0.17]) // 5Y expiry vols
        .build()
        .unwrap();

    // Create FX volatility surface for FX options
    let fx_vol_surface = VolSurface::builder("FX_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0]) // 3M, 6M, 1Y, 2Y, 5Y
        .strikes(&[0.85, 0.90, 0.95, 1.00, 1.05]) // Strike prices (EUR/USD)
        .row(&[0.12, 0.11, 0.10, 0.09, 0.08]) // 3M expiry vols
        .row(&[0.11, 0.10, 0.09, 0.08, 0.07]) // 6M expiry vols
        .row(&[0.10, 0.09, 0.08, 0.07, 0.06]) // 1Y expiry vols
        .row(&[0.09, 0.08, 0.07, 0.06, 0.05]) // 2Y expiry vols
        .row(&[0.08, 0.07, 0.06, 0.05, 0.04]) // 5Y expiry vols
        .build()
        .unwrap();

    // Create base correlation curve for CDS tranches
    let base_correlation_curve = BaseCorrelationCurve::builder("CDX_NA_IG_42")
        .knots(vec![
            (3.0, 0.25),  // 0-3% tranche: 25% base correlation
            (7.0, 0.45),  // 0-7% tranche: 45% base correlation
            (10.0, 0.60), // 0-10% tranche: 60% base correlation
            (15.0, 0.75), // 0-15% tranche: 75% base correlation
            (30.0, 0.85), // 0-30% tranche: 85% base correlation
        ])
        .build()
        .unwrap();

    // Create credit index data for CDS tranches
    let credit_index_data = CreditIndexData::builder()
        .num_constituents(125) // CDX.NA.IG has 125 constituents
        .recovery_rate(0.4) // 40% recovery rate
        .index_credit_curve(Arc::new(index_hazard_curve))
        .base_correlation_curve(Arc::new(base_correlation_curve))
        .build()
        .unwrap();

    // Create equity spot prices
    let aapl_price = MarketScalar::Unitless(150.0);
    let msft_price = MarketScalar::Unitless(300.0);

    // Create dividend yields (annualized, decimal)
    let aapl_dividend_yield = MarketScalar::Unitless(0.015); // 1.5% annual dividend yield
    let msft_dividend_yield = MarketScalar::Unitless(0.008); // 0.8% annual dividend yield

    // Create inflation curves for inflation-linked instruments
    let us_cpi_curve = InflationCurve::builder("US-CPI")
        .base_cpi(100.0)
        .knots(vec![
            (0.0, 100.0),  // Base CPI level
            (1.0, 102.5),  // 1Y: 2.5% inflation
            (2.0, 105.0),  // 2Y: 2.5% inflation
            (5.0, 112.0),  // 5Y: 2.4% inflation
            (10.0, 125.0), // 10Y: 2.2% inflation
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    // Create inflation index for inflation-linked instruments
    let us_cpi_index = InflationIndex::new(
        "US-CPI-U",
        vec![
            (as_of, 100.0),
            (as_of + time::Duration::days(30), 100.2),
            (as_of + time::Duration::days(60), 100.4),
            (as_of + time::Duration::days(90), 100.6),
        ],
        Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear)
    .with_lag(InflationLag::Months(3));

    // Create FX matrix
    let fx_provider = SimpleFxProvider::new();
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    fx_provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    let fx_matrix = FxMatrix::new(Arc::new(fx_provider));

    MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve)
        .insert_forward(usd_sofr_fwd)
        .insert_hazard(hazard_curve)
        .insert_surface(vol_surface)
        .insert_surface(ir_vol_surface)
        .insert_surface(equity_vol_surface)
        .insert_surface(fx_vol_surface)
        .insert_inflation(us_cpi_curve)
        .insert_inflation_index("US-CPI-U", us_cpi_index)
        .insert_price("AAPL-SPOT", aapl_price)
        .insert_price("MSFT-SPOT", msft_price)
        .insert_price("AAPL-DIV-YIELD", aapl_dividend_yield)
        .insert_price("MSFT-DIV-YIELD", msft_dividend_yield)
        .insert_credit_index("CDX_NA_IG_42", credit_index_data)
        .insert_fx(fx_matrix)
}

fn build_sample_portfolio(as_of: Date) -> finstack_portfolio::Result<Portfolio> {
    // Create entities
    let acme_corp = Entity::new("ACME_CORP")
        .with_name("Acme Corporation")
        .with_tag("sector", "Technology")
        .with_tag("rating", "BBB");

    let fund_a = Entity::new("FUND_A")
        .with_name("Investment Fund A")
        .with_tag("sector", "Finance")
        .with_tag("rating", "AA");

    // Create instruments

    // 1. Deposit for ACME Corp (entity-based)
    let mut deposit1 = Deposit::builder()
        .id("DEP_ACME_3M".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();
    deposit1.quote_rate = Some(0.05); // 5% quoted rate

    let deposit1_position = Position::new(
        "POS_DEP_001",
        "ACME_CORP",
        "DEP_ACME_3M",
        Arc::new(deposit1),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "BBB")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Technology");

    // 2. Deposit for FUND_A (entity-based)
    let mut deposit2 = Deposit::builder()
        .id("DEP_FUND_6M".into())
        .notional(Money::new(10_000_000.0, Currency::USD)) // $10M
        .start(as_of)
        .end(date!(2024 - 07 - 01))
        .day_count(DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();
    deposit2.quote_rate = Some(0.045); // 4.5% quoted rate

    let deposit2_position = Position::new(
        "POS_DEP_002",
        "FUND_A",
        "DEP_FUND_6M",
        Arc::new(deposit2),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AA")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Finance");

    // 3. EUR-denominated deposit (standalone - demonstrates cross-currency)
    let mut deposit_eur = Deposit::builder()
        .id("DEP_EUR_3M".into())
        .notional(Money::new(10_000_000.0, Currency::EUR)) // €10M
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(DayCount::Act360)
        .discount_curve_id("EUR".into())
        .build()
        .unwrap();
    deposit_eur.quote_rate = Some(0.03); // 3% quoted rate

    let deposit_eur_position = Position::new(
        "POS_DEP_003",
        DUMMY_ENTITY_ID, // Standalone instrument
        "DEP_EUR_3M",
        Arc::new(deposit_eur),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AAA")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Banking");

    // 4. Fixed Rate Bond (entity-based)
    let bond_fixed = Bond::fixed(
        "BOND_FIXED_5Y",
        Money::new(10_000_000.0, Currency::USD), // $10M
        0.045,                                   // 4.5% coupon
        as_of,
        date!(2029 - 01 - 01),
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let bond_fixed_position = Position::new(
        "POS_BOND_FIXED",
        "FUND_A",
        "BOND_FIXED_5Y",
        Arc::new(bond_fixed),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap()
    .with_tag("rating", "A")
    .with_tag("instrument_type", "bond_fixed")
    .with_tag("sector", "Finance");

    // 5. Fixed Rate Corporate Bond with different convention (entity-based)
    let bond_corporate = Bond::with_convention(
        "BOND_CORP_3Y",
        Money::new(10_000_000.0, Currency::USD), // $10M
        0.05,                                    // 5% coupon
        as_of,
        date!(2027 - 01 - 01),
        crate::instruments::common::parameters::BondConvention::Corporate,
        "USD",
    )
    .expect("Bond::with_convention should succeed for Corporate bonds");

    let bond_corporate_position = Position::new(
        "POS_BOND_CORPORATE",
        "ACME_CORP",
        "BOND_CORP_3Y",
        Arc::new(bond_corporate),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap()
    .with_tag("rating", "BBB")
    .with_tag("instrument_type", "bond_corporate")
    .with_tag("sector", "Technology");

    // 6. Floating Rate Note (FRN) - references forward curve
    // For now, use a fixed bond to represent FRN functionality
    // Full FRN support with builder requires additional market data setup
    let bond_frn = Bond::fixed(
        "FRN_SOFR_3Y",
        Money::new(10_000_000.0, Currency::USD), // $10M
        0.0575,                                  // Approximate SOFR + 75bp = ~5.75%
        as_of,
        date!(2027 - 01 - 01),
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let bond_frn_position = Position::new(
        "POS_FRN",
        "FUND_A",
        "FRN_SOFR_3Y",
        Arc::new(bond_frn),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap()
    .with_tag("rating", "A-")
    .with_tag("instrument_type", "frn")
    .with_tag("sector", "Finance");

    // 7. Interest Rate Swap (standalone)
    let irs = InterestRateSwap::builder()
        .id("IRS_5Y".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::PayFixed)
        .fixed(FixedLegSpec {
            discount_curve_id: "USD".into(),
            rate: dec!(0.04),
            freq: finstack_core::dates::Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            start: as_of,
            end: date!(2029 - 01 - 01),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
        })
        .float(FloatLegSpec {
            discount_curve_id: "USD".into(),
            forward_curve_id: "USD_SOFR_3M".into(),
            spread_bp: dec!(25.0),
            freq: finstack_core::dates::Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            start: as_of,
            end: date!(2029 - 01 - 01),
            reset_lag_days: 2,
            compounding: Default::default(),
            payment_delay_days: 0,
        })
        .build()
        .unwrap();

    let irs_position = Position::new(
        "POS_IRS_001",
        DUMMY_ENTITY_ID, // Standalone derivative
        "IRS_5Y",
        Arc::new(irs),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "irs")
    .with_tag("sector", "Derivatives");

    // 8. FX Swap (standalone)
    let fx_swap = FxSwap::builder()
        .id("FX_SWAP_EURUSD".into())
        .base_notional(Money::new(10_000_000.0, Currency::EUR)) // €10M
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(date!(2024 - 02 - 01)) // 1 month from as_of
        .far_date(date!(2024 - 08 - 01)) // 7 months from as_of
        .domestic_discount_curve_id("USD".into())
        .foreign_discount_curve_id("EUR".into())
        .build()
        .unwrap();

    let fx_swap_position = Position::new(
        "POS_FXSWAP_001",
        DUMMY_ENTITY_ID, // Standalone FX instrument
        "FX_SWAP_EURUSD",
        Arc::new(fx_swap),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "fx_swap")
    .with_tag("sector", "FX");

    // 8. CDS (Credit Default Swap) - standalone
    let cds = CreditDefaultSwap::buy_protection(
        "CDS_5Y",
        Money::new(10_000_000.0, Currency::USD),
        100.0, // 100bp running spread
        as_of,
        date!(2029 - 01 - 01),
        "USD",     // Discount curve
        "CORP_BB", // Credit/hazard curve
    )?;

    let cds_position = Position::new(
        "POS_CDS_001",
        DUMMY_ENTITY_ID,
        "CDS_5Y",
        Arc::new(cds),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "BB")
    .with_tag("instrument_type", "cds")
    .with_tag("sector", "Credit");

    // 9. CDS Index (CDX.NA.IG) - standalone
    let index_params = CDSIndexParams::cdx_na_ig(42, 1, 100.0) // Series 42, Version 1, 100bp coupon
        .with_index_factor(0.95); // 95% index factor (some defaults occurred)

    let construction_params = CDSIndexConstructionParams::buy_protection(
        Money::new(10_000_000.0, Currency::USD), // $10M
    );

    let credit_params = finstack_valuations::instruments::common::parameters::CreditParams {
        reference_entity: "CORP_BB".into(),
        credit_curve_id: "CORP_BB".into(),
        recovery_rate: isda::STANDARD_RECOVERY_SENIOR,
    };

    let cds_index = CDSIndex::new_standard(
        "CDX_NA_IG_42",
        &index_params,
        &construction_params,
        as_of,
        date!(2029 - 01 - 01), // 5Y maturity
        &credit_params,
        "USD",
        "CORP_BB",
    )
    .expect("valid index parameters");

    let cds_index_position = Position::new(
        "POS_CDX_001",
        DUMMY_ENTITY_ID,
        "CDX_NA_IG_42",
        Arc::new(cds_index),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "IG")
    .with_tag("instrument_type", "cds_index")
    .with_tag("sector", "Credit");

    // 10. CDS Option (option on CDS spread) - standalone
    let option_params = CdsOptionParams::call(
        120.0,                                   // Strike at 120bp
        date!(2024 - 07 - 01),                   // 6M expiry
        date!(2029 - 01 - 01),                   // 5Y CDS maturity
        Money::new(10_000_000.0, Currency::USD), // $10M
    )
    .expect("valid CDS option params");

    let cds_option = CdsOption::new(
        "CDS_OPTION_6M",
        &option_params,
        &credit_params,
        "USD",
        "CDS_VOL",
    )
    .expect("valid CDS option");

    let cds_option_position = Position::new(
        "POS_CDSOPT_001",
        DUMMY_ENTITY_ID,
        "CDS_OPTION_6M",
        Arc::new(cds_option),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "cds_option")
    .with_tag("sector", "Derivatives");

    // 11. CDS Tranche (3-7% mezzanine tranche) - standalone
    let tranche_params = CDSTrancheParams::mezzanine_tranche(
        "CDX.NA.IG",
        42,                                      // Series 42
        Money::new(10_000_000.0, Currency::USD), // $10M
        date!(2029 - 01 - 01),                   // 5Y maturity
        500.0,                                   // 500bp running coupon
    );

    let schedule_params = finstack_valuations::cashflow::builder::ScheduleParams {
        freq: finstack_core::dates::Tenor::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: finstack_core::dates::StubKind::None,
    };

    let cds_tranche = CdsTranche::new(
        "CDS_TRANCHE_3_7",
        &tranche_params,
        &schedule_params,
        "USD",
        "CDX_NA_IG_42",
        TrancheSide::SellProtection, // Sell protection (receive premium, pay protection)
    );

    let cds_tranche_position = Position::new(
        "POS_TRANCHE_001",
        DUMMY_ENTITY_ID,
        "CDS_TRANCHE_3_7",
        Arc::new(cds_tranche),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "IG")
    .with_tag("instrument_type", "cds_tranche")
    .with_tag("sector", "Structured Credit");

    // 12. Interest Rate Cap - standalone
    let cap_params = InterestRateOptionParams::cap(
        Money::new(10_000_000.0, Currency::USD), // $10M
        0.05,                                    // 5% strike rate
        finstack_core::dates::Tenor::quarterly(),
        DayCount::Act360,
    );

    let interest_rate_cap = InterestRateOption::new(
        "IR_CAP_5Y",
        &cap_params,
        date!(2024 - 04 - 01), // Start 3 months from as_of
        date!(2029 - 04 - 01), // 5Y maturity from start
        "USD",
        "USD_SOFR_3M",
        "IR_VOL",
    );

    let cap_position = Position::new(
        "POS_CAP_001",
        DUMMY_ENTITY_ID,
        "IR_CAP_5Y",
        Arc::new(interest_rate_cap),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "cap")
    .with_tag("sector", "Interest Rate Options");

    // 13. Swaption (option on interest rate swap) - standalone
    let swaption_params = SwaptionParams::payer(
        Money::new(10_000_000.0, Currency::USD), // $10M
        0.045,                                   // 4.5% strike rate
        date!(2025 - 01 - 01),                   // 1Y expiry
        date!(2025 - 01 - 01),                   // Swap starts at expiry
        date!(2030 - 01 - 01),                   // 5Y swap maturity
    );

    let swaption = Swaption::new_payer(
        "SWAPTION_1Y5Y",
        &swaption_params,
        "USD",
        "USD_SOFR_3M",
        "IR_VOL",
    );

    let swaption_position = Position::new(
        "POS_SWAPTION_001",
        DUMMY_ENTITY_ID,
        "SWAPTION_1Y5Y",
        Arc::new(swaption),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "swaption")
    .with_tag("sector", "Interest Rate Options");

    // 14. Equity (Apple stock) - entity-based
    let apple_equity = Equity::builder()
        .id("AAPL".into())
        .ticker("AAPL".to_string())
        .currency(Currency::USD)
        .shares(66_666.67) // Shares to make ~$10M at $150/share
        .price_id("AAPL-SPOT".to_string())
        .div_yield_id("AAPL-DIV-YIELD".to_string())
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let apple_position = Position::new(
        "POS_AAPL_001",
        "ACME_CORP",
        "AAPL",
        Arc::new(apple_equity),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "equity")
    .with_tag("sector", "Technology");

    // 15. Equity Option (Microsoft call option) - standalone
    let option_params = EquityOptionParams::european_call(
        Money::new(300.0, Currency::USD), // $300 strike
        date!(2024 - 12 - 20),            // ~6M expiry
        33_333.33,                        // Shares to make ~$10M exposure
    );

    let underlying_params = finstack_valuations::instruments::common::parameters::underlying::EquityUnderlyingParams::new(
        "MSFT",
        "MSFT-SPOT",
        Currency::USD,
    )
    .with_contract_size(100.0)
    .with_dividend_yield("MSFT-DIV-YIELD");

    let msft_call_option = EquityOption::new(
        "MSFT_CALL_320",
        &option_params,
        &underlying_params,
        "USD".into(),
        "EQ_VOL".into(),
    );

    let msft_option_position = Position::new(
        "POS_MSFT_CALL",
        DUMMY_ENTITY_ID,
        "MSFT_CALL_320",
        Arc::new(msft_call_option),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "equity_option")
    .with_tag("sector", "Equity Options");

    // 16. Inflation-Linked Bond (US TIPS) - entity-based
    let tips_params = InflationLinkedBondParams::tips(
        Money::new(10_000_000.0, Currency::USD), // $10M notional
        0.025,                                   // 2.5% real coupon
        date!(2020 - 01 - 15),                   // Issue date
        date!(2030 - 01 - 15),                   // 10Y maturity
        100.0,                                   // Base CPI level
    );

    let tips_bond = InflationLinkedBond::builder()
        .id("TIPS_2030".into())
        .notional(tips_params.notional)
        .real_coupon(tips_params.real_coupon)
        .freq(tips_params.frequency)
        .dc(tips_params.day_count)
        .issue(tips_params.issue)
        .maturity(tips_params.maturity)
        .base_index(tips_params.base_index)
        .base_date(tips_params.issue)
        .indexation_method(finstack_valuations::instruments::inflation_linked_bond::IndexationMethod::TIPS)
        .lag(finstack_core::market_data::scalars::InflationLag::Months(3))
        .deflation_protection(finstack_valuations::instruments::inflation_linked_bond::DeflationProtection::MaturityOnly)
        .bdc(finstack_core::dates::BusinessDayConvention::ModifiedFollowing)
        .stub(finstack_core::dates::StubKind::None)
        .discount_curve_id("USD".into())
        .inflation_index_id("US-CPI".into())
        .build()
        .unwrap();

    let tips_position = Position::new(
        "POS_TIPS_001",
        "ACME_CORP",
        "TIPS_2030",
        Arc::new(tips_bond),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AAA")
    .with_tag("instrument_type", "inflation_linked_bond")
    .with_tag("sector", "Government");

    // 17. Inflation Swap (receive inflation, pay fixed) - standalone
    let inflation_swap = InflationSwap::builder()
        .id("INF_SWAP_5Y".into())
        .notional(Money::new(10_000_000.0, Currency::USD)) // $10M
        .start(as_of)
        .maturity(date!(2029 - 01 - 01)) // 5Y maturity
        .fixed_rate(0.025) // 2.5% fixed rate
        .inflation_index_id("US-CPI".into())
        .discount_curve_id("USD".into())
        .dc(finstack_core::dates::DayCount::Act365F)
        .side(PayReceiveInflation::ReceiveFixed) // Receive fixed, pay inflation
        .lag_override(finstack_core::market_data::scalars::InflationLag::Months(3))
        .build()
        .unwrap();

    let inflation_swap_position = Position::new(
        "POS_INFSWAP_001",
        DUMMY_ENTITY_ID,
        "INF_SWAP_5Y",
        Arc::new(inflation_swap),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "inflation_swap")
    .with_tag("sector", "Inflation Derivatives");

    // 18. FX Spot (EUR/USD) - standalone
    let eur_usd_spot = FxSpot::builder()
        .id("EUR_USD_SPOT".into())
        .base(Currency::EUR)
        .quote(Currency::USD)
        .settlement_lag_days(2)
        .spot_rate(1.10) // EUR/USD = 1.10
        .notional(Money::new(10_000_000.0, Currency::EUR)) // €10M
        .bdc(finstack_core::dates::BusinessDayConvention::Following)
        .build()
        .unwrap();

    let fx_spot_position = Position::new(
        "POS_FXSPOT_001",
        DUMMY_ENTITY_ID,
        "EUR_USD_SPOT",
        Arc::new(eur_usd_spot),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "fx_spot")
    .with_tag("sector", "Foreign Exchange");

    // 19. FX Option (EUR/USD Call) - standalone
    let fx_option_params = FxOptionParams::european_call(
        1.05,                                    // Strike: EUR/USD = 1.05
        date!(2024 - 12 - 31),                   // 1Y expiry
        Money::new(10_000_000.0, Currency::EUR), // €10M
    );

    let fx_underlying_params =
        finstack_valuations::instruments::common::parameters::FxUnderlyingParams::new(
            Currency::EUR,
            Currency::USD,
            "USD",
            "EUR",
        );

    let eur_usd_call = FxOption::new(
        "EUR_USD_CALL_105",
        &fx_option_params,
        &fx_underlying_params,
        "FX_VOL",
    );

    let fx_option_position = Position::new(
        "POS_FXOPT_001",
        DUMMY_ENTITY_ID,
        "EUR_USD_CALL_105",
        Arc::new(eur_usd_call),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "N/A")
    .with_tag("instrument_type", "fx_option")
    .with_tag("sector", "Foreign Exchange");

    // 20. CLO Mezzanine Tranche - standalone
    // Create a simple CLO with a mezzanine tranche
    let mut clo_pool = Pool::new("CLO_POOL_001", DealType::CLO, Currency::USD);

    // Add some sample corporate loans to the pool (scaled to make mezz tranche = $10M)
    let loan1 = finstack_valuations::instruments::bond::Bond::fixed(
        "LOAN_001",
        Money::new(7_142_857.14, Currency::USD), // Scaled down
        0.065,                                   // 6.5% coupon
        date!(2020 - 01 - 15),
        date!(2027 - 01 - 15),
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let loan2 = finstack_valuations::instruments::bond::Bond::fixed(
        "LOAN_002",
        Money::new(4_285_714.29, Currency::USD), // Scaled down
        0.055,                                   // 5.5% coupon
        date!(2020 - 03 - 15),
        date!(2026 - 03 - 15),
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    clo_pool.add_bond(&loan1, Some("Technology".to_string()));
    clo_pool.add_bond(&loan2, Some("Healthcare".to_string()));

    // Create tranche structure: Senior, Mezzanine, Equity
    let senior_tranche = TrancheBuilder::new()
        .id("SENIOR_A")
        .attachment_detachment(15.0, 100.0) // 15% subordination to 100%
        .seniority(Seniority::Senior)
        .balance(Money::new(10_000_000.0, Currency::USD)) // $10M senior tranche
        .coupon(TrancheCoupon::Fixed { rate: 0.035 }) // 3.5% coupon
        .legal_maturity(date!(2031 - 01 - 15))
        .build()
        .unwrap();

    let mezz_tranche = TrancheBuilder::new()
        .id("MEZZANINE_B")
        .attachment_detachment(5.0, 15.0) // 5% subordination to 15%
        .seniority(Seniority::Mezzanine)
        .balance(Money::new(10_000_000.0, Currency::USD)) // $10M mezzanine tranche
        .coupon(TrancheCoupon::Fixed { rate: 0.055 }) // 5.5% coupon
        .legal_maturity(date!(2031 - 01 - 15))
        .build()
        .unwrap();

    let equity_tranche = TrancheBuilder::new()
        .id("EQUITY")
        .attachment_detachment(0.0, 5.0) // 0% subordination to 5%
        .seniority(Seniority::Equity)
        .balance(Money::new(10_000_000.0, Currency::USD)) // $10M equity tranche
        .coupon(TrancheCoupon::Fixed { rate: 0.00 }) // No coupon
        .legal_maturity(date!(2031 - 01 - 15))
        .build()
        .unwrap();

    let tranches =
        TrancheStructure::new(vec![senior_tranche, mezz_tranche, equity_tranche]).unwrap();

    // Create the CLO instrument
    let clo_instrument = StructuredCredit::new_clo(
        "CLO_2024_001",
        clo_pool,
        tranches,
        date!(2024 - 01 - 15), // Closing date
        date!(2031 - 01 - 15), // Legal maturity
        "USD",
    );

    let clo_position = Position::new(
        "POS_CLO_MEZZ_001",
        DUMMY_ENTITY_ID,
        "CLO_2024_001",
        Arc::new(clo_instrument),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "BB")
    .with_tag("instrument_type", "structured_credit")
    .with_tag("sector", "Structured Credit")
    .with_tag("tranche", "mezzanine");

    // Build the portfolio
    let portfolio = PortfolioBuilder::new("SAMPLE_PORTFOLIO")
        .name("Sample Multi-Asset Portfolio")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(acme_corp)
        .entity(fund_a)
        .position(deposit1_position)
        .position(deposit2_position)
        .position(deposit_eur_position)
        .position(bond_fixed_position)
        .position(bond_corporate_position)
        .position(bond_frn_position)
        .position(irs_position)
        .position(fx_swap_position)
        .position(cds_position)
        .position(cds_index_position)
        .position(cds_option_position)
        .position(cds_tranche_position)
        .position(cap_position)
        .position(swaption_position)
        .position(apple_position)
        .position(msft_option_position)
        .position(tips_position)
        .position(inflation_swap_position)
        .position(fx_spot_position)
        .position(fx_option_position)
        .position(clo_position)
        .tag("strategy", "multi_asset")
        .build()?;

    Ok(portfolio)
}
