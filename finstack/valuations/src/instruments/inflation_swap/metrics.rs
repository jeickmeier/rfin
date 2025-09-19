//! Inflation swap specific metric calculators.
//!
//! Provides metric calculators for inflation swaps including breakeven inflation,
//! fixed leg PV, and inflation leg PV. These metrics are essential for inflation
//! swap valuation and risk management.

use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// Calculates breakeven inflation rate for inflation swaps.
///
/// Computes the fixed rate that makes the swap's present value zero.
/// This represents the market's implied expectation of average inflation
/// over the swap's term.
///
/// Formula: K_BE = (E[I(T_mat)]/I(T_start))^(1/τ) - 1
pub struct BreakevenCalculator;

impl MetricCalculator for BreakevenCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        // Get inflation index for historical reference value
        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_index".to_string(),
                })
            })?;

        // Get inflation curve for forward projection
        let inflation_curve =
            context
                .curves
                .get_ref::<finstack_core::market_data::term_structures::inflation::InflationCurve>(
                    s.inflation_id,
                )?;

        // Historical index value at start (with any lag applied by the index)
        let i_start = inflation_index.value_on(s.start)?;

        // Project inflation index value at maturity
        let t_maturity = DayCount::Act365F
            .year_fraction(
                context.as_of,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity);

        // Year fraction for the full term of the swap
        let tau_accrual = s.dc.year_fraction(
            s.start,
            s.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Breakeven rate: K_BE = (I_mat/I_start)^(1/tau) - 1
        if i_start <= 0.0 || tau_accrual <= 0.0 {
            return Ok(0.0);
        }

        let inflation_ratio = i_maturity_projected / i_start;
        let breakeven = inflation_ratio.powf(1.0 / tau_accrual) - 1.0;

        Ok(breakeven)
    }
}

/// Calculates PV of fixed leg for inflation swaps.
///
/// Computes the present value of the fixed-rate leg by discounting
/// the single payment at maturity using the swap's discount curve.
///
/// Formula: PV_fixed = N * ((1 + K)^τ - 1) * P(as_of, T_mat)
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let pv_fixed = s.pv_fixed_leg(&context.curves, context.as_of)?;
        Ok(pv_fixed.amount())
    }
}

/// Calculates PV of inflation leg for inflation swaps.
///
/// Computes the present value of the inflation-linked leg by discounting
/// the expected inflation payment at maturity using the swap's discount curve.
///
/// Formula: PV_inflation = N * (E[I(T_mat)]/I(T_start) - 1) * P(as_of, T_mat)
pub struct InflationLegPvCalculator;

impl MetricCalculator for InflationLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let pv_inflation = s.pv_inflation_leg(&context.curves, context.as_of)?;
        Ok(pv_inflation.amount())
    }
}

/// Calculates IR01 (1bp nominal interest rate sensitivity) for inflation swaps.
///
/// Computes the change in present value for a 1 basis point parallel shift
/// in nominal interest rates. Uses analytical approximation for efficiency.
///
/// Formula: IR01 ≈ -Duration × PV × 0.0001
pub struct Ir01Calculator;

impl MetricCalculator for Ir01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            s.disc_id,
        )?;
        let base = disc.base_date();

        // Calculate the time to maturity for duration calculation
        let t_maturity = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        // Get current PV
        let pv_fixed = s.pv_fixed_leg(&context.curves, context.as_of)?;
        let pv_inflation = s.pv_inflation_leg(&context.curves, context.as_of)?;
        let total_pv = pv_fixed.amount() + pv_inflation.amount();

        // Analytical approximation: IR01 ≈ -Duration × PV × 1bp
        // For zero-coupon instruments, duration ≈ time to maturity
        let duration = t_maturity;
        let ir01 = -duration * total_pv * 0.0001;

        // Apply direction based on swap side
        let signed_ir01 = match s.side {
            PayReceiveInflation::PayFixed => ir01, // Receive inflation, pay fixed
            PayReceiveInflation::ReceiveFixed => -ir01, // Receive fixed, pay inflation
        };

        Ok(signed_ir01)
    }
}

/// Calculates Inflation01 (1bp inflation rate sensitivity) for inflation swaps.
///
/// Computes the change in present value for a 1 basis point parallel shift
/// in inflation expectations. Uses analytical approximation for efficiency.
///
/// Formula: Inflation01 ≈ (∂PV/∂inflation) × 0.0001
pub struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            s.disc_id,
        )?;
        let base = disc.base_date();

        // Get inflation data for analytical calculation
        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_index".to_string(),
                })
            })?;

        let inflation_curve =
            context
                .curves
                .get_ref::<finstack_core::market_data::term_structures::inflation::InflationCurve>(
                    s.inflation_id,
                )?;

        // Get current inflation values
        let i_start = inflation_index.value_on(s.start)?;
        let t_maturity = DayCount::Act365F
            .year_fraction(
                context.as_of,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity);

        // Calculate discount factor to maturity
        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        // Analytical sensitivity: ∂PV/∂inflation ≈ N × (I_mat/I_start) × DF × (∂ln(I_mat)/∂inflation)
        // For a 1bp shift in inflation: ∂ln(I_mat)/∂inflation ≈ t_maturity × 0.0001
        let inflation_sensitivity =
            s.notional.amount() * (i_maturity_projected / i_start) * df * t_maturity * 0.0001;

        // Apply direction based on swap side
        let signed_sensitivity = match s.side {
            PayReceiveInflation::PayFixed => inflation_sensitivity, // Receive inflation
            PayReceiveInflation::ReceiveFixed => -inflation_sensitivity, // Pay inflation
        };

        Ok(signed_sensitivity)
    }
}

/// Register all inflation swap metrics with the registry
pub fn register_inflation_swap_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::custom("breakeven"),
            Arc::new(BreakevenCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("fixed_leg_pv"),
            Arc::new(FixedLegPvCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("inflation_leg_pv"),
            Arc::new(InflationLegPvCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("ir01"),
            Arc::new(Ir01Calculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("inflation01"),
            Arc::new(Inflation01Calculator),
            &["InflationSwap"],
        );
}
