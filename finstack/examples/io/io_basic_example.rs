//! Basic example of using finstack-io for persistence
//!
//! This example demonstrates:
//! - Opening/creating a SQLite database
//! - Saving and loading market data (curves)
//! - Saving and loading instruments
//! - Saving and loading portfolios with automatic instrument hydration
//! - Using lookback queries for historical data
//!
//! Run with: `cargo run --example io_basic_example --features "all"`

use finstack::{
    core::{
        currency::Currency,
        dates::Date,
        market_data::{context::MarketContext, term_structures::DiscountCurve},
        math::interp::InterpStyle,
        money::Money,
    },
    io::{BulkStore, LookbackStore, SqliteStore, Store},
    portfolio::{Entity, EntityId, Portfolio, Position, PositionUnit},
    valuations::instruments::{rates::deposit::Deposit, InstrumentJson},
};
use indexmap::IndexMap;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗄️  Finstack IO - Basic Persistence Example");
    println!("=============================================\n");

    // Create a temporary database (in real usage, use a persistent path)
    let db_path = std::env::temp_dir().join("finstack_example.db");
    println!("📁 Database path: {}\n", db_path.display());

    // Open (or create) the database - migrations run automatically
    let store = SqliteStore::open(&db_path)?;
    println!("✅ Database opened successfully\n");

    // =========================================================================
    // 1. SAVING AND LOADING MARKET DATA
    // =========================================================================
    println!("📈 1. Market Data Persistence");
    println!("   ----------------------------");

    let as_of = Date::from_calendar_date(2024, time::Month::January, 15)?;

    // Create a discount curve
    let usd_ois = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),     // Today
            (0.25, 0.9875), // 3M
            (0.5, 0.975),   // 6M
            (1.0, 0.95),    // 1Y
            (2.0, 0.90),    // 2Y
            (5.0, 0.80),    // 5Y
            (10.0, 0.67),   // 10Y
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()?;

    // Create a market context with this curve
    let market_ctx = MarketContext::new().insert_discount(usd_ois);

    // Save with optional metadata
    let meta = serde_json::json!({
        "source": "example",
        "curve_build_version": "1.0"
    });
    store.put_market_context("DEFAULT", as_of, &market_ctx, Some(&meta))?;
    println!("   ✅ Saved market context for {}", as_of);

    // Load it back
    let loaded_ctx = store
        .get_market_context("DEFAULT", as_of)?
        .expect("Market context should exist");

    let loaded_curve = loaded_ctx.get_discount("USD-OIS")?;
    println!("   ✅ Loaded market context");
    println!("      Curve ID: {}", loaded_curve.id().as_str());
    println!("      1Y DF: {:.6}", loaded_curve.df(1.0));
    println!("      5Y DF: {:.6}", loaded_curve.df(5.0));

    // =========================================================================
    // 2. SAVING AND LOADING INSTRUMENTS
    // =========================================================================
    println!("\n📋 2. Instrument Persistence");
    println!("   --------------------------");

    // Create some deposit instruments
    let deposit_1m = Deposit::builder()
        .id("USD_DEP_1M".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .start(as_of)
        .end(Date::from_calendar_date(2024, time::Month::February, 15)?)
        .quote_rate(0.0525)
        .day_count(finstack::core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let deposit_3m = Deposit::builder()
        .id("USD_DEP_3M".into())
        .notional(Money::new(25_000_000.0, Currency::USD))
        .start(as_of)
        .end(Date::from_calendar_date(2024, time::Month::April, 15)?)
        .quote_rate(0.0535)
        .day_count(finstack::core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    // Save instruments individually
    store.put_instrument(
        "USD_DEP_1M",
        &InstrumentJson::Deposit(deposit_1m.clone()),
        None,
    )?;
    store.put_instrument(
        "USD_DEP_3M",
        &InstrumentJson::Deposit(deposit_3m.clone()),
        None,
    )?;
    println!("   ✅ Saved 2 instruments");

    // Load an instrument
    let loaded_instr = store
        .get_instrument("USD_DEP_1M")?
        .expect("Instrument should exist");

    if let InstrumentJson::Deposit(dep) = loaded_instr {
        println!("   ✅ Loaded instrument: {}", dep.id);
        println!("      Notional: {:?}", dep.notional);
        if let Some(rate) = dep.quote_rate {
            println!("      Rate: {:.2}%", rate * 100.0);
        }
    }

    // =========================================================================
    // 3. BULK OPERATIONS (More Efficient)
    // =========================================================================
    println!("\n⚡ 3. Bulk Operations");
    println!("   -------------------");

    // Save multiple market contexts in a single transaction
    let d1 = Date::from_calendar_date(2024, time::Month::January, 16)?;
    let d2 = Date::from_calendar_date(2024, time::Month::January, 17)?;

    let curve1 = DiscountCurve::builder("USD-OIS")
        .base_date(d1)
        .knots(vec![(0.0, 1.0), (1.0, 0.951), (5.0, 0.801)])
        .set_interp(InterpStyle::LogLinear)
        .build()?;
    let ctx1 = MarketContext::new().insert_discount(curve1);

    let curve2 = DiscountCurve::builder("USD-OIS")
        .base_date(d2)
        .knots(vec![(0.0, 1.0), (1.0, 0.952), (5.0, 0.802)])
        .set_interp(InterpStyle::LogLinear)
        .build()?;
    let ctx2 = MarketContext::new().insert_discount(curve2);

    store
        .put_market_contexts_batch(&[("DEFAULT", d1, &ctx1, None), ("DEFAULT", d2, &ctx2, None)])?;
    println!("   ✅ Bulk saved 2 market contexts in single transaction");

    // =========================================================================
    // 4. LOOKBACK QUERIES (Historical Data)
    // =========================================================================
    println!("\n📊 4. Lookback Queries");
    println!("   --------------------");

    // List all market contexts in a date range
    let snapshots = store.list_market_contexts("DEFAULT", as_of, d2)?;
    println!("   Found {} market context snapshots:", snapshots.len());
    for snap in &snapshots {
        let curve = snap.context.get_discount("USD-OIS")?;
        println!("      {} -> 1Y DF: {:.6}", snap.as_of, curve.df(1.0));
    }

    // Get the latest context on or before a date
    let latest = store
        .latest_market_context_on_or_before("DEFAULT", d2)?
        .expect("Should have at least one context");
    println!("   Latest context on or before {}: {}", d2, latest.as_of);

    // =========================================================================
    // 5. PORTFOLIO WITH INSTRUMENT HYDRATION
    // =========================================================================
    println!("\n💼 5. Portfolio Persistence & Hydration");
    println!("   -------------------------------------");

    // Create a portfolio
    let mut portfolio = Portfolio::new("TREASURY_DESK", Currency::USD, as_of);
    portfolio
        .entities
        .insert(EntityId::new("FUND_A"), Entity::new("FUND_A"));

    // Add positions with inline instruments
    let instr1 = Arc::from(InstrumentJson::Deposit(deposit_1m).into_boxed()?);
    let instr2 = Arc::from(InstrumentJson::Deposit(deposit_3m).into_boxed()?);

    portfolio.positions.push(Position::new(
        "POS_1",
        "FUND_A",
        "USD_DEP_1M",
        instr1,
        1.0,
        PositionUnit::Units,
    )?);
    portfolio.positions.push(Position::new(
        "POS_2",
        "FUND_A",
        "USD_DEP_3M",
        instr2,
        1.0,
        PositionUnit::Units,
    )?);

    // Convert to spec and clear inline instruments to test hydration
    let mut spec = portfolio.to_spec();
    for pos in &mut spec.positions {
        pos.instrument_spec = None; // Clear to test hydration from registry
    }

    // Save portfolio spec (positions reference instruments by ID only)
    store.put_portfolio_spec("TREASURY_DESK", as_of, &spec, None)?;
    println!("   ✅ Saved portfolio spec (instruments referenced by ID)");

    // Load portfolio - instruments are automatically hydrated from the registry
    let hydrated = store.load_portfolio("TREASURY_DESK", as_of)?;
    println!("   ✅ Loaded and hydrated portfolio");
    println!("      Positions: {}", hydrated.positions.len());
    for pos in &hydrated.positions {
        println!(
            "      - {} (instrument: {})",
            pos.position_id.as_str(),
            pos.instrument_id
        );
    }

    // =========================================================================
    // 6. CONVENIENCE: LOAD PORTFOLIO WITH MARKET DATA
    // =========================================================================
    println!("\n🎯 6. Load Portfolio with Market Data");
    println!("   ------------------------------------");

    let (port, mkt) = store.load_portfolio_with_market("TREASURY_DESK", "DEFAULT", as_of)?;
    println!("   ✅ Loaded portfolio and market context together");
    println!(
        "      Portfolio: {} ({} positions)",
        port.id,
        port.positions.len()
    );
    println!("      Market curves: {}", mkt.stats().total_curves);

    // =========================================================================
    // 7. SCENARIOS AND STATEMENT MODELS
    // =========================================================================
    println!("\n📝 7. Scenarios and Statement Models");
    println!("   -----------------------------------");

    use finstack::scenarios::{CurveKind, OperationSpec, TenorMatchMode};

    // Create a rate shock scenario with actual operations
    let rate_shock = finstack::scenarios::ScenarioSpec {
        id: "rate_shock_100bp".into(),
        name: Some("100bp Parallel Rate Shock".into()),
        description: Some("Parallel shift of +100bp across all USD discount curves".into()),
        operations: vec![
            // Parallel shift to USD-OIS curve
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-OIS".into(),
                bp: 100.0, // +100bp parallel shift
            },
        ],
        priority: 0,
    };
    store.put_scenario("RATE_SHOCK_100BP", &rate_shock, None)?;
    println!("   ✅ Saved scenario: {}", rate_shock.id);
    println!("      Operations: {} shock(s)", rate_shock.operations.len());

    // Create a more complex steepener scenario
    let steepener = finstack::scenarios::ScenarioSpec {
        id: "curve_steepener".into(),
        name: Some("2s10s Steepener".into()),
        description: Some("Short end down, long end up - simulates steepening yield curve".into()),
        operations: vec![
            // Tenor-specific shocks for curve reshaping
            OperationSpec::CurveNodeBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-OIS".into(),
                nodes: vec![
                    ("3M".into(), -25.0), // Short end down 25bp
                    ("6M".into(), -20.0),
                    ("1Y".into(), -10.0),
                    ("2Y".into(), 0.0), // Pivot point
                    ("5Y".into(), 15.0),
                    ("10Y".into(), 30.0), // Long end up 30bp
                    ("30Y".into(), 40.0),
                ],
                match_mode: TenorMatchMode::Interpolate,
            },
        ],
        priority: 1,
    };
    store.put_scenario("CURVE_STEEPENER", &steepener, None)?;
    println!("   ✅ Saved scenario: {}", steepener.id);
    println!("      Operations: {} shock(s)", steepener.operations.len());

    // Create an FX stress scenario
    let fx_stress = finstack::scenarios::ScenarioSpec {
        id: "usd_strengthening".into(),
        name: Some("USD Strengthening Stress".into()),
        description: Some("USD appreciates 10% against major currencies".into()),
        operations: vec![
            OperationSpec::MarketFxPct {
                base: Currency::USD,
                quote: Currency::EUR,
                pct: 10.0, // USD strengthens 10% vs EUR
            },
            OperationSpec::MarketFxPct {
                base: Currency::USD,
                quote: Currency::GBP,
                pct: 10.0, // USD strengthens 10% vs GBP
            },
            OperationSpec::MarketFxPct {
                base: Currency::USD,
                quote: Currency::JPY,
                pct: 10.0, // USD strengthens 10% vs JPY
            },
        ],
        priority: 0,
    };
    store.put_scenario("USD_STRENGTHENING", &fx_stress, None)?;
    println!("   ✅ Saved scenario: {}", fx_stress.id);
    println!("      Operations: {} shock(s)", fx_stress.operations.len());

    // Load and display a scenario
    let loaded_scenario = store
        .get_scenario("CURVE_STEEPENER")?
        .expect("Scenario should exist");
    println!(
        "\n   📋 Loaded scenario: {} - {:?}",
        loaded_scenario.id, loaded_scenario.name
    );
    println!("      Description: {:?}", loaded_scenario.description);
    println!("      Operations:");
    for (i, op) in loaded_scenario.operations.iter().enumerate() {
        match op {
            OperationSpec::CurveParallelBp {
                curve_kind,
                curve_id,
                bp,
            } => {
                println!(
                    "        {}. {:?} {} parallel shift: {:+}bp",
                    i + 1,
                    curve_kind,
                    curve_id,
                    bp
                );
            }
            OperationSpec::CurveNodeBp {
                curve_kind,
                curve_id,
                nodes,
                ..
            } => {
                println!(
                    "        {}. {:?} {} node shocks:",
                    i + 1,
                    curve_kind,
                    curve_id
                );
                for (tenor, bp) in nodes {
                    println!("           {}: {:+}bp", tenor, bp);
                }
            }
            OperationSpec::MarketFxPct { base, quote, pct } => {
                println!("        {}. FX {}/{}: {:+.1}%", i + 1, base, quote, pct);
            }
            _ => println!("        {}. {:?}", i + 1, op),
        }
    }

    // Create a statement model with periods and nodes
    use finstack::core::dates::{build_periods, PeriodId};
    use finstack::statements::types::{AmountOrScalar, ForecastSpec, NodeSpec, NodeType};

    // Build quarterly periods for 2024
    let period_plan = build_periods("2024Q1..Q4", Some("2024Q2"))?; // Q1-Q2 are actuals

    // Create a simple P&L model
    let mut model =
        finstack::statements::FinancialModelSpec::new("quarterly_pnl", period_plan.periods);

    // Add revenue node with explicit values for actuals and growth forecast for future periods
    let mut revenue_values = IndexMap::new();
    revenue_values.insert(
        PeriodId::quarter(2024, 1),
        AmountOrScalar::amount(1_500_000.0, Currency::USD),
    );
    revenue_values.insert(
        PeriodId::quarter(2024, 2),
        AmountOrScalar::amount(1_650_000.0, Currency::USD),
    );
    // Q3 and Q4 will use the growth forecast (10% QoQ growth)

    let revenue = NodeSpec::new("revenue", NodeType::Mixed)
        .with_name("Total Revenue")
        .with_values(revenue_values)
        .with_forecast(ForecastSpec::growth(0.10)) // 10% quarter-over-quarter growth
        .with_tags(vec!["income_statement".into(), "top_line".into()]);
    model.nodes.insert("revenue".into(), revenue);

    // Add COGS as a calculated node (formula-based)
    let cogs = NodeSpec::new("cogs", NodeType::Calculated)
        .with_name("Cost of Goods Sold")
        .with_formula("revenue * -0.60") // 60% COGS margin
        .with_tags(vec!["income_statement".into(), "expense".into()]);
    model.nodes.insert("cogs".into(), cogs);

    // Add Gross Profit as calculated
    let gross_profit = NodeSpec::new("gross_profit", NodeType::Calculated)
        .with_name("Gross Profit")
        .with_formula("revenue + cogs") // Revenue - COGS (COGS is negative)
        .with_tags(vec!["income_statement".into(), "subtotal".into()]);
    model.nodes.insert("gross_profit".into(), gross_profit);

    // Add Operating Expenses with curve-based forecast (different growth per period)
    let mut opex_values = IndexMap::new();
    opex_values.insert(
        PeriodId::quarter(2024, 1),
        AmountOrScalar::amount(-350_000.0, Currency::USD),
    );
    opex_values.insert(
        PeriodId::quarter(2024, 2),
        AmountOrScalar::amount(-375_000.0, Currency::USD),
    );

    let opex = NodeSpec::new("opex", NodeType::Mixed)
        .with_name("Operating Expenses")
        .with_values(opex_values)
        .with_forecast(ForecastSpec::curve(vec![0.05, 0.03])) // Q3: +5%, Q4: +3% (expense growth slowing)
        .with_tags(vec!["income_statement".into(), "expense".into()]);
    model.nodes.insert("opex".into(), opex);

    // Add headcount with forward-fill forecast (carry last value)
    let mut headcount_values = IndexMap::new();
    headcount_values.insert(PeriodId::quarter(2024, 1), AmountOrScalar::scalar(45.0));
    headcount_values.insert(PeriodId::quarter(2024, 2), AmountOrScalar::scalar(48.0));

    let headcount = NodeSpec::new("headcount", NodeType::Mixed)
        .with_name("Employee Headcount")
        .with_values(headcount_values)
        .with_forecast(ForecastSpec::forward_fill()) // Assume headcount stays flat
        .with_tags(vec!["kpi".into(), "operating".into()]);
    model.nodes.insert("headcount".into(), headcount);

    // Add EBITDA
    let ebitda = NodeSpec::new("ebitda", NodeType::Calculated)
        .with_name("EBITDA")
        .with_formula("gross_profit + opex")
        .with_tags(vec!["income_statement".into(), "profit".into()]);
    model.nodes.insert("ebitda".into(), ebitda);

    // Add revenue per employee as a derived KPI
    let rev_per_head = NodeSpec::new("revenue_per_employee", NodeType::Calculated)
        .with_name("Revenue per Employee")
        .with_formula("revenue / headcount")
        .with_tags(vec!["kpi".into(), "efficiency".into()]);
    model
        .nodes
        .insert("revenue_per_employee".into(), rev_per_head);

    // -------------------------------------------------------------------------
    // Add EBITDA Adjustments (normalization for add-backs like synergies, one-time costs)
    // -------------------------------------------------------------------------
    use finstack::statements::adjustments::types::{
        Adjustment, AdjustmentValue, NormalizationConfig,
    };

    // Create adjustment amounts per period
    let mut synergy_amounts = IndexMap::new();
    synergy_amounts.insert(PeriodId::quarter(2024, 1), 50_000.0);
    synergy_amounts.insert(PeriodId::quarter(2024, 2), 75_000.0);
    synergy_amounts.insert(PeriodId::quarter(2024, 3), 100_000.0);
    synergy_amounts.insert(PeriodId::quarter(2024, 4), 100_000.0);

    let mut restructuring_amounts = IndexMap::new();
    restructuring_amounts.insert(PeriodId::quarter(2024, 1), 25_000.0);
    restructuring_amounts.insert(PeriodId::quarter(2024, 2), 15_000.0);

    // Build the normalization config for adjusted EBITDA
    let normalization = NormalizationConfig::new("ebitda")
        .add_adjustment(
            Adjustment::fixed("synergies", "Expected Synergies", synergy_amounts)
                .with_cap(Some("ebitda".into()), 0.20), // Cap at 20% of EBITDA
        )
        .add_adjustment(Adjustment::fixed(
            "restructuring",
            "Restructuring Costs (One-Time)",
            restructuring_amounts,
        ))
        .add_adjustment(
            Adjustment::percentage(
                "mgmt_fees",
                "Management Fee Add-back",
                "revenue",
                0.02, // 2% of revenue
            )
            .with_cap(None, 50_000.0), // Hard cap at $50k
        );

    // Store adjustments in model metadata (serializable)
    model.meta.insert(
        "ebitda_adjustments".into(),
        serde_json::to_value(&normalization)?,
    );

    store.put_statement_model("QUARTERLY_PNL", &model, None)?;
    println!("\n   📊 Saved statement model: {}", model.id);
    println!("      Periods: {} quarters", model.periods.len());
    for p in &model.periods {
        let kind = if p.is_actual { "actual" } else { "forecast" };
        println!("        {} ({} to {}) - {}", p.id, p.start, p.end, kind);
    }
    println!("      Nodes: {}", model.nodes.len());
    for (id, node) in &model.nodes {
        let formula = node
            .formula_text
            .as_ref()
            .map(|f| format!(" = {}", f))
            .unwrap_or_default();
        let forecast = node
            .forecast
            .as_ref()
            .map(|f| format!(" [forecast: {:?}]", f.method))
            .unwrap_or_default();
        println!(
            "        {} ({:?}){}{} {:?}",
            id, node.node_type, formula, forecast, node.tags
        );
    }

    // Load and verify - check that forecasts round-trip correctly
    let loaded_model = store
        .get_statement_model("QUARTERLY_PNL")?
        .expect("Statement model should exist");
    println!(
        "\n   ✅ Loaded statement model: {} with {} periods and {} nodes",
        loaded_model.id,
        loaded_model.periods.len(),
        loaded_model.nodes.len()
    );

    // Verify forecasts were saved/loaded correctly
    println!("      Forecast verification:");
    for (id, node) in &loaded_model.nodes {
        if let Some(forecast) = &node.forecast {
            println!(
                "        {} -> {:?} with params: {:?}",
                id, forecast.method, forecast.params
            );
        }
    }

    // Verify adjustments were saved/loaded correctly
    if let Some(adj_value) = loaded_model.meta.get("ebitda_adjustments") {
        let loaded_adj: NormalizationConfig = serde_json::from_value(adj_value.clone())?;
        println!("\n      Adjustments for '{}':", loaded_adj.target_node);
        for adj in &loaded_adj.adjustments {
            let cap_info = adj
                .cap
                .as_ref()
                .map(|c| {
                    if let Some(base) = &c.base_node {
                        format!(" (capped at {:.0}% of {})", c.value * 100.0, base)
                    } else {
                        format!(" (capped at ${:.0})", c.value)
                    }
                })
                .unwrap_or_default();
            let value_desc = match &adj.value {
                AdjustmentValue::Fixed { amounts } => {
                    format!("{} periods of fixed amounts", amounts.len())
                }
                AdjustmentValue::PercentageOfNode {
                    node_id,
                    percentage,
                } => {
                    format!("{:.1}% of {}", percentage * 100.0, node_id)
                }
                AdjustmentValue::Formula { expression } => format!("formula: {}", expression),
            };
            println!(
                "        {} - {} [{}]{}",
                adj.id, adj.name, value_desc, cap_info
            );
        }
    }

    // =========================================================================
    // 8. METRIC REGISTRIES (Reusable financial metrics)
    // =========================================================================
    println!("\n📐 8. Metric Registries");
    println!("   ----------------------");

    use finstack::statements::registry::{MetricDefinition, MetricRegistry, UnitType};

    // Create a standard "fin" registry with common financial metrics
    let fin_registry = MetricRegistry {
        namespace: "fin".into(),
        schema_version: 1,
        metrics: vec![
            MetricDefinition {
                id: "gross_margin".into(),
                name: "Gross Margin %".into(),
                formula: "gross_profit / revenue".into(),
                description: Some("Gross profit as percentage of revenue".into()),
                category: Some("margins".into()),
                unit_type: Some(UnitType::Percentage),
                requires: vec!["revenue".into(), "gross_profit".into()],
                tags: vec!["margins".into(), "profitability".into()],
                meta: IndexMap::new(),
            },
            MetricDefinition {
                id: "ebitda_margin".into(),
                name: "EBITDA Margin %".into(),
                formula: "ebitda / revenue".into(),
                description: Some("EBITDA as percentage of revenue".into()),
                category: Some("margins".into()),
                unit_type: Some(UnitType::Percentage),
                requires: vec!["revenue".into(), "ebitda".into()],
                tags: vec![
                    "margins".into(),
                    "profitability".into(),
                    "cash_proxy".into(),
                ],
                meta: IndexMap::new(),
            },
            MetricDefinition {
                id: "debt_to_ebitda".into(),
                name: "Debt to EBITDA".into(),
                formula: "total_debt / ebitda".into(),
                description: Some("Leverage ratio: total debt divided by EBITDA".into()),
                category: Some("leverage".into()),
                unit_type: Some(UnitType::Ratio),
                requires: vec!["total_debt".into(), "ebitda".into()],
                tags: vec!["leverage".into(), "credit".into()],
                meta: IndexMap::new(),
            },
        ],
        meta: IndexMap::new(),
    };

    // Create a custom registry with company-specific metrics
    let custom_registry = MetricRegistry {
        namespace: "custom".into(),
        schema_version: 1,
        metrics: vec![
            MetricDefinition {
                id: "revenue_growth".into(),
                name: "Revenue Growth %".into(),
                formula: "(revenue - revenue[-1]) / revenue[-1]".into(),
                description: Some("Quarter-over-quarter revenue growth rate".into()),
                category: Some("growth".into()),
                unit_type: Some(UnitType::Percentage),
                requires: vec!["revenue".into()],
                tags: vec!["growth".into(), "top_line".into()],
                meta: IndexMap::new(),
            },
            MetricDefinition {
                id: "revenue_per_head".into(),
                name: "Revenue per Employee".into(),
                formula: "revenue / headcount".into(),
                description: Some("Revenue divided by headcount".into()),
                category: Some("efficiency".into()),
                unit_type: Some(UnitType::Currency),
                requires: vec!["revenue".into(), "headcount".into()],
                tags: vec!["kpi".into(), "efficiency".into()],
                meta: IndexMap::new(),
            },
        ],
        meta: IndexMap::new(),
    };

    // Store both registries using the dedicated API
    store.put_metric_registry("fin", &fin_registry, None)?;
    store.put_metric_registry("custom", &custom_registry, None)?;
    println!("   ✅ Saved metric registries: 'fin' and 'custom'");

    // List all available registries
    let namespaces = store.list_metric_registries()?;
    println!("   Available namespaces: {:?}", namespaces);

    // Load and display each registry
    for ns in &namespaces {
        let loaded = store.load_metric_registry(ns)?;
        println!(
            "\n   📊 Registry '{}' (v{}, {} metrics):",
            loaded.namespace,
            loaded.schema_version,
            loaded.metrics.len()
        );
        for metric in &loaded.metrics {
            let unit = metric
                .unit_type
                .map(|u| format!(" [{:?}]", u))
                .unwrap_or_default();
            let desc = metric
                .description
                .as_ref()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();
            println!(
                "      {}.{}{} = {}{}",
                loaded.namespace, metric.id, unit, metric.formula, desc
            );
        }
    }

    // Demonstrate deletion
    println!("\n   Deleting 'custom' registry...");
    let deleted = store.delete_metric_registry("custom")?;
    println!("   Deleted: {}", deleted);

    let remaining = store.list_metric_registries()?;
    println!("   Remaining namespaces: {:?}", remaining);

    // =========================================================================
    // SUMMARY
    // =========================================================================
    println!("\n🎉 Example Complete!");
    println!("=====================");
    println!("Key takeaways:");
    println!("  • SqliteStore::open() creates/migrates the database automatically");
    println!("  • Use put_*/get_* for individual operations");
    println!("  • Use bulk methods (put_*_batch) for efficiency with many records");
    println!("  • load_portfolio() automatically hydrates instruments from registry");
    println!("  • LookbackStore provides historical queries (list_*, latest_*_on_or_before)");
    println!("  • MetricRegistry is a first-class entity with dedicated CRUD operations");
    println!("  • Adjustments are stored in FinancialModelSpec.meta for model-specific config");
    println!("  • All data is stored as JSON blobs with SQL indexes for fast lookup");

    // Clean up
    //std::fs::remove_file(&db_path)?;
    //println!("\n🧹 Cleaned up temporary database");

    Ok(())
}
