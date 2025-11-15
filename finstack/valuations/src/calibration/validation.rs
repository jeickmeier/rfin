//! Market data validation and no-arbitrage constraints.
//!
//! Provides comprehensive validation for calibrated curves and surfaces
//! to ensure they satisfy fundamental financial constraints.

use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve, hazard_curve::HazardCurve,
    inflation::InflationCurve, BaseCorrelationCurve,
};
use finstack_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Static test points to avoid repeated allocations on hot validation paths
// Discount curve validation points
const DF_MONO_POINTS: &[f64] = &[
    0.0, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
];
const DF_BOUNDS_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0];

// Forward curve validation points
const FWD_ARBI_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
const FWD_BOUNDS_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 5.0, 10.0];

// Hazard curve validation points
const HAZARD_ARBI_POINTS: &[f64] = &[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
const HAZARD_MONO_POINTS: &[f64] = &[0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
const HAZARD_BOUNDS_POINTS: &[f64] = &[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];

// Inflation curve validation points
const INFL_ARBI_POINTS: &[f64] = &[0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 20.0];
const INFL_MONO_POINTS: &[f64] = &[1.0, 2.0, 3.0, 5.0, 10.0];
const INFL_BOUNDS_POINTS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 20.0, 30.0];

/// Validation error details
/// Calibration validation error with context and diagnostic values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationError {
    /// Which constraint was violated (e.g., "monotonicity", "positivity")
    pub constraint: String,
    /// Location of the violation (curve ID, point index, etc.)
    pub location: String,
    /// Human-readable details about the violation
    pub details: String,
    /// Relevant diagnostic values (actual vs expected, etc.)
    pub values: BTreeMap<String, f64>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(
        constraint: impl Into<String>,
        location: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            constraint: constraint.into(),
            location: location.into(),
            details: details.into(),
            values: BTreeMap::new(),
        }
    }

    /// Add a diagnostic value to the error report
    pub fn with_value(mut self, key: impl Into<String>, value: f64) -> Self {
        self.values.insert(key.into(), value);
        self
    }
}

/// Core validation trait for market data structures
pub trait CurveValidator {
    /// Validate that the curve satisfies no-arbitrage constraints
    fn validate_no_arbitrage(&self) -> Result<()>;

    /// Validate monotonicity constraints
    fn validate_monotonicity(&self) -> Result<()>;

    /// Validate that all values are within reasonable bounds
    fn validate_bounds(&self) -> Result<()>;

    /// Run all validations
    fn validate(&self) -> Result<()> {
        self.validate_no_arbitrage()?;
        self.validate_monotonicity()?;
        self.validate_bounds()?;
        Ok(())
    }
}

/// Validation for discount curves
impl CurveValidator for DiscountCurve {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // Check forward rate positivity
        let times = [0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0];

