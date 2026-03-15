//! Shared components for revolving credit pricing.
//!
//! This module contains common pricing components used by both deterministic
//! and stochastic pricing engines to ensure consistent behavior and reduce duplication.

#![allow(dead_code)] // WIP: public API not yet wired into main pricing paths

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::traits::{Discounting, Forward, Survival};
use finstack_core::money::Money;
use finstack_core::types::{Bps, Rate};
use finstack_core::Result;
use std::sync::Arc;

/// Precomputed discount factors for efficient pricing.
///
/// Handles relative discounting from a valuation date and provides
/// consistent discount factor computation across pricing engines.
#[derive(Debug, Clone)]
pub struct DiscountFactors {
    /// Precomputed discount factors aligned with dates
    factors: Vec<f64>,
}

impl DiscountFactors {
    /// Create discount factors from a curve for specific dates.
    ///
    /// Computes discount factors for each date relative to the valuation date.
    /// Handles the case where as_of differs from the curve's base date.
    ///
    /// # Arguments
    ///
    /// * `curve` - Discount curve for factor computation
    /// * `dates` - Dates at which to compute factors
    /// * `as_of` - Valuation date for relative discounting
    pub fn from_curve(curve: &dyn Discounting, dates: &[Date], as_of: Date) -> Result<Self> {
        // Compute discount factors relative to as_of
        let mut factors = Vec::with_capacity(dates.len());
        for &date in dates {
            if date <= as_of {
                // Past or current date - no discounting needed
                factors.push(1.0);
            } else {
                // Future date - discount relative to as_of
                factors.push(curve.df_between_dates(as_of, date).unwrap_or(1.0));
            }
        }

        Ok(Self { factors })
    }

    /// Create discount factors for a time grid (MC engine usage).
    ///
    /// # Arguments
    ///
    /// * `curve` - Discount curve
    /// * `time_points` - Time points in years from a start time
    /// * `start_time` - Start time offset from curve base date
    /// * `day_count` - Day count convention
    /// * `as_of` - Valuation date
    pub fn from_time_grid(
        curve: &dyn Discounting,
        time_points: &[f64],
        start_time: f64,
        day_count: DayCount,
        as_of: Date,
    ) -> Result<Self> {
        let base_date = curve.base_date();

        let t_as_of = day_count.year_fraction(base_date, as_of, DayCountCtx::default())?;

        // Compute discount factors for each time point
        let mut factors = Vec::with_capacity(time_points.len());
        for &t_rel in time_points {
            let t_abs = start_time + t_rel;
            factors.push(curve.df_between_times(t_as_of, t_abs).unwrap_or(1.0));
        }

        Ok(Self { factors })
    }

    /// Get discount factor at a specific index.
    ///
    /// Returns 1.0 if index is out of bounds (conservative default).
    pub fn get(&self, index: usize) -> f64 {
        self.factors.get(index).copied().unwrap_or(1.0)
    }

    /// Get all discount factors.
    pub fn factors(&self) -> &[f64] {
        &self.factors
    }

    /// Get the number of discount factors.
    pub fn len(&self) -> usize {
        self.factors.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.factors.is_empty()
    }
}

/// Precomputed survival probabilities for credit risk adjustment.
///
/// Provides consistent survival probability computation from hazard curves
/// across pricing engines.
#[derive(Debug, Clone)]
pub struct SurvivalWeights {
    /// Survival probabilities aligned with dates/time points
    weights: Vec<f64>,
    /// Recovery rate for credit spread to hazard conversion
    recovery_rate: f64,
}

impl SurvivalWeights {
    /// Create survival weights from a hazard curve for specific dates.
    ///
    /// # Arguments
    ///
    /// * `hazard` - Hazard curve for survival probability computation
    /// * `dates` - Dates at which to compute survival probabilities
    /// * `base_date` - Base date of the hazard curve
    /// * `day_count` - Day count convention to use
    pub fn from_hazard_curve(
        hazard: &dyn Survival,
        dates: &[Date],
        base_date: Date,
        day_count: DayCount,
    ) -> Result<Self> {
        let mut weights = Vec::with_capacity(dates.len());

        for &date in dates {
            let t = day_count.year_fraction(base_date, date, DayCountCtx::default())?;
            let sp = hazard.sp(t).clamp(0.0, 1.0);
            weights.push(sp);
        }

        Ok(Self {
            weights,
            recovery_rate: 0.0, // Not used for static hazard curves
        })
    }

