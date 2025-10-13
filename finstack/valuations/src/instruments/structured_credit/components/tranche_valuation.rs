//! Tranche-specific valuation and metrics for structured credit instruments.
//!
//! This module provides functionality to value and calculate metrics for individual
//! tranches within structured credit instruments (CLO, ABS, RMBS, CMBS).

use crate::cashflow::traits::DatedFlows;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::cashflow::CashFlow;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

/// Result containing tranche-specific cashflows and metadata
#[derive(Debug, Clone)]
pub struct TrancheCashflowResult {
    /// Tranche identifier
    pub tranche_id: String,
    /// Cashflow schedule for this tranche (simple dated flows for backward compatibility)
    pub cashflows: DatedFlows,
    /// Detailed cashflows with proper classification using CFKind
    pub detailed_flows: Vec<CashFlow>,
    /// Interest cashflows (component of total)
    pub interest_flows: DatedFlows,
    /// Principal cashflows (component of total)  
    pub principal_flows: DatedFlows,
    /// PIK capitalization flows (using CFKind::PIK)
    pub pik_flows: DatedFlows,
    /// Final tranche balance after all payments
    pub final_balance: Money,
    /// Total interest received
    pub total_interest: Money,
    /// Total principal received
    pub total_principal: Money,
    /// Total PIK capitalized
    pub total_pik: Money,
}

/// Tranche-specific valuation result
#[derive(Debug, Clone)]
pub struct TrancheValuation {
    /// Tranche identifier
    pub tranche_id: String,
    /// Present value of all cashflows
    pub pv: Money,
    /// Clean price (as percentage of par)
    pub clean_price: f64,
    /// Dirty price (as percentage of par)
    pub dirty_price: f64,
    /// Accrued interest
    pub accrued: Money,
    /// Weighted average life
    pub wal: f64,
    /// Modified duration
    pub modified_duration: f64,
    /// Z-spread (basis points)
    pub z_spread_bps: f64,
    /// CS01 (credit DV01)
    pub cs01: f64,
    /// Yield to maturity
    pub ytm: f64,
    /// Additional metrics
    pub metrics: HashMap<MetricId, f64>,
}

/// Extension trait for tranche-specific valuation
pub trait TrancheValuationExt {
    /// Generate cashflows for a specific tranche after waterfall allocation
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<TrancheCashflowResult>;

