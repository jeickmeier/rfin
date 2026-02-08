//! YTM (Yield to Maturity) calculator for structured credit.

use crate::instruments::fixed_income::structured_credit::types::constants::YTM_SOLVER_TOLERANCE;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::DayCountCtx;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

/// Calculates YTM (Yield to Maturity) for structured credit.
///
/// YTM is the internal rate of return that equates the present value of
/// all future cashflows to the current price. For structured credit, this
/// is most relevant for fixed-rate tranches.
///
/// # Formula
///
/// Solve for y such that:
/// ```text
/// Σ CF_i / (1 + y)^t_i = Dirty Price
/// ```
///
/// # Compounding Convention
///
/// Uses **annual compounding** by default: discount factor = `(1 + y)^(-t)`.
/// Market-specific conventions may differ:
/// - **ABS**: Often semi-annual (bond-equivalent yield)
/// - **RMBS**: Often monthly (mortgage-equivalent yield)
/// - **CLO**: Often quarterly (matching coupon frequency)
/// - **CMBS**: Often semi-annual
///
/// TODO: Make compounding frequency configurable via deal parameters or a
/// `CompoundingFrequency` field so callers can select the appropriate
/// convention for their asset class.
///
/// # Typical Yield Ranges
///
/// - **ABS (fixed)**: 4-7% typical for AAA
/// - **RMBS (fixed)**: 4-6% typical for agency
/// - **CMBS (fixed)**: 5-7% typical
/// - **CLO (floating)**: Less meaningful (use Z-spread instead)
///
/// # Note
///
/// For structured credit, **Z-spread is generally more important than YTM**
/// because it properly accounts for the term structure of rates.
///
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get dirty price (target value in percentage)
        let dirty_price = context
            .computed
            .get(&MetricId::DirtyPrice)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "metric:DirtyPrice".to_string(),
                })
            })?;

        // Get cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Get notional to convert price to currency
        let base_npv = context.base_value.amount();
        let target_value = base_npv * (dirty_price / 100.0);

        if flows.is_empty() {
            return Ok(0.0);
        }

        // Day count for year fractions
        let day_count = finstack_core::dates::DayCount::Act365F;

        // Objective function: PV(y) - target = 0
        let objective = |y: f64| -> f64 {
            let mut pv = finstack_core::math::summation::NeumaierAccumulator::new();
            for (date, amount) in flows {
                if *date <= context.as_of {
                    continue;
                }

                let t = day_count
                    .year_fraction(context.as_of, *date, DayCountCtx::default())
                    .unwrap_or(0.0);

                if t > 0.0 {
                    let df = (1.0 + y).powf(-t);
                    pv.add(amount.amount() * df);
                }
            }
            pv.total() - target_value
        };

        // Solve for YTM using Brent solver
        // Tolerance: 1e-6 = 0.01 bps precision (market standard)
        let solver = BrentSolver::new().tolerance(YTM_SOLVER_TOLERANCE);

        // Initial guess: 5% is reasonable for structured credit
        let ytm = solver.solve(objective, 0.05)?;

        Ok(ytm)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DirtyPrice]
    }
}
