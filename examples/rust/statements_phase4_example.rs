//! Phase 4 Example — Forecasting
//!
//! This example demonstrates the Phase 4 features of the finstack-statements crate:
//! - Forward Fill forecast method
//! - Growth Percentage (compound growth) forecast
//! - Curve Percentage (period-specific growth rates)
//! - Override forecast with explicit values
//! - Normal distribution forecast (statistical, deterministic)
//! - LogNormal distribution forecast (always positive)
//! - Combining forecasts with formulas
//! - Complete P&L model with mixed forecast methods

use finstack_statements::prelude::*;
use indexmap::indexmap;

fn main() -> Result<()> {
    println!("=== Phase 4: Forecasting Examples ===\n");

    // Example 1: Forward Fill
    example_1_forward_fill()?;

    // Example 2: Growth Percentage
    example_2_growth_pct()?;

    // Example 3: Curve Percentage
    example_3_curve_pct()?;

    // Example 4: Override
    example_4_override()?;

    // Example 5: Statistical Forecasts (Normal)
    example_5_normal_forecast()?;

    // Example 6: Statistical Forecasts (LogNormal)
    example_6_lognormal_forecast()?;

    // Example 7: Forecasts with Formula Dependencies
    example_7_forecast_with_formulas()?;

    // Example 8: Complete P&L with Mixed Forecasts
    example_8_complete_pl_with_forecasts()?;

    // Example 9: Negative Growth (Declining Revenue)
    example_9_negative_growth()?;

    println!("\n✅ All Phase 4 examples completed successfully!");

    Ok(())
}

/// Example 1: Forward Fill
///
/// The simplest forecast method - carries the last actual value forward to all forecast periods.
fn example_1_forward_fill() -> Result<()> {
    println!("📈 Example 1: Forward Fill");
    println!("---------------------------");
    println!("Carry the last actual value (Q2: 110,000) forward to Q3-Q4\n");

    let model = ModelBuilder::new("Forward Fill")
        .periods("2025Q1..Q4", Some("2025Q2"))?
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::ForwardFill,
                params: indexmap! {},
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        println!("  {} ({}): ${:>12.2}", period.id, period_type, value);
    }

    println!();
    Ok(())
}

/// Example 2: Growth Percentage
///
/// Applies compound growth rate period-over-period.
fn example_2_growth_pct() -> Result<()> {
    println!("📈 Example 2: Growth Percentage (5% compound growth)");
    println!("-----------------------------------------------------");
    println!("Starting from Q1: 100,000, apply 5% compound growth\n");

    let model = ModelBuilder::new("Growth Percentage")
        .periods("2025Q1..2026Q2", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    let mut prev_value = 100_000.0;
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        let growth = if period.is_actual {
            0.0
        } else {
            (value - prev_value) / prev_value * 100.0
        };
        println!(
            "  {} ({}): ${:>12.2}  (growth: {:>6.2}%)",
            period.id, period_type, value, growth
        );
        prev_value = value;
    }

    println!();
    Ok(())
}

/// Example 3: Curve Percentage
///
/// Applies different growth rates for each forecast period.
fn example_3_curve_pct() -> Result<()> {
    println!("📈 Example 3: Curve Percentage (variable growth rates)");
    println!("-------------------------------------------------------");
    println!("Q2: +5%, Q3: +6%, Q4: +5%, 2026Q1: +4%\n");

    let model = ModelBuilder::new("Curve Percentage")
        .periods("2025Q1..2026Q1", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::CurvePct,
                params: indexmap! {
                    "curve".into() => serde_json::json!([0.05, 0.06, 0.05, 0.04])
                },
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    let curve_rates = [0.05, 0.06, 0.05, 0.04];
    let mut curve_idx = 0;
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        let rate_str = if period.is_actual {
            "       -".to_string()
        } else {
            format!("{:>7.1}%", curve_rates[curve_idx] * 100.0)
        };
        if !period.is_actual {
            curve_idx += 1;
        }
        println!(
            "  {} ({}): ${:>12.2}  (rate: {})",
            period.id, period_type, value, rate_str
        );
    }

    println!();
    Ok(())
}

