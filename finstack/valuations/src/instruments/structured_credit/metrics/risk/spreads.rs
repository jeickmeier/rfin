//! Spread calculators for structured credit (Z-spread, CS01, Spread Duration).

use crate::cashflow::traits::DatedFlows;
use crate::constants::ONE_BASIS_POINT;
use crate::instruments::structured_credit::types::constants::{
    Z_SPREAD_INITIAL_BRACKET, Z_SPREAD_SOLVER_TOLERANCE,
};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;
use finstack_core::Result;

// Z-spread bounds in decimal (not basis points)
// -500 bps to allow for premium bonds at tight spreads
const Z_SPREAD_MIN: f64 = -0.05;
// 5000 bps (50%) for distressed credits
const Z_SPREAD_MAX: f64 = 0.50;

/// Calculates Z-spread for structured credit.
///
/// Z-spread (zero-volatility spread) is the constant spread added to the
/// discount curve that equates the present value of cashflows to the market price.
///
/// # Market Standard Definition
///
/// Z-spread is the constant additive spread `z` such that:
/// ```text
/// Σ CF_i × DF(t_i) × exp(-z × t_i) = Market Price
/// ```
///
/// # Returns
///
/// Z-spread in decimal units (e.g., 0.0175 = 175 basis points)
///
/// # Market Conventions
///
/// - **CLO (fixed)**: 150-300 bps typical for AAA
/// - **ABS (fixed)**: 50-150 bps typical for AAA
/// - **RMBS (fixed)**: 100-250 bps typical
/// - **CMBS (fixed)**: 75-200 bps typical
///
pub struct ZSpreadCalculator;

impl MetricCalculator for ZSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get dirty price (target value)
        let dirty_price = context
            .computed
            .get(&MetricId::DirtyPrice)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:DirtyPrice".to_string(),
                })
            })?;

        // Get notional to convert price to currency
        let base_npv = context.base_value.amount();
        let target_value = base_npv * (dirty_price / 100.0);

        // Get cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Get discount curve
        let disc_curve_id = context.discount_curve_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "discount_curve_id".to_string(),
            })
        })?;

        let disc = context.curves.get_discount_ref(disc_curve_id.as_str())?;
        let base_date = disc.base_date();
        let day_count = finstack_core::dates::DayCount::Act365F;

        // Pre-compute (t, df, amount) for deterministic, fallible date handling.
        let cached_flows: Vec<(f64, f64, f64)> = flows
            .iter()
            .filter(|(date, _)| *date > context.as_of)
            .map(|(date, amount)| -> finstack_core::Result<(f64, f64, f64)> {
                let t = day_count.year_fraction(base_date, *date, DayCountCtx::default())?;
                let df = disc.try_df_on_date_curve(*date)?;
                Ok((t, df, amount.amount()))
            })
            .collect::<finstack_core::Result<Vec<_>>>()?;

        // Objective function: PV(z) - target = 0
        let objective = |z: f64| -> f64 {
            let mut pv = 0.0;
            for (t, df, amt) in &cached_flows {
                let df_z = df * (-z * t).exp();
                pv += amt * df_z;
            }
            pv - target_value
        };

        // Solve for z-spread using Brent's method with adaptive bracketing
        //
        // Credit spread characteristics:
        // - Investment grade: 50-300 bps (0.005-0.03)
        // - High yield: 300-1000 bps (0.03-0.10)
        // - Distressed: 1000+ bps (0.10+)
        // - Premium bonds may have negative Z-spread
        //
        // We start with a moderate bracket and allow expansion for edge cases.
        // Tolerance: 1e-6 = 0.01 bps precision (market standard)
        let solver = BrentSolver::new()
            .with_tolerance(Z_SPREAD_SOLVER_TOLERANCE)
            .with_initial_bracket_size(Some(Z_SPREAD_INITIAL_BRACKET));

        let valid_range = Z_SPREAD_MIN..=Z_SPREAD_MAX;

        // Try solving with standard initial guess
        match solver.solve(objective, 0.01) {
            Ok(z) if valid_range.contains(&z) => Ok(z),
            _ => {
                // Adaptive retry: try with a different initial guess
                // For distressed credits, start higher
                let z_high_guess = solver.solve(objective, 0.10);
                if let Ok(z) = z_high_guess {
                    if valid_range.contains(&z) {
                        return Ok(z);
                    }
                }

                // For premium bonds, try negative initial guess
                let z_low_guess = solver.solve(objective, -0.01);
                if let Ok(z) = z_low_guess {
                    if valid_range.contains(&z) {
                        return Ok(z);
                    }
                }

                // Final fallback: wider bracket with explicit bounds
                let wide_solver = BrentSolver::new()
                    .with_tolerance(Z_SPREAD_SOLVER_TOLERANCE)
                    .with_initial_bracket_size(Some(0.20)); // ±2000 bps

                wide_solver.solve(objective, 0.05)
            }
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DirtyPrice]
    }
}