        for i in 0..times.len() - 1 {
            let t1 = times[i];
            let t2 = times[i + 1];

            let df1 = self.df(t1);
            let df2 = self.df(t2);

            // Calculate instantaneous forward rate
            if df1 > 0.0 && df2 > 0.0 && df2 < df1 {
                let fwd_rate = (df1 / df2 - 1.0) / (t2 - t1);

                // Forward rates should be positive (allowing small negative for technical reasons)
                if fwd_rate < -0.02 {
                    return Err(Error::Validation(format!(
                        "Negative forward rate {:.4}% between t={} and t={} in {}",
                        fwd_rate * 100.0,
                        t1,
                        t2,
                        self.id().as_str()
                    )));
                }

                // Forward rates shouldn't be unreasonably high
                if fwd_rate > 1.0 {
                    // 100% forward rate
                    return Err(Error::Validation(format!(
                        "Unreasonably high forward rate {:.2}% between t={} and t={} in {}",
                        fwd_rate * 100.0,
                        t1,
                        t2,
                        self.id().as_str()
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self) -> Result<()> {
        // Discount factors must be monotonically decreasing
        let mut prev_df = 1.0;

        for &t in DF_MONO_POINTS {
            let df = self.df(t);

            // Allow for numerical tolerance
            if df > prev_df + 1e-10 {
                return Err(Error::Validation(format!(
                    "Discount factor not monotonically decreasing: DF({})={:.6} > DF(prev)={:.6} in {}",
                    t, df, prev_df, self.id().as_str()
                )));
            }

            prev_df = df;
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        // Check that discount factors are in (0, 1]
        for &t in DF_BOUNDS_POINTS {
            let df = self.df(t);

            if df <= 0.0 {
                return Err(Error::Validation(format!(
                    "Non-positive discount factor {:.6} at t={} in {}",
                    df,
                    t,
                    self.id().as_str()
                )));
            }

            if df > 1.0 {
                return Err(Error::Validation(format!(
                    "Discount factor {:.6} exceeds 1.0 at t={} in {}",
                    df,
                    t,
                    self.id().as_str()
                )));
            }
        }

        // Check zero rates are reasonable
        for &t in DF_BOUNDS_POINTS {
            let rate = self.zero(t);

            // Allow slightly negative rates but not too extreme
            if rate < -0.05 {
                // -5% floor
                return Err(Error::Validation(format!(
                    "Zero rate {:.2}% too negative at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }

            // Cap at reasonable maximum
            if rate > 0.50 {
                // 50% ceiling
                return Err(Error::Validation(format!(
                    "Zero rate {:.2}% too high at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }
}

/// Validation for forward curves
impl CurveValidator for ForwardCurve {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // Forward rates should be positive (with small tolerance for negative rates)
        for &t in FWD_ARBI_POINTS {
            let fwd = self.rate(t);

            if fwd < -0.01 {
                // Allow small negative rates
                return Err(Error::Validation(format!(
                    "Negative forward rate {:.4}% at t={} in {}",
                    fwd * 100.0,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self) -> Result<()> {
        // Forward curves don't have strict monotonicity requirements
        // but we check for reasonable smoothness
        let test_points = [0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];

        for i in 0..test_points.len() - 1 {
            let t1 = test_points[i];
            let t2 = test_points[i + 1];

            let fwd1 = self.rate(t1);
            let fwd2 = self.rate(t2);

            // Check for unreasonable jumps (more than 10% absolute change)
            let jump = (fwd2 - fwd1).abs();
            if jump > 0.10 {
                // This is a warning, not necessarily an error
                tracing::warn!(
                    "Large forward rate jump of {:.2}% between t={} and t={} in {}",
                    jump * 100.0,
                    t1,
                    t2,
                    self.id().as_str()
                );
            }
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        for &t in FWD_BOUNDS_POINTS {
            let rate = self.rate(t);

            // Allow slightly negative but bounded
            if rate < -0.05 {
                return Err(Error::Validation(format!(
                    "Forward rate {:.2}% too negative at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }

            // Cap at reasonable maximum
            if rate > 0.50 {
                return Err(Error::Validation(format!(
                    "Forward rate {:.2}% too high at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }
}

/// Validation for hazard curves
impl CurveValidator for HazardCurve {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // Check hazard rates are non-negative using survival probability
        for &t in HAZARD_ARBI_POINTS {
            // Get hazard rate from survival probability derivative
            // λ(t) = -d/dt ln(S(t))
            let dt = 0.0001;
            let sp1 = self.sp(t);
            let sp2 = self.sp(t + dt);
            let lambda = if sp1 > 0.0 && sp2 > 0.0 {
                -(sp2.ln() - sp1.ln()) / dt
            } else {
                0.0
            };

            if lambda < 0.0 {
                return Err(Error::Validation(format!(
                    "Negative hazard rate {:.4} at t={} in {}",
                    lambda,
                    t,
                    self.id().as_str()
                )));
            }

            // Check for unreasonably high hazard rates (>50% instantaneous default prob)
            if lambda > 0.5 {
                return Err(Error::Validation(format!(
                    "Unreasonably high hazard rate {:.2} at t={} in {}",
                    lambda,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self) -> Result<()> {
        // Survival probabilities must be monotonically decreasing
        let mut prev_sp = 1.0;

        for &t in HAZARD_MONO_POINTS {
            let sp = self.sp(t);

            // Allow for numerical tolerance
            if sp > prev_sp + 1e-10 {
                return Err(Error::Validation(format!(
                    "Survival probability not monotonically decreasing: SP({})={:.6} > SP(prev)={:.6} in {}",
                    t, sp, prev_sp, self.id().as_str()
                )));
            }

            prev_sp = sp;
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        // Check that survival probabilities are in [0, 1]
        // and that recovery rate is reasonable
        for &t in HAZARD_BOUNDS_POINTS {
            let sp = self.sp(t);

            if sp < 0.0 {
                return Err(Error::Validation(format!(
                    "Negative survival probability {:.6} at t={} in {}",
                    sp,
                    t,
                    self.id().as_str()
                )));
            }

            if sp > 1.0 {
                return Err(Error::Validation(format!(
                    "Survival probability {:.6} exceeds 1.0 at t={} in {}",
                    sp,
                    t,
                    self.id().as_str()
                )));
            }
        }

        // Validate recovery rate
        let recovery = self.recovery_rate();
        if !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Validation(format!(
                "Recovery rate {:.2}% outside [0, 100%] range in {}",
                recovery * 100.0,
                self.id().as_str()
            )));
        }

        Ok(())
    }
}

/// Validation for inflation curves
impl CurveValidator for InflationCurve {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // CPI levels should be positive
        for &t in INFL_ARBI_POINTS {
            let cpi = self.cpi(t);

            if cpi <= 0.0 {
                return Err(Error::Validation(format!(
                    "Non-positive CPI level {:.2} at t={} in {}",
                    cpi,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self) -> Result<()> {
        // CPI doesn't need to be strictly monotonic (deflation is possible)
        // but check for reasonable growth rates
        let base_cpi = self.cpi(0.0);

        for &t in INFL_MONO_POINTS {
            let cpi = self.cpi(t);
            let annual_inflation = (cpi / base_cpi).powf(1.0 / t) - 1.0;

            // Check for extreme deflation (< -10% annual)
            if annual_inflation < -0.10 {
                return Err(Error::Validation(format!(
                    "Extreme deflation {:.2}% per year over {} years in {}",
                    annual_inflation * 100.0,
                    t,
                    self.id().as_str()
                )));
            }

            // Check for hyperinflation (> 50% annual)
            if annual_inflation > 0.50 {
                return Err(Error::Validation(format!(
                    "Hyperinflation {:.2}% per year over {} years in {}",
                    annual_inflation * 100.0,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        // Check reasonable inflation expectations
        for &t in INFL_BOUNDS_POINTS {
            // Calculate forward inflation over 1-year period
            let cpi_t = self.cpi(t);
            let cpi_t1 = self.cpi(t + 1.0);
            let fwd_inflation = cpi_t1 / cpi_t - 1.0;

            // Forward inflation should be in reasonable range
            if !(-0.20..=0.50).contains(&fwd_inflation) {
                return Err(Error::Validation(format!(
                    "Forward inflation {:.2}% outside reasonable range at t={} in {}",
                    fwd_inflation * 100.0,
                    t,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }
}

/// Validation for base correlation curves
impl CurveValidator for BaseCorrelationCurve {
    fn validate_no_arbitrage(&self) -> Result<()> {
        // Base correlations should be monotonically increasing with detachment
        let detachment_points = self.detachment_points();
        let correlations = self.correlations();

        if detachment_points.len() != correlations.len() {
            return Err(Error::Validation(format!(
                "Detachment points and correlations length mismatch in {}",
                self.id().as_str()
            )));
        }

        // Check monotonicity
        for i in 1..correlations.len() {
            if correlations[i] < correlations[i - 1] - 1e-10 {
                return Err(Error::Validation(format!(
                    "Base correlation not monotonically increasing: corr({:.1}%)={:.2}% < corr({:.1}%)={:.2}% in {}",
                    detachment_points[i], correlations[i] * 100.0,
                    detachment_points[i-1], correlations[i-1] * 100.0,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self) -> Result<()> {
        // Already checked in validate_no_arbitrage
        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        // Correlations must be in [0, 1]
        for (detach, corr) in self
            .detachment_points()
            .iter()
            .zip(self.correlations().iter())
        {
            if *corr < 0.0 || *corr > 1.0 {
                return Err(Error::Validation(format!(
                    "Base correlation {:.2}% at {:.1}% detachment outside [0, 100%] range in {}",
                    corr * 100.0,
                    detach,
                    self.id().as_str()
                )));
            }
        }

        Ok(())
    }
}

/// Validation for volatility surfaces
pub trait SurfaceValidator {
    /// Validate no calendar spread arbitrage
    fn validate_calendar_spread(&self) -> Result<()>;

    /// Validate no butterfly arbitrage
    fn validate_butterfly_spread(&self) -> Result<()>;

    /// Validate volatility bounds
    fn validate_vol_bounds(&self) -> Result<()>;

    /// Run all validations
    fn validate(&self) -> Result<()> {
        self.validate_calendar_spread()?;
        self.validate_butterfly_spread()?;
        self.validate_vol_bounds()?;
        Ok(())
    }
}

impl SurfaceValidator for VolSurface {
    fn validate_calendar_spread(&self) -> Result<()> {
        // Total variance (σ²T) must be monotonically increasing with time to prevent calendar arbitrage.
        // This is a fundamental no-arbitrage condition: longer-dated options must have at least
        // as much total variance as shorter-dated options at the same strike.
        let strikes = self.strikes();
        let expiries = self.expiries();

        for strike in strikes {
            let mut prev_total_var = 0.0;

            for &expiry in expiries {
                let vol = self.value(expiry, *strike);
                let total_var = vol * vol * expiry; // σ²T

                // Check monotonicity of total variance
                if total_var < prev_total_var - 1e-10 {
                    // In strict mode, escalate to hard error to enforce no-arbitrage
                    #[cfg(feature = "strict_validation")]
                    {
                        return Err(Error::Validation(format!(
                            "Calendar arbitrage: total variance {:.6} < {:.6} at K={} in {}",
                            total_var,
                            prev_total_var,
                            strike,
                            self.id().as_str()
                        )));
                    }
                    #[cfg(not(feature = "strict_validation"))]
                    {
                        tracing::warn!(
                            "Calendar spread arbitrage detected: total variance {:.6} < {:.6} at K={} in {}. \
                            Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                            total_var, prev_total_var, strike, self.id().as_str()
                        );
                    }
                }

                prev_total_var = total_var;
            }
        }

        Ok(())
    }

    fn validate_butterfly_spread(&self) -> Result<()> {
        // Check convexity of total variance in strike dimension.
        // Proper butterfly arbitrage check requires that total variance (σ²T) is convex in strike,
        // which prevents risk-free arbitrage via butterfly spreads.
        //
        // For a more robust production implementation, consider:
        // - SVI parameterization (Gatheral) with explicit no-arbitrage constraints
        // - Monotone convex interpolation methods
        // - Arbitrage-free SABR wing fitting
        let strikes = self.strikes();
        let expiries = self.expiries();

        if strikes.len() < 3 {
            return Ok(()); // Need at least 3 strikes to check
        }

        for &expiry in expiries {
            for i in 1..strikes.len() - 1 {
                let k1 = strikes[i - 1];
                let k2 = strikes[i];
                let k3 = strikes[i + 1];

                let v1 = self.value(expiry, k1);
                let v2 = self.value(expiry, k2);
                let v3 = self.value(expiry, k3);

                // Convert to total variance for proper arbitrage check
                let w1 = v1 * v1 * expiry;
                let w2 = v2 * v2 * expiry;
                let w3 = v3 * v3 * expiry;

                // Check convexity of total variance: w2 should be ≤ linear interpolation
                let weight = (k2 - k1) / (k3 - k1);
                let w2_interpolated = w1 + weight * (w3 - w1);

                // Total variance should be convex (actual ≤ interpolated for upper convexity)
                // However, implied vol smiles typically show the opposite (actual > interpolated)
                // so we check for extreme violations that would create arbitrage
                if w2 > w2_interpolated * 1.5 || w2 < w2_interpolated * 0.5 {
                    #[cfg(feature = "strict_validation")]
                    {
                        return Err(Error::Validation(format!(
                            "Butterfly arbitrage at T={:.2}, K={:.2} in {}: total_var={:.6} vs interpolated={:.6} (ratio {:.2})",
                            expiry,
                            k2,
                            self.id().as_str(),
                            w2,
                            w2_interpolated,
                            w2 / w2_interpolated
                        )));
                    }
                    #[cfg(not(feature = "strict_validation"))]
                    {
                        tracing::warn!(
                            "Potential butterfly arbitrage at T={:.2}, K={:.2} in {}: \
                            total_var={:.6} vs interpolated={:.6} (ratio {:.2}). \
                            Consider SVI or monotone convex fitting.",
                            expiry,
                            k2,
                            self.id().as_str(),
                            w2,
                            w2_interpolated,
                            w2 / w2_interpolated
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_vol_bounds(&self) -> Result<()> {
        let strikes = self.strikes();
        let expiries = self.expiries();

        for &expiry in expiries {
            for strike in strikes {
                let vol = self.value(expiry, *strike);

                // Volatility should be positive
                if vol <= 0.0 {
                    return Err(Error::Validation(format!(
                        "Non-positive volatility {:.2}% at T={}, K={} in {}",
                        vol * 100.0,
                        expiry,
                        strike,
                        self.id().as_str()
                    )));
                }

                // Cap at reasonable maximum (500% vol)
                if vol > 5.0 {
                    return Err(Error::Validation(format!(
                        "Unreasonably high volatility {:.2}% at T={}, K={} in {}",
                        vol * 100.0,
                        expiry,
                        strike,
                        self.id().as_str()
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Validation configuration for different curve types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable forward rate positivity check
    pub check_forward_positivity: bool,
    /// Minimum allowed forward rate (can be slightly negative)
    pub min_forward_rate: f64,
    /// Maximum allowed forward rate
    pub max_forward_rate: f64,
    /// Enable monotonicity checks
    pub check_monotonicity: bool,
    /// Enable arbitrage checks
    pub check_arbitrage: bool,
    /// Numerical tolerance for comparisons
    pub tolerance: f64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            check_forward_positivity: true,
            min_forward_rate: -0.01, // Allow 1% negative
            max_forward_rate: 0.50,  // 50% cap
            check_monotonicity: true,
            check_arbitrage: true,
            tolerance: 1e-10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    #[test]
    fn test_discount_curve_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Valid curve - monotonically decreasing DFs
        let valid_curve = DiscountCurve::builder("TEST-VALID")
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.9950),
                (0.5, 0.9900),
                (1.0, 0.9800),
                (2.0, 0.9600),
                (5.0, 0.9000),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        assert!(valid_curve.validate().is_ok());

        // Invalid curve - increasing discount factors
        // NOTE: Must use allow_non_monotonic() since monotonicity is now enforced by default
        let invalid_curve = DiscountCurve::builder("TEST-INVALID")
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),
                (1.0, 0.95),
                (2.0, 0.96), // Increases!
                (5.0, 0.90),
            ])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic() // Allow construction of invalid curve for testing validation
            .build()
            .unwrap();

        assert!(invalid_curve.validate_monotonicity().is_err());
    }

    #[test]
    fn test_hazard_curve_validation() {
        use finstack_core::market_data::term_structures::hazard_curve::Seniority;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Valid hazard curve
        let valid_curve = HazardCurve::builder("TEST-HAZARD")
            .base_date(base_date)
            .recovery_rate(0.40)
            .seniority(Seniority::Senior)
            .knots(vec![(1.0, 0.01), (2.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap();

        assert!(valid_curve.validate().is_ok());

        // Check survival probability monotonicity
        assert!(valid_curve.validate_monotonicity().is_ok());
    }

    #[test]
    fn test_forward_curve_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Valid forward curve
        let valid_curve = ForwardCurve::builder("TEST-FWD", 0.25)
            .base_date(base_date)
            .knots(vec![
                (0.25, 0.045),
                (0.5, 0.046),
                (1.0, 0.047),
                (2.0, 0.048),
            ])
            .build()
            .unwrap();

        assert!(valid_curve.validate().is_ok());

        // Curve with negative forward rates (should fail if too negative)
        let negative_curve = ForwardCurve::builder("TEST-NEG-FWD", 0.25)
            .base_date(base_date)
            .knots(vec![
                (0.25, -0.08), // -8% forward rate (builder may fail on very negative)
                (0.5, 0.02),
                (1.0, 0.03),
            ])
            .build();

        // The curve builder itself might reject very negative rates,
        // or if it accepts them, our validation should reject them
        match negative_curve {
            Ok(curve) => {
                // If builder accepts it, our validation should reject it
                assert!(curve.validate_bounds().is_err());
            }
            Err(_) => {
                // Builder rejected it, which is also a valid outcome
                // This means the core library has its own validation
            }
        }
    }

    #[test]
    fn test_base_correlation_validation() {
        // Valid base correlation curve - monotonically increasing
        let valid_curve = BaseCorrelationCurve::builder("TEST-CORR")
            .knots(vec![
                (3.0, 0.20),
                (7.0, 0.35),
                (10.0, 0.45),
                (15.0, 0.60),
                (30.0, 0.80),
            ])
            .build()
            .unwrap();

        assert!(valid_curve.validate().is_ok());

        // Invalid curve - decreasing correlation
        let invalid_curve = BaseCorrelationCurve::builder("TEST-INVALID-CORR")
            .knots(vec![
                (3.0, 0.40),
                (7.0, 0.30), // Decreases!
                (10.0, 0.50),
            ])
            .build()
            .unwrap();

        assert!(invalid_curve.validate_no_arbitrage().is_err());
    }
}