/// Example 4: Override
///
/// Uses explicit values for specific periods, forward fills for gaps.
fn example_4_override() -> Result<()> {
    println!("📈 Example 4: Override (explicit period values)");
    println!("------------------------------------------------");
    println!("Override Q2 and Q4, forward fill Q3\n");

    let model = ModelBuilder::new("Override")
        .periods("2025Q1..Q4", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::Override,
                params: indexmap! {
                    "overrides".into() => serde_json::json!({
                        "2025Q2": 120_000.0,
                        "2025Q4": 140_000.0,
                    })
                },
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        let source = match period.id.to_string().as_str() {
            "2025Q1" => "(actual)",
            "2025Q2" => "(override)",
            "2025Q3" => "(forward fill from Q2)",
            "2025Q4" => "(override)",
            _ => "",
        };
        println!(
            "  {} ({}): ${:>12.2}  {}",
            period.id, period_type, value, source
        );
    }

    println!();
    Ok(())
}

/// Example 5: Normal Distribution Forecast
///
/// Samples from a normal distribution with deterministic seeding.
fn example_5_normal_forecast() -> Result<()> {
    println!("📊 Example 5: Normal Distribution Forecast");
    println!("--------------------------------------------");
    println!("Mean: 100,000, Std Dev: 15,000, Seed: 42\n");

    let model = ModelBuilder::new("Normal Forecast")
        .periods("2025Q1..Q4", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::Normal,
                params: indexmap! {
                    "mean".into() => serde_json::json!(100_000.0),
                    "std_dev".into() => serde_json::json!(15_000.0),
                    "seed".into() => serde_json::json!(42),
                },
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results (deterministic with seed=42):");
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        let z_score = if period.is_actual {
            0.0
        } else {
            (value - 100_000.0) / 15_000.0
        };
        println!(
            "  {} ({}): ${:>12.2}  (z-score: {:>6.2})",
            period.id, period_type, value, z_score
        );
    }

    println!("\nNote: Same seed always produces same sequence (deterministic)\n");
    Ok(())
}

/// Example 6: LogNormal Distribution Forecast
///
/// Samples from a log-normal distribution (always positive).
fn example_6_lognormal_forecast() -> Result<()> {
    println!("📊 Example 6: LogNormal Distribution Forecast");
    println!("----------------------------------------------");
    println!("Mean: 11.5, Std Dev: 0.15, Seed: 42 (always positive)\n");

    let model = ModelBuilder::new("LogNormal Forecast")
        .periods("2025Q1..Q4", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::LogNormal,
                params: indexmap! {
                    "mean".into() => serde_json::json!(11.5),  // ln(~99,500)
                    "std_dev".into() => serde_json::json!(0.15),
                    "seed".into() => serde_json::json!(42),
                },
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results (all values guaranteed positive):");
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        println!("  {} ({}): ${:>12.2}", period.id, period_type, value);
    }

    println!("\nNote: LogNormal ensures positive values (e.g., for prices, volatility)\n");
    Ok(())
}