/// Calculates CS01 (credit spread DV01) for structured credit.
///
/// CS01 measures the dollar change in value for a 1 basis point parallel shift
/// in the credit spread. This is **THE PRIMARY RISK METRIC** for structured credit.
///
/// # Formula
///
/// CS01 = -(PV_bumped - PV_base) where spread is bumped by 1bp
///
/// # Market Conventions
///
/// - **CLO AAA**: $0.30-$0.50 per $100 face (30-50 DV01)
/// - **ABS AAA**: $2-$6 per $100 face
/// - **RMBS AAA**: $3-$8 per $100 face
/// - **CMBS AAA**: $4-$8 per $100 face
///
/// # Key Insight
///
/// For **floating-rate CLO**: CS01 >> DV01 (spread risk dominates IR risk)
///
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get base NPV
        let base_npv = context.base_value.amount();

        // Get Z-spread (base spread)
        let base_spread = context
            .computed
            .get(&MetricId::ZSpread)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:ZSpread".to_string(),
                })
            })?;

        // Bump spread by 1bp
        let bumped_spread = base_spread + ONE_BASIS_POINT;

        // Get cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Get discount curve
        let disc_curve_id = context.discount_curve_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "discount_curve_id".to_string(),
            })
        })?;

        let disc = context.curves.get_discount_ref(disc_curve_id.as_str())?;
        let base_date = disc.base_date();
        let day_count = finstack_core::dates::DayCount::Act365F;

        // Calculate PV with bumped spread
        let mut bumped_npv = 0.0;
        for (date, amount) in flows {
            if *date <= context.as_of {
                continue;
            }

            let t = day_count.year_fraction(base_date, *date, DayCountCtx::default())?;

            let df = disc.try_df_on_date_curve(*date)?;
            let df_bumped = df * (-bumped_spread * t).exp();

            bumped_npv += amount.amount() * df_bumped;
        }

        // CS01 = -(PV_bumped - PV_base)
        // Negative sign because price decreases when spread increases
        let cs01 = -(bumped_npv - base_npv);

        Ok(cs01)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::ZSpread]
    }
}

/// Calculates spread duration for structured credit.
///
/// Spread duration measures the percentage change in price for a 1% change in spread,
/// expressed in years. This converts CS01 into a duration-like metric.
///
/// # Formula
///
/// Spread Duration = CS01 / (Price × 0.0001)
///
/// Or approximately: CS01 / (NPV × 1bp)
///
/// # Interpretation
///
/// - **CLO AAA (floating)**: 0.3-0.5 years (low spread duration)
/// - **ABS (fixed)**: 2-4 years
/// - **RMBS (fixed)**: 3-7 years (varies with prepayments)
/// - **CMBS (fixed)**: 4-8 years (close to modified duration)
///
/// # Key Insight
///
/// For fixed-rate structures, spread duration ≈ modified duration.
/// For floating-rate (CLO), spread duration >> IR duration.
///
pub struct SpreadDurationCalculator;

impl MetricCalculator for SpreadDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get CS01
        let cs01 = context
            .computed
            .get(&MetricId::Cs01)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Cs01".to_string(),
                })
            })?;

        // Note: We use base_npv directly instead of dirty_price for spread duration
        // since we're measuring dollar value change, not percentage change

        // Get base NPV
        let base_npv = context.base_value.amount();

        if base_npv == 0.0 {
            return Ok(0.0);
        }

        // Spread duration = CS01 / (Price × 1bp)
        // This gives duration in years
        let spread_duration = cs01 / (base_npv * ONE_BASIS_POINT);

        Ok(spread_duration)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Cs01, MetricId::DirtyPrice]
    }
}

/// Calculate tranche-specific Z-spread in basis points.
///
/// Z-spread (zero-volatility spread) is the constant spread added to the
/// discount curve that equates the present value of cashflows to the market price.
///
/// # Arguments
///
/// * `cashflows` - The dated cashflows for the tranche
/// * `discount_curve` - The discount curve for PV calculation
/// * `target_pv` - The target present value to solve for
/// * `as_of` - The valuation date
///
/// # Returns
///
/// Z-spread in basis points
pub fn calculate_tranche_z_spread(
    cashflows: &DatedFlows,
    discount_curve: &DiscountCurve,
    target_pv: Money,
    as_of: Date,
) -> Result<f64> {
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

    // Tolerance: 1e-6 = 0.01 bps precision (market standard)
    let solver = BrentSolver::new()
        .with_tolerance(Z_SPREAD_SOLVER_TOLERANCE)
        .with_initial_bracket_size(Some(Z_SPREAD_INITIAL_BRACKET));

    let z_spread = solver.solve(objective, 0.0)?;

    // Convert to basis points
    Ok(z_spread * 10_000.0)
}

/// Calculate tranche-specific CS01 (credit spread sensitivity).
///
/// CS01 measures the dollar change in value for a 1 basis point parallel shift
/// in the credit spread.
///
/// # Arguments
///
/// * `cashflows` - The dated cashflows for the tranche
/// * `discount_curve` - The discount curve for PV calculation
/// * `z_spread` - The Z-spread in decimal (not basis points)
/// * `as_of` - The valuation date
///
/// # Returns
///
/// CS01 in currency units (dollar value change per 1bp spread increase)
pub fn calculate_tranche_cs01(
    cashflows: &DatedFlows,
    discount_curve: &DiscountCurve,
    z_spread: f64,
    as_of: Date,
) -> Result<f64> {
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