    /// Create constant survival weights (no credit risk).
    ///
    /// Returns a vector of 1.0 values for the specified length.
    pub fn no_credit_risk(len: usize) -> Self {
        Self {
            weights: vec![1.0; len],
            recovery_rate: 0.0,
        }
    }

    /// Get survival weight at a specific index.
    ///
    /// Returns 1.0 if index is out of bounds (no credit risk default).
    pub fn get(&self, index: usize) -> f64 {
        self.weights.get(index).copied().unwrap_or(1.0)
    }

    /// Get all survival weights.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Get recovery rate.
    pub fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }
}

/// Trait for projecting interest rates over periods.
///
/// Provides a unified interface for fixed and floating rate projection
/// used by both deterministic and stochastic pricing engines.
pub trait RateProjector: Send + Sync {
    /// Project the interest rate for a specific period.
    ///
    /// # Arguments
    ///
    /// * `t0` - Start time of the interest period (year fraction)
    /// * `t1` - End time of the interest period (year fraction)
    /// * `step` - Step index (for pre-computed rates)
    fn project_rate(&self, t0: f64, t1: f64, step: usize) -> Result<f64>;

    /// Clone the projector into a boxed trait object.
    fn clone_box(&self) -> Box<dyn RateProjector>;
}

/// Fixed rate projector.
///
/// Always returns the same fixed rate regardless of period.
#[derive(Debug, Clone)]
pub struct FixedRateProjector {
    /// Annual fixed rate
    rate: f64,
}

impl FixedRateProjector {
    /// Create a new fixed rate projector.
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }

    /// Create a new fixed rate projector using a typed rate.
    pub fn new_rate(rate: Rate) -> Self {
        Self {
            rate: rate.as_decimal(),
        }
    }
}

impl RateProjector for FixedRateProjector {
    fn project_rate(&self, _t0: f64, _t1: f64, _step: usize) -> Result<f64> {
        Ok(self.rate)
    }

    fn clone_box(&self) -> Box<dyn RateProjector> {
        Box::new(self.clone())
    }
}

/// Floating rate projector using forward curves.
///
/// Projects rates from a forward curve with margin and optional floor.
#[derive(Clone)]
pub struct FloatingRateProjector {
    /// Forward curve for rate projection
    forward_curve: Arc<dyn Forward + Send + Sync>,
    /// Margin over the index rate (in basis points)
    margin_bp: f64,
    /// Optional floor on the index rate (in basis points)
    floor_bp: Option<f64>,
}

impl FloatingRateProjector {
    /// Create a new floating rate projector.
    pub fn new(
        forward_curve: Arc<dyn Forward + Send + Sync>,
        margin_bp: f64,
        floor_bp: Option<f64>,
    ) -> Self {
        Self {
            forward_curve,
            margin_bp,
            floor_bp,
        }
    }

    /// Create a new floating rate projector using typed basis points.
    pub fn new_bps(
        forward_curve: Arc<dyn Forward + Send + Sync>,
        margin_bp: Bps,
        floor_bp: Option<Bps>,
    ) -> Self {
        Self {
            forward_curve,
            margin_bp: margin_bp.as_bps() as f64,
            floor_bp: floor_bp.map(|bps| bps.as_bps() as f64),
        }
    }

    /// Project rate for a period using forward curve.
    fn project_internal(&self, t0: f64, t1: f64) -> Result<f64> {
        // Get forward rate for the period
        let mut index_rate = self.forward_curve.rate_period(t0, t1);

        // Apply floor if specified (before adding margin)
        if let Some(floor_bp) = self.floor_bp {
            let floor_rate = floor_bp / 10000.0;
            index_rate = index_rate.max(floor_rate);
        }

        // Add margin
        let all_in_rate = index_rate + (self.margin_bp / 10000.0);

        Ok(all_in_rate)
    }
}

impl RateProjector for FloatingRateProjector {
    fn project_rate(&self, t0: f64, t1: f64, _step: usize) -> Result<f64> {
        self.project_internal(t0, t1)
    }

    fn clone_box(&self) -> Box<dyn RateProjector> {
        Box::new(self.clone())
    }
}

/// Pre-computed term-locked rate projector.
///
/// Used for floating rate facilities in Monte Carlo where rates are
/// pre-computed and locked for each period.
#[derive(Debug, Clone)]
pub struct TermLockedRateProjector {
    /// Pre-computed all-in rates by step
    rates_by_step: Vec<f64>,
}

impl TermLockedRateProjector {
    /// Create a new term-locked rate projector.
    pub fn new(rates_by_step: Vec<f64>) -> Self {
        Self { rates_by_step }
    }
}

