//! Private markets fund metrics: IRR, MOIC, DPI, TVPI, carry calculations, and theta.

use crate::instruments::private_markets_fund::PrivateMarketsFund;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};
use finstack_core::dates::{Date, DayCount};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;

/// LP Internal Rate of Return calculator.
pub struct LpIrrCalculator;

impl MetricCalculator for LpIrrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;
        let ledger = pe.run_waterfall()?;
        let lp_flows = ledger.lp_cashflows();

        if lp_flows.len() < 2 {
            return Ok(0.0);
        }

        calculate_irr(&lp_flows, pe.spec.irr_basis)
    }
}

/// GP Internal Rate of Return calculator.
pub struct GpIrrCalculator;

impl MetricCalculator for GpIrrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;
        let ledger = pe.run_waterfall()?;

        // Extract GP cashflows (carry distributions)
        let mut gp_flows = Vec::new();
        let mut gp_flows_by_date: indexmap::IndexMap<Date, Money> = indexmap::IndexMap::new();

        for row in &ledger.rows {
            if row.to_gp.amount().abs() > 1e-6 {
                let existing = gp_flows_by_date
                    .get(&row.date)
                    .copied()
                    .unwrap_or_else(|| Money::new(0.0, row.to_gp.currency()));

                if let Ok(new_amount) = existing + row.to_gp {
                    gp_flows_by_date.insert(row.date, new_amount);
                }
            }
        }

        for (date, amount) in gp_flows_by_date {
            gp_flows.push((date, amount));
        }

        if gp_flows.is_empty() {
            return Ok(0.0); // No GP distributions
        }

        // GP has no initial investment, so IRR is based on distributions only
        // For GP, we calculate the return on carry basis (not meaningful as traditional IRR)
        // Return total GP carry as a percentage of total fund proceeds
        let total_gp_carry: f64 = gp_flows.iter().map(|(_, amount)| amount.amount()).sum();
        Ok(total_gp_carry)
    }
}

/// Multiple on Invested Capital (MOIC) for LP calculator.
pub struct MoicLpCalculator;

impl MetricCalculator for MoicLpCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;

        let total_contributions: f64 = pe
            .events
            .iter()
            .filter(|e| {
                e.kind == crate::instruments::private_markets_fund::FundEventKind::Contribution
            })
            .map(|e| e.amount.amount())
            .sum();

        let total_distributions: f64 = pe
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    crate::instruments::private_markets_fund::FundEventKind::Distribution
                        | crate::instruments::private_markets_fund::FundEventKind::Proceeds
                )
            })
            .map(|e| e.amount.amount())
            .sum();

        if total_contributions <= 1e-6 {
            return Ok(0.0);
        }

        Ok(total_distributions / total_contributions)
    }
}

/// Distributions to Paid-In Capital (DPI) calculator.
pub struct DpiLpCalculator;

impl MetricCalculator for DpiLpCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;
        let ledger = pe.run_waterfall()?;

        let total_contributions: f64 = pe
            .events
            .iter()
            .filter(|e| {
                e.kind == crate::instruments::private_markets_fund::FundEventKind::Contribution
            })
            .map(|e| e.amount.amount())
            .sum();

        let total_lp_distributions: f64 = ledger.rows.iter().map(|r| r.to_lp.amount()).sum();

        if total_contributions <= 1e-6 {
            return Ok(0.0);
        }

        Ok(total_lp_distributions / total_contributions)
    }
}

/// Total Value to Paid-In Capital (TVPI) calculator.
pub struct TvpiLpCalculator;

impl MetricCalculator for TvpiLpCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;
        let ledger = pe.run_waterfall()?;

        let total_contributions: f64 = pe
            .events
            .iter()
            .filter(|e| {
                e.kind == crate::instruments::private_markets_fund::FundEventKind::Contribution
            })
            .map(|e| e.amount.amount())
            .sum();

        // TVPI = (Distributions + Residual NAV) / Contributions
        // For simplicity, assume residual NAV = unreturned capital
        let total_lp_distributions: f64 = ledger.rows.iter().map(|r| r.to_lp.amount()).sum();

        let residual_nav = ledger
            .rows
            .last()
            .map(|r| r.lp_unreturned.amount())
            .unwrap_or(0.0);

        if total_contributions <= 1e-6 {
            return Ok(0.0);
        }

        Ok((total_lp_distributions + residual_nav) / total_contributions)
    }
}

/// GP carry accrued calculator.
pub struct CarryAccruedCalculator;

