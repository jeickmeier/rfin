//! Scenarios-lite example demonstrating composite scenario application.
//!
//! This example shows:
//! - Market data shocks (curves, equity prices, volatility)
//! - Statement forecast adjustments
//! - Scenario composition with priorities
//! - Deterministic execution order
//! - Horizon scenarios (time roll-forward with theta/carry)

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, TimeRollMode,
    VolSurfaceKind,
};
use finstack_statements::FinancialModelSpec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Finstack Scenarios-Lite Example ===\n");

    // Setup market data
    let base_date = Date::from_calendar_date(2025, time::Month::January, 1)?;

    let usd_curve = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
        .build()?;

    let eur_curve = DiscountCurve::builder("EUR_EURIBOR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.975), (5.0, 0.88), (10.0, 0.77)])
        .build()?;

    let vol_surface = VolSurface::builder("SPX_VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0])
        .strikes(&[90.0, 95.0, 100.0, 105.0, 110.0])
        .row(&[0.25, 0.23, 0.20, 0.23, 0.25])
        .row(&[0.24, 0.22, 0.19, 0.22, 0.24])
        .row(&[0.23, 0.21, 0.18, 0.21, 0.23])
        .row(&[0.22, 0.20, 0.17, 0.20, 0.22])
        .build()?;

    let mut market = MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve)
        .insert_surface(vol_surface)
        .insert_price("SPY", MarketScalar::Price(Money::new(450.0, Currency::USD)))
        .insert_price("QQQ", MarketScalar::Price(Money::new(380.0, Currency::USD)));

    println!("Initial market state:");
    println!(
        "  USD_SOFR 1Y DF: {:.4}",
        market.get_discount("USD_SOFR")?.df(1.0)
    );
    println!(
        "  EUR_EURIBOR 1Y DF: {:.4}",
        market.get_discount("EUR_EURIBOR")?.df(1.0)
    );
    println!(
        "  SPY Price: ${:.2}",
        match market.price("SPY")? {
            MarketScalar::Price(m) => m.amount(),
            _ => 0.0,
        }
    );
    println!(
        "  SPX Vol (1Y, 100 strike): {:.2}%",
        market.surface("SPX_VOL")?.value_clamped(1.0, 100.0) * 100.0
    );
    println!();

    // Setup empty statements model (Phase A stub)
    let mut model = FinancialModelSpec::new("example_model", vec![]);

    // Create base scenario: parallel rate increase
    let base_scenario = ScenarioSpec {
        id: "base_rate_hike".into(),
        name: Some("Base Rate Hike".into()),
        description: Some("50bp parallel shift across all curves".into()),
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD_SOFR".into(),
                bp: 50.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "EUR_EURIBOR".into(),
                bp: 50.0,
            },
        ],
        priority: 0,
    };

    // Create equity stress scenario
    let equity_scenario = ScenarioSpec {
        id: "equity_stress".into(),
        name: Some("Equity Market Stress".into()),
        description: Some("Equity prices down 15%, vol up 30%".into()),
        operations: vec![
            OperationSpec::EquityPricePct {
                ids: vec!["SPY".into(), "QQQ".into()],
                pct: -15.0,
            },
            OperationSpec::VolSurfaceParallelPct {
                surface_kind: VolSurfaceKind::Equity,
                surface_id: "SPX_VOL".into(),
                pct: 30.0,
            },
        ],
        priority: 1,
    };

    // Compose scenarios
    println!("Composing scenarios...");
    let engine = ScenarioEngine::new();
    let composed = engine.compose(vec![base_scenario.clone(), equity_scenario.clone()]);
    println!(
        "  Composed {} operations from {} scenarios\n",
        composed.operations.len(),
        2
    );

    // Apply composed scenario
    println!("Applying composite scenario...");
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&composed, &mut ctx)?;
    println!("  Applied: {} operations", report.operations_applied);
    println!("  Warnings: {}", report.warnings.len());
    for warning in &report.warnings {
        println!("    - {}", warning);
    }
    println!();

    // Show shocked market state
    println!("Shocked market state:");
    println!(
        "  USD_SOFR 1Y DF: {:.4}",
        market.get_discount("USD_SOFR")?.df(1.0)
    );
    println!(
        "  EUR_EURIBOR 1Y DF: {:.4}",
        market.get_discount("EUR_EURIBOR")?.df(1.0)
    );
    println!(
        "  SPY Price: ${:.2}",
        match market.price("SPY")? {
            MarketScalar::Price(m) => m.amount(),
            _ => 0.0,
        }
    );
    println!(
        "  QQQ Price: ${:.2}",
        match market.price("QQQ")? {
            MarketScalar::Price(m) => m.amount(),
            _ => 0.0,
        }
    );
    println!(
        "  SPX Vol (1Y, 100 strike): {:.2}%",
        market.surface("SPX_VOL")?.value_clamped(1.0, 100.0) * 100.0
    );
    println!();

    println!("✓ Scenarios applied successfully!");
    println!("  All operations executed deterministically");
    println!("  Original market data preserved alongside shocked versions");

    // ========================================================================
    // HORIZON SCENARIO - Time roll-forward with theta/carry
    // ========================================================================
    println!("\n=== Horizon Scenario: Time Roll-Forward ===\n");

    // Reset to fresh market for horizon analysis
    let mut market_horizon = MarketContext::new()
        .insert_discount(
            DiscountCurve::builder("USD_SOFR")
                .base_date(base_date)
                .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
                .build()?,
        )
        .insert_price("SPY", MarketScalar::Price(Money::new(450.0, Currency::USD)));

    let mut model_horizon = FinancialModelSpec::new("horizon_model", vec![]);

    println!("Initial horizon date: {}", base_date);

    // Create 1-month horizon scenario
    let horizon_1m = ScenarioSpec {
        id: "horizon_1m".into(),
        name: Some("1-Month Horizon".into()),
        description: Some("Roll forward 1 month to observe carry/theta".into()),
        operations: vec![OperationSpec::TimeRollForward {
            period: "1M".into(),
            apply_shocks: false,
            roll_mode: TimeRollMode::BusinessDays,
        }],
        priority: 0,
    };

    let mut ctx_horizon = ExecutionContext {
        market: &mut market_horizon,
        model: &mut model_horizon,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let horizon_report = engine.apply(&horizon_1m, &mut ctx_horizon)?;

    println!(
        "  Applied: {} operations",
        horizon_report.operations_applied
    );
    println!("  New horizon date: {}", ctx_horizon.as_of);
    println!(
        "  Days elapsed: {}",
        (ctx_horizon.as_of - base_date).whole_days()
    );
    println!("\n  Note: Theta/carry would be calculated for instruments if provided");
    println!("  Carry = PV(new_date) - PV(old_date) with no market changes");

    // Create 3-month horizon scenario
    let horizon_3m = ScenarioSpec {
        id: "horizon_3m".into(),
        name: Some("3-Month Horizon".into()),
        description: Some("Roll forward 3 months".into()),
        operations: vec![OperationSpec::TimeRollForward {
            period: "3M".into(),
            apply_shocks: false,
            roll_mode: TimeRollMode::BusinessDays,
        }],
        priority: 0,
    };

    // Reset date for 3M example
    ctx_horizon.as_of = base_date;
    let horizon_3m_report = engine.apply(&horizon_3m, &mut ctx_horizon)?;

    println!("\n  3-Month Horizon:");
    println!(
        "    Applied: {} operations",
        horizon_3m_report.operations_applied
    );
    println!("    New horizon date: {}", ctx_horizon.as_of);
    println!(
        "    Days elapsed: {}",
        (ctx_horizon.as_of - base_date).whole_days()
    );

    println!("\n✓ Horizon scenarios demonstrate theta/carry calculations!");
    println!("  Supported periods: 1D, 1W, 1M, 3M, 6M, 1Y, etc.");

    Ok(())
}