    /// Calculate present value for a specific tranche
    fn value_tranche(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money>;

    /// Get full valuation with metrics for a specific tranche
    fn value_tranche_with_metrics(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<TrancheValuation>;
}

/// Calculate tranche-specific WAL
pub fn calculate_tranche_wal(cashflows: &TrancheCashflowResult, as_of: Date) -> Result<f64> {
    let mut weighted_sum = 0.0;
    let mut total_principal = 0.0;

    for (date, amount) in &cashflows.principal_flows {
        if *date <= as_of {
            continue;
        }

        let years = finstack_core::dates::DayCount::Act365F
            .year_fraction(as_of, *date, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        weighted_sum += amount.amount() * years;
        total_principal += amount.amount();
    }

    if total_principal > 0.0 {
        Ok(weighted_sum / total_principal)
    } else {
        Ok(0.0)
    }
}

/// Calculate tranche-specific modified duration
pub fn calculate_tranche_duration(
    cashflows: &DatedFlows,
    discount_curve: &DiscountCurve,
    as_of: Date,
    pv: Money,
) -> Result<f64> {
    use finstack_core::dates::DayCount;

    let day_count = DayCount::Act365F;
    let mut weighted_pv = 0.0;

    // Pre-compute as_of discount factor for correct theta
    let disc_dc = discount_curve.day_count();
    let t_as_of = disc_dc
        .year_fraction(discount_curve.base_date(), as_of, DayCountCtx::default())
        .unwrap_or(0.0);
    let df_as_of = discount_curve.df(t_as_of);

    for (date, amount) in cashflows {
        if *date <= as_of {
            continue;
        }

        let years = day_count
            .year_fraction(as_of, *date, DayCountCtx::default())
            .unwrap_or(0.0);

        // Discount from as_of
        let t_cf = disc_dc
            .year_fraction(discount_curve.base_date(), *date, DayCountCtx::default())
            .unwrap_or(0.0);
        let df_cf_abs = discount_curve.df(t_cf);
        let df = if df_as_of != 0.0 {
            df_cf_abs / df_as_of
        } else {
            1.0
        };
        let flow_pv = amount.amount() * df;

        weighted_pv += flow_pv * years;
    }

    if pv.amount() > 0.0 {
        Ok(weighted_pv / pv.amount())
    } else {
        Ok(0.0)
    }
}

/// Calculate tranche-specific Z-spread in basis points
pub fn calculate_tranche_z_spread(
    cashflows: &DatedFlows,
    discount_curve: &DiscountCurve,
    target_pv: Money,
    as_of: Date,
) -> Result<f64> {
    use finstack_core::dates::DayCount;
    use finstack_core::math::solver::{BrentSolver, Solver};

    let day_count = DayCount::Act365F;
    let base_date = discount_curve.base_date();

    // Pre-compute as_of discount factor for correct theta
    let disc_dc = discount_curve.day_count();
    let t_as_of_val = disc_dc
        .year_fraction(base_date, as_of, DayCountCtx::default())
        .unwrap_or(0.0);
    let df_as_of_val = discount_curve.df(t_as_of_val);

    let objective = |z: f64| -> f64 {
        let mut pv = 0.0;
        for (date, amount) in cashflows {
            if *date <= as_of {
                continue;
            }

            let t_from_as_of = day_count
                .year_fraction(as_of, *date, DayCountCtx::default())
                .unwrap_or(0.0);

            // Discount from as_of
            let t_cf = disc_dc
                .year_fraction(base_date, *date, DayCountCtx::default())
                .unwrap_or(0.0);
            let df_cf_abs = discount_curve.df(t_cf);
            let df = if df_as_of_val != 0.0 {
                df_cf_abs / df_as_of_val
            } else {
                1.0
            };
            let df_z = df * (-z * t_from_as_of).exp();

            pv += amount.amount() * df_z;
        }
        pv - target_pv.amount()
    };

    let solver = BrentSolver::new()
        .with_tolerance(1e-8)
        .with_initial_bracket_size(Some(0.5));

    let z_spread = solver.solve(objective, 0.0)?;

    // Convert to basis points
    Ok(z_spread * 10_000.0)
}

/// Calculate tranche-specific CS01
pub fn calculate_tranche_cs01(
    cashflows: &DatedFlows,
    discount_curve: &DiscountCurve,
    z_spread: f64,
    as_of: Date,
) -> Result<f64> {
    use crate::constants::ONE_BASIS_POINT;
    use finstack_core::dates::DayCount;

    let day_count = DayCount::Act365F;
    let base_date = discount_curve.base_date();

    // Pre-compute as_of discount factor for correct theta
    let disc_dc = discount_curve.day_count();
    let t_as_of_val = disc_dc
        .year_fraction(base_date, as_of, DayCountCtx::default())
        .unwrap_or(0.0);
    let df_as_of_val = discount_curve.df(t_as_of_val);

    // Calculate base PV
    let mut base_pv = 0.0;
    let mut bumped_pv = 0.0;
    let bumped_spread = z_spread + ONE_BASIS_POINT;

    for (date, amount) in cashflows {
        if *date <= as_of {
            continue;
        }

        let t_from_as_of = day_count
            .year_fraction(as_of, *date, DayCountCtx::default())
            .unwrap_or(0.0);

        // Discount from as_of
        let t_cf = disc_dc
            .year_fraction(base_date, *date, DayCountCtx::default())
            .unwrap_or(0.0);
        let df_cf_abs = discount_curve.df(t_cf);
        let df = if df_as_of_val != 0.0 {
            df_cf_abs / df_as_of_val
        } else {
            1.0
        };

        // Base PV
        let df_base = df * (-z_spread * t_from_as_of).exp();
        base_pv += amount.amount() * df_base;

        // Bumped PV
        let df_bumped = df * (-bumped_spread * t_from_as_of).exp();
        bumped_pv += amount.amount() * df_bumped;
    }

    // CS01 = -(PV_bumped - PV_base)
    Ok(-(bumped_pv - base_pv))
}

/// Tranche-specific metric calculator wrapper
pub struct TrancheMetricCalculator {
    pub base_calculator: Box<dyn MetricCalculator>,
    pub tranche_id: String,
}

impl MetricCalculator for TrancheMetricCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Set context to use tranche-specific cashflows
        // This would require modifying MetricContext to support tranche filtering
        self.base_calculator.calculate(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        self.base_calculator.dependencies()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_tranche_wal_calculation() {
        let cashflows = TrancheCashflowResult {
            tranche_id: "AAA".to_string(),
            cashflows: vec![],
            detailed_flows: vec![],
            interest_flows: vec![],
            principal_flows: vec![
                (
                    Date::from_calendar_date(2024, time::Month::June, 30).unwrap(),
                    Money::new(100_000.0, Currency::USD),
                ),
                (
                    Date::from_calendar_date(2025, time::Month::June, 30).unwrap(),
                    Money::new(100_000.0, Currency::USD),
                ),
            ],
            pik_flows: vec![],
            final_balance: Money::new(0.0, Currency::USD),
            total_interest: Money::new(10_000.0, Currency::USD),
            total_principal: Money::new(200_000.0, Currency::USD),
            total_pik: Money::new(0.0, Currency::USD),
        };

        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
        let wal = calculate_tranche_wal(&cashflows, as_of).unwrap();

        // Should be approximately 1.0 year (average of 0.5 and 1.5 years)
        assert!((wal - 1.0).abs() < 0.1);
    }
}