impl RateProjector for TermLockedRateProjector {
    fn project_rate(&self, _t0: f64, _t1: f64, step: usize) -> Result<f64> {
        Ok(self.rates_by_step.get(step).copied().unwrap_or(0.0))
    }

    fn clone_box(&self) -> Box<dyn RateProjector> {
        Box::new(self.clone())
    }
}

/// Fee calculator for revolving credit facilities.
///
/// Centralizes fee computation logic with support for tiered fee structures.
#[derive(Debug, Clone)]
pub struct FeeCalculator {
    /// Commitment fee in basis points (or tiered structure)
    commitment_fee_bp: f64,
    /// Usage fee in basis points (or tiered structure)
    usage_fee_bp: f64,
    /// Facility fee in basis points (flat, not tiered)
    facility_fee_bp: f64,
}

impl FeeCalculator {
    /// Create a simple fee calculator with flat fees.
    pub fn flat(commitment_fee_bp: f64, usage_fee_bp: f64, facility_fee_bp: f64) -> Self {
        Self {
            commitment_fee_bp,
            usage_fee_bp,
            facility_fee_bp,
        }
    }

    /// Calculate all fees for a period.
    ///
    /// Returns (commitment_fee, usage_fee, facility_fee) tuple.
    pub fn calculate_fees(
        &self,
        drawn_amount: f64,
        commitment_amount: f64,
        accrual_factor: f64,
    ) -> (f64, f64, f64) {
        let undrawn_amount = (commitment_amount - drawn_amount).max(0.0);

        // Commitment fee on undrawn
        let commitment_fee = undrawn_amount * (self.commitment_fee_bp / 10000.0) * accrual_factor;

        // Usage fee on drawn
        let usage_fee = drawn_amount * (self.usage_fee_bp / 10000.0) * accrual_factor;

        // Facility fee on total commitment
        let facility_fee = commitment_amount * (self.facility_fee_bp / 10000.0) * accrual_factor;

        (commitment_fee, usage_fee, facility_fee)
    }
}

/// Helper to compute upfront fee present value.
///
/// Only includes the upfront fee when the commitment date is strictly after the
/// valuation date, consistent with "PV of remaining cashflows" semantics.
/// When `commitment_date <= as_of` the fee has already been paid and is excluded
/// from the mark-to-market valuation.
pub fn compute_upfront_fee_pv(
    upfront_fee_opt: Option<Money>,
    commitment_date: Date,
    as_of: Date,
    disc_curve: &dyn Discounting,
) -> Result<f64> {
    let upfront_fee = match upfront_fee_opt {
        Some(fee) => fee,
        None => return Ok(0.0),
    };

    if commitment_date > as_of {
        let df = disc_curve
            .df_between_dates(as_of, commitment_date)
            .unwrap_or(1.0);
        Ok(upfront_fee.amount() * df)
    } else {
        Ok(0.0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use time::Month;

    #[test]
    fn test_discount_factors_from_curve() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let as_of = base_date;

        let curve = DiscountCurve::builder("TEST")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, 0.97), (2.0, 0.94)])
            .build()
            .expect("should succeed");

        let dates = vec![
            base_date,
            base_date + time::Duration::days(365),
            base_date + time::Duration::days(730),
        ];

        let factors = DiscountFactors::from_curve(&curve, &dates, as_of).expect("should succeed");

        assert_eq!(factors.len(), 3);
        assert!((factors.get(0) - 1.0).abs() < 1e-10);
        assert!((factors.get(1) - 0.97).abs() < 1e-10);
        assert!((factors.get(2) - 0.94).abs() < 1e-10);
    }

    #[test]
    fn test_fixed_rate_projector() {
        let projector = FixedRateProjector::new(0.05);

        let rate = projector
            .project_rate(0.0, 0.25, 0)
            .expect("should succeed");
        assert!((rate - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_fee_calculator() {
        let calculator = FeeCalculator::flat(25.0, 10.0, 5.0);

        let (commitment, usage, facility) = calculator.calculate_fees(
            5_000_000.0,  // drawn
            10_000_000.0, // commitment
            0.25,         // quarter year
        );

        // Commitment fee: 5M * 25bp * 0.25 = 3125
        assert!((commitment - 3125.0).abs() < 1e-6);

        // Usage fee: 5M * 10bp * 0.25 = 1250
        assert!((usage - 1250.0).abs() < 1e-6);

        // Facility fee: 10M * 5bp * 0.25 = 1250
        assert!((facility - 1250.0).abs() < 1e-6);
    }
}
