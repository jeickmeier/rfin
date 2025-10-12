//! Comprehensive scenarios example demonstrating all shock types.
//!
//! This example demonstrates:
//! - All curve types (discount, forecast, hazard, inflation)
//! - Volatility surface shocks
//! - Base correlation shocks  
//! - Equity price shocks
//! - FX shocks
//! - Statement forecast modifications
//! - Rate bindings for capital structure responsiveness
//! - Scenario composition

use finstack_core::currency::Currency;
use finstack_core::dates::{build_periods, Date};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::money::Money;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, VolSurfaceKind,
};
use finstack_statements::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
use indexmap::{indexmap, IndexMap};
use std::sync::Arc;
use time::Month;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comprehensive Scenarios Example ===\n");

    let base_date = Date::from_calendar_date(2025, Month::January, 1)?;

    // Build comprehensive market data
    let mut market = build_market(base_date)?;
    let mut model = build_model()?;

    println!("📊 Initial Market State:");
    print_market_state(&market);
    print_model_state(&model);

    // Create comprehensive stress scenario
    let stress_scenario = ScenarioSpec {
        id: "comprehensive_stress".into(),
        name: Some("Comprehensive Stress Test".into()),
        description: Some("Multi-asset stress with capital structure impact".into()),
        operations: vec![
            // Curve shocks
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD_SOFR".into(),
                bp: 100.0, // +100bp rate hike
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Forecast,
                curve_id: "USD_LIBOR_3M".into(),
                bp: 120.0, // +120bp (steeper forward curve)
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Hazard,
                curve_id: "CDX_IG".into(),
                bp: 50.0, // +50bp credit spread widening
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Inflation,
                curve_id: "USD_CPI".into(),
                bp: 25.0, // +0.25% inflation
            },
            // Equity shocks
            OperationSpec::EquityPricePct {
                ids: vec!["SPY".into(), "QQQ".into()],
                pct: -20.0, // -20% equity crash
            },
            // Volatility shock
            OperationSpec::VolSurfaceParallelPct {
                surface_kind: VolSurfaceKind::Equity,
                surface_id: "SPX_VOL".into(),
                pct: 50.0, // +50% vol spike
            },
            // Base correlation shock (credit)
            OperationSpec::BaseCorrParallelPts {
                surface_id: "CDX_IG_CORR".into(),
                points: 0.10, // +10 percentage points
            },
            // FX shock
            OperationSpec::MarketFxPct {
                base: Currency::EUR,
                quote: Currency::USD,
                pct: 5.0, // EUR strengthens
            },
            // Statement shocks
            OperationSpec::StmtForecastPercent {
                node_id: "Revenue".into(),
                pct: -10.0, // Revenue down
            },
            OperationSpec::StmtForecastPercent {
                node_id: "EBITDA".into(),
                pct: -15.0, // EBITDA margin compression
            },
        ],
        priority: 0,
    };

    // Configure rate bindings for capital structure
    let rate_bindings = Some(indexmap! {
        "DebtInterestRate".to_string() => "USD_SOFR".to_string(),
    });

    // Execute scenario
    println!("\n⚡ Applying Comprehensive Stress Scenario...");
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        rate_bindings,
        as_of: base_date,
    };

    let report = engine.apply(&stress_scenario, &mut ctx)?;

    println!("  ✓ Applied: {} operations", report.operations_applied);
    println!("  ⚠ Warnings: {}", report.warnings.len());
    for warning in &report.warnings {
        println!("    - {}", warning);
    }

    println!("\n📉 Shocked Market State:");
    print_market_state(&market);
    print_model_state(&model);

    println!("\n✅ All shock types executed successfully!");
    println!("   Demonstrates:");
    println!("   - Discount/forward/hazard/inflation curve shocks");
    println!("   - Equity price and volatility shocks");
    println!("   - Base correlation shocks");
    println!("   - FX rate shocks");
    println!("   - Statement forecast modifications");
    println!("   - Rate bindings for capital structure sensitivity");

    Ok(())
}

fn build_market(base_date: Date) -> Result<MarketContext, Box<dyn std::error::Error>> {
    // Discount curves
    let usd_sofr = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.975), (5.0, 0.88), (10.0, 0.75)])
        .build()?;

    // Forward curves
    let usd_libor = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(base_date)
        .knots(vec![(0.0, 0.03), (1.0, 0.032), (5.0, 0.035)])
        .build()?;

    // Hazard curves
    let cdx_ig = HazardCurve::builder("CDX_IG")
        .base_date(base_date)
        .knots(vec![(0.0, 0.005), (3.0, 0.008), (5.0, 0.010)])
        .recovery_rate(0.4)
        .build()?;

    // Inflation curves
    let usd_cpi = InflationCurve::builder("USD_CPI")
        .base_cpi(100.0)
        .knots(vec![(0.0, 100.0), (1.0, 102.5), (5.0, 112.0)])
        .build()?;

    // Volatility surface
    let spx_vol = VolSurface::builder("SPX_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0])
        .strikes(&[90.0, 95.0, 100.0, 105.0, 110.0])
        .row(&[0.22, 0.20, 0.18, 0.20, 0.22])
        .row(&[0.21, 0.19, 0.17, 0.19, 0.21])
        .row(&[0.20, 0.18, 0.16, 0.18, 0.20])
        .row(&[0.19, 0.17, 0.15, 0.17, 0.19])
        .build()?;

    // Base correlation
    let cdx_corr = BaseCorrelationCurve::builder("CDX_IG_CORR")
        .points(vec![(3.0, 0.30), (7.0, 0.50), (10.0, 0.60), (15.0, 0.70)])
        .build()?;

    // FX
    let fx_provider = Arc::new(SimpleFxProvider::new());
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.10);
    fx_provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    let fx_matrix = FxMatrix::new(fx_provider);

    // Equity prices
    let market = MarketContext::new()
        .insert_discount(usd_sofr)
        .insert_forward(usd_libor)
        .insert_hazard(cdx_ig)
        .insert_inflation(usd_cpi)
        .insert_surface(spx_vol)
        .insert_base_correlation(cdx_corr)
        .insert_fx(fx_matrix)
        .insert_price("SPY", MarketScalar::Price(Money::new(450.0, Currency::USD)))
        .insert_price("QQQ", MarketScalar::Price(Money::new(380.0, Currency::USD)));

    Ok(market)
}