impl MetricCalculator for CarryAccruedCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let pe: &PrivateMarketsFund = context.instrument_as()?;
        let ledger = pe.run_waterfall()?;

        // Return final GP carry cumulative amount
        Ok(ledger
            .rows
            .last()
            .map(|r| r.gp_carry_cum.amount())
            .unwrap_or(0.0))
    }
}

/// Helper function to calculate IRR using robust root finding.
pub fn calculate_irr(flows: &[(Date, Money)], day_count: DayCount) -> finstack_core::Result<f64> {
    if flows.len() < 2 {
        return Err(finstack_core::error::InputError::TooFewPoints.into());
    }

    let base_date = flows[0].0;

    let npv_function = |rate: f64| -> f64 {
        let mut npv = 0.0;
        for (date, amount) in flows {
            let t = day_count
                .year_fraction(
                    base_date,
                    *date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let df = if rate.abs() < 1e-10 {
                1.0 - rate * t // Linear approximation for rates near zero
            } else {
                (1.0 + rate).powf(-t)
            };
            npv += amount.amount() * df;
        }
        npv
    };

    // Use BrentSolver with reasonable bounds for PE returns
    let solver = BrentSolver::new()
        .with_tolerance(1e-12)
        .with_initial_bracket_size(Some(1.0)); // Start with reasonable IRR range

    solver
        .solve(npv_function, 0.15) // Start with 15% initial guess for PE returns
        .map_err(|_| finstack_core::error::InputError::Invalid.into())
}

mod carry01;
mod hurdle01;
mod nav01;

/// Register all private markets fund metrics.
pub fn register_private_markets_fund_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Private markets fund-specific risk metrics (custom metrics)
    registry.register_metric(
        MetricId::Nav01,
        Arc::new(nav01::Nav01Calculator),
        &["PrivateMarketsFund"],
    );
    registry.register_metric(
        MetricId::Carry01,
        Arc::new(carry01::Carry01Calculator),
        &["PrivateMarketsFund"],
    );
    registry.register_metric(
        MetricId::Hurdle01,
        Arc::new(hurdle01::Hurdle01Calculator),
        &["PrivateMarketsFund"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "PrivateMarketsFund",
        metrics: [
            (LpIrr, LpIrrCalculator),
            (GpIrr, GpIrrCalculator),
            (MoicLp, MoicLpCalculator),
            (DpiLp, DpiLpCalculator),
            (TvpiLp, TvpiLpCalculator),
            (CarryAccrued, CarryAccruedCalculator),
            (Theta, ThetaCalculator),
        ]
    }
}

/// Theta calculator for private markets fund time decay
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        crate::metrics::generic_theta_calculator::<PrivateMarketsFund>(context)
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::private_markets_fund::{FundEvent, WaterfallSpec};
    use time::Month;

    fn test_currency() -> finstack_core::currency::Currency {
        finstack_core::currency::Currency::USD
    }

    fn test_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid date"), day).expect("should succeed")
    }

    #[test]
    fn test_irr_calculation() {
        // Simple 2x return over 5 years should be ~15% IRR
        let flows = vec![
            (
                test_date(2020, 1, 1),
                Money::new(-1000000.0, test_currency()),
            ), // Contribution
            (
                test_date(2025, 1, 1),
                Money::new(2000000.0, test_currency()),
            ), // Distribution
        ];

        let irr = calculate_irr(&flows, DayCount::Act365F).expect("should succeed");

        // 2x over 5 years = (2.0)^(1/5) - 1 ≈ 0.1487 or ~14.87%
        assert!(
            (irr - 0.1487).abs() < 0.01,
            "Expected ~14.87% IRR, got {:.4}%",
            irr * 100.0
        );
    }

    #[test]
    fn test_moic_calculation() {
        let spec = WaterfallSpec::builder()
            .return_of_capital()
            .build()
            .expect("should succeed");

        let events = vec![
            FundEvent::contribution(
                test_date(2020, 1, 1),
                Money::new(1000000.0, test_currency()),
            ),
            FundEvent::distribution(
                test_date(2025, 1, 1),
                Money::new(2000000.0, test_currency()),
            ),
        ];

        let pe = PrivateMarketsFund::new("TEST", test_currency(), spec, events);

        let curves = finstack_core::market_data::MarketContext::new();
        let base_value = Money::new(2000000.0, test_currency());
        let mut context = MetricContext::new(
            std::sync::Arc::new(pe),
            std::sync::Arc::new(curves),
            test_date(2025, 1, 1),
            base_value,
        );

        let moic = MoicLpCalculator.calculate(&mut context).expect("should succeed");
        assert!((moic - 2.0).abs() < 1e-6); // 2x multiple
    }
}