/// Example 7: Forecasts with Formula Dependencies
///
/// Demonstrates how forecasted values flow into formula calculations.
fn example_7_forecast_with_formulas() -> Result<()> {
    println!("🔗 Example 7: Forecasts with Formula Dependencies");
    println!("--------------------------------------------------");
    println!("Revenue forecasted (5% growth), COGS = 60% of revenue\n");

    let model = ModelBuilder::new("Forecast + Formulas")
        .periods("2025Q1..Q4", Some("2025Q2"))?
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(1_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(1_100_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .compute("cogs", "revenue * 0.6")?
        .compute("gross_profit", "revenue - cogs")?
        .compute("gross_margin_pct", "gross_profit / revenue * 100")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    println!(
        "{:<10} {:>12} {:>12} {:>12} {:>10}",
        "Period", "Revenue", "COGS", "Gross Profit", "Margin %"
    );
    println!("{}", "-".repeat(64));

    for period in &model.periods {
        let revenue = results.get("revenue", &period.id).unwrap();
        let cogs = results.get("cogs", &period.id).unwrap();
        let gp = results.get("gross_profit", &period.id).unwrap();
        let margin = results.get("gross_margin_pct", &period.id).unwrap();

        println!(
            "{:<10} ${:>11.0} ${:>11.0} ${:>11.0} {:>9.1}%",
            period.id.to_string(),
            revenue,
            cogs,
            gp,
            margin
        );
    }

    println!();
    Ok(())
}

/// Example 8: Complete P&L with Mixed Forecasts
///
/// A realistic P&L model using different forecast methods for different line items.
fn example_8_complete_pl_with_forecasts() -> Result<()> {
    println!("💼 Example 8: Complete P&L with Mixed Forecasts");
    println!("------------------------------------------------");
    println!("Revenue: 5% growth, OpEx: forward fill, Tax Rate: override\n");

    let model = ModelBuilder::new("P&L with Forecasts")
        .periods("2025Q1..2025Q4", Some("2025Q2"))?
        // Revenue: actual + growth forecast
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(10_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(11_000_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        // COGS: formula based on revenue
        .compute("cogs", "revenue * 0.6")?
        // Operating Expenses: actual + forward fill
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(2_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(2_100_000.0),
                ),
            ],
        )
        .forecast(
            "opex",
            ForecastSpec {
                method: ForecastMethod::ForwardFill,
                params: indexmap! {},
            },
        )
        // Calculated line items
        .compute("gross_profit", "revenue - cogs")?
        .compute("operating_income", "gross_profit - opex")?
        // Tax rate: actual + override forecast
        .value(
            "tax_rate",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(0.21)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(0.21)),
            ],
        )
        .forecast(
            "tax_rate",
            ForecastSpec {
                method: ForecastMethod::Override,
                params: indexmap! {
                    "overrides".into() => serde_json::json!({
                        "2025Q3": 0.19,  // Tax rate change
                        "2025Q4": 0.19,
                    })
                },
            },
        )
        .compute("tax_expense", "operating_income * tax_rate")?
        .compute("net_income", "operating_income - tax_expense")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("P&L Statement (in thousands):");
    println!(
        "{:<18} {:>12} {:>12} {:>12} {:>12}",
        "Line Item", "2025Q1", "2025Q2", "2025Q3", "2025Q4"
    );
    println!("{}", "-".repeat(70));

    let line_items = vec![
        ("Revenue", "revenue"),
        ("COGS", "cogs"),
        ("Gross Profit", "gross_profit"),
        ("OpEx", "opex"),
        ("Operating Income", "operating_income"),
        ("Tax Rate", "tax_rate"),
        ("Tax Expense", "tax_expense"),
        ("Net Income", "net_income"),
    ];

    for (label, node_id) in line_items {
        print!("{:<18}", label);
        for q in 1..=4 {
            let value = results.get(node_id, &PeriodId::quarter(2025, q)).unwrap();
            if node_id == "tax_rate" {
                print!(" {:>11.1}%", value * 100.0);
            } else {
                print!(" ${:>10.0}k", value / 1000.0);
            }
        }
        println!();
    }

    println!("\nForecast Methods Used:");
    println!("  - Revenue: 5% compound growth");
    println!("  - OpEx: Forward fill (flat)");
    println!("  - Tax Rate: Override (21% → 19%)");
    println!("  - Other items: Calculated from formulas\n");

    Ok(())
}

/// Example 9: Negative Growth (Declining Revenue)
///
/// Demonstrates declining revenue scenario.
fn example_9_negative_growth() -> Result<()> {
    println!("📉 Example 9: Negative Growth (Declining Revenue)");
    println!("--------------------------------------------------");
    println!("Revenue declining by 10% per quarter\n");

    let model = ModelBuilder::new("Negative Growth")
        .periods("2025Q1..Q4", Some("2025Q1"))?
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(-0.10) }, // -10%
            },
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    let mut prev_value = 100_000.0;
    for period in &model.periods {
        let value = results.get("revenue", &period.id).unwrap();
        let period_type = if period.is_actual {
            "Actual"
        } else {
            "Forecast"
        };
        let change = if period.is_actual {
            0.0
        } else {
            value - prev_value
        };
        println!(
            "  {} ({}): ${:>10.2}  (change: ${:>10.2})",
            period.id, period_type, value, change
        );
        prev_value = value;
    }

    println!();
    Ok(())
}