fn build_model() -> Result<FinancialModelSpec, Box<dyn std::error::Error>> {
    let period_plan = build_periods("2025Q1..Q4", None)?;
    let periods = period_plan.periods;
    let mut model = FinancialModelSpec::new("stress_test_model", periods.clone());

    // Add revenue node
    let mut revenue_values = IndexMap::new();
    for (i, period) in periods.iter().enumerate() {
        revenue_values.insert(period.id, AmountOrScalar::Scalar(1_000_000.0 * (i as f64 + 1.0)));
    }
    model.add_node(NodeSpec::new("Revenue", NodeType::Value).with_values(revenue_values));

    // Add EBITDA node
    let mut ebitda_values = IndexMap::new();
    for (i, period) in periods.iter().enumerate() {
        ebitda_values.insert(period.id, AmountOrScalar::Scalar(200_000.0 * (i as f64 + 1.0)));
    }
    model.add_node(NodeSpec::new("EBITDA", NodeType::Value).with_values(ebitda_values));

    // Add interest rate node
    let mut rate_values = IndexMap::new();
    for period in &periods {
        rate_values.insert(period.id, AmountOrScalar::Scalar(0.025)); // 2.5% initial
    }
    model.add_node(NodeSpec::new("DebtInterestRate", NodeType::Value).with_values(rate_values));

    Ok(model)
}

fn print_market_state(market: &MarketContext) {
    println!("  Curves:");
    if let Ok(c) = market.get_discount("USD_SOFR") {
        println!("    USD_SOFR 1Y DF: {:.4}", c.df(1.0));
    }
    if let Ok(c) = market.get_discount("USD_SOFR_bump_100bp") {
        println!("    USD_SOFR_bump_100bp 1Y DF: {:.4}", c.df(1.0));
    }
    if let Ok(c) = market.get_forward("USD_LIBOR_3M") {
        println!("    USD_LIBOR_3M fwd[0]: {:.4}", c.forwards()[0]);
    }
    if let Ok(c) = market.get_forward("USD_LIBOR_3M_bump_120bp") {
        println!("    USD_LIBOR_3M_bump_120bp fwd[0]: {:.4}", c.forwards()[0]);
    }

    println!("  Equities:");
    if let Ok(MarketScalar::Price(m)) = market.price("SPY") {
        println!("    SPY: ${:.2}", m.amount());
    }
    if let Ok(MarketScalar::Price(m)) = market.price("QQQ") {
        println!("    QQQ: ${:.2}", m.amount());
    }

    println!("  Vol:");
    if let Ok(s) = market.surface("SPX_VOL") {
        println!("    SPX (1Y@100): {:.2}%", s.value(1.0, 100.0) * 100.0);
    }

    println!("  Base Corr:");
    if let Ok(c) = market.get_base_correlation("CDX_IG_CORR") {
        println!("    CDX_IG_CORR (7% detach): {:.2}%", c.correlation(7.0) * 100.0);
    }
    if let Ok(c) = market.get_base_correlation("CDX_IG_CORR_bump_10pct") {
        println!("    CDX_IG_CORR_bump_10pct (7% detach): {:.2}%", c.correlation(7.0) * 100.0);
    }

    println!("  FX:");
    if let Some(fx) = &market.fx {
        let query = finstack_core::money::fx::FxQuery::new(
            Currency::EUR,
            Currency::USD,
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        );
        if let Ok(result) = fx.rate(query) {
            println!("    EUR/USD: {:.4}", result.rate);
        }
    }
}

fn print_model_state(model: &FinancialModelSpec) {
    println!("  Statements:");
    if let Some(node) = model.get_node("Revenue") {
        if let Some(values) = &node.values {
            if let Some(AmountOrScalar::Scalar(s)) = values.values().next() {
                println!("    Revenue Q1: ${:.0}", s);
            }
        }
    }
    if let Some(node) = model.get_node("DebtInterestRate") {
        if let Some(values) = &node.values {
            if let Some(AmountOrScalar::Scalar(s)) = values.values().next() {
                println!("    Debt Rate: {:.2}%", s * 100.0);
            }
        }
    }
}

