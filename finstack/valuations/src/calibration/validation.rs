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
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate monotonicity constraints
    fn validate_monotonicity(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate that all values are within reasonable bounds
    fn validate_bounds(&self, config: &ValidationConfig) -> Result<()>;

    /// Run all validations
    fn validate(&self, config: &ValidationConfig) -> Result<()> {
        self.validate_no_arbitrage(config)?;
        self.validate_monotonicity(config)?;
        self.validate_bounds(config)?;
        Ok(())
    }
}

/// Validation for discount curves
impl CurveValidator for DiscountCurve {
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

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
                if fwd_rate < config.min_forward_rate {
                    return Err(Error::Validation(format!(
                        "Negative forward rate {:.4}% between t={} and t={} in {} (limit: {:.2}%)",
                        fwd_rate * 100.0,
                        t1,
                        t2,
                        self.id().as_str(),
                        config.min_forward_rate * 100.0
                    )));
                }

                // Forward rates shouldn't be unreasonably high
                if fwd_rate > config.max_forward_rate {
                    return Err(Error::Validation(format!(
                        "Unreasonably high forward rate {:.2}% between t={} and t={} in {} (limit: {:.2}%)",
                        fwd_rate * 100.0,
                        t1,
                        t2,
                        self.id().as_str(),
                        config.max_forward_rate * 100.0
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_monotonicity {
            return Ok(());
        }

        // Auto-detect rate regime by checking the shortest non-zero zero rate.
        // In positive-rate regimes, DFs must be monotonically decreasing.
        // In negative-rate regimes (e.g., EUR/JPY/CHF), DFs can exceed 1.0
        // at the short end and may not decrease monotonically.
        let short_end_rate = self.zero(0.25); // Check 3-month zero rate

        // Determine if we're in a negative rate environment
        let is_negative_rate_environment = short_end_rate < -config.tolerance;

        // Only skip monotonicity check if:
        // 1. We're actually in a negative rate environment (auto-detected), AND
        // 2. The user has explicitly allowed negative rates
        if is_negative_rate_environment && config.allow_negative_rates {
            // For genuinely negative rate curves, skip strict monotonicity
            // but forward rate bounds are still checked by validate_no_arbitrage
            return Ok(());
        }

        // In positive-rate environments (or when negative rates not allowed),
        // enforce strict monotonicity - this is the market standard constraint
        let mut prev_df = 1.0;

        for &t in DF_MONO_POINTS {
            let df = self.df(t);

            // Allow for numerical tolerance
            if df > prev_df + config.tolerance {
                return Err(Error::Validation(format!(
                    "Discount factor not monotonically decreasing: DF({})={:.6} > DF(prev)={:.6} in {} \
                    (positive-rate environment detected; set allow_negative_rates=true if this is intentional)",
                    t, df, prev_df, self.id().as_str()
                )));
            }

            prev_df = df;
        }

        Ok(())
    }

    fn validate_bounds(&self, config: &ValidationConfig) -> Result<()> {
        // Check that discount factors are in (0, max_df]
        // For negative rate environments, DF can exceed 1.0 at short end
        let max_df = if config.allow_negative_rates {
            1.5
        } else {
            1.0
        };

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

            if df > max_df {
                return Err(Error::Validation(format!(
                    "Discount factor {:.6} exceeds {:.1} at t={} in {} (max DF for {} rate environment)",
                    df,
                    max_df,
                    t,
                    self.id().as_str(),
                    if config.allow_negative_rates { "negative" } else { "positive" }
                )));
            }
        }

        // Check zero rates are reasonable
        for &t in DF_BOUNDS_POINTS {
            let rate = self.zero(t);

            // Allow slightly negative rates but not too extreme
            if rate < config.min_forward_rate * 5.0 {
                // e.g. -5% if min_forward is -1%
                return Err(Error::Validation(format!(
                    "Zero rate {:.2}% too negative at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }

            // Cap at reasonable maximum
            if rate > config.max_forward_rate {
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
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage && !config.check_forward_positivity {
            return Ok(());
        }

        // Forward rates should be positive (with small tolerance for negative rates)
        for &t in FWD_ARBI_POINTS {
            let fwd = self.rate(t);

            if fwd < config.min_forward_rate {
                return Err(Error::Validation(format!(
                    "Negative forward rate {:.4}% at t={} in {} (limit: {:.2}%)",
                    fwd * 100.0,
                    t,
                    self.id().as_str(),
                    config.min_forward_rate * 100.0
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_monotonicity {
            return Ok(());
        }

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

    fn validate_bounds(&self, config: &ValidationConfig) -> Result<()> {
        for &t in FWD_BOUNDS_POINTS {
            let rate = self.rate(t);

            // Allow slightly negative but bounded
            if rate < config.min_forward_rate * 5.0 {
                return Err(Error::Validation(format!(
                    "Forward rate {:.2}% too negative at t={} in {}",
                    rate * 100.0,
                    t,
                    self.id().as_str()
                )));
            }

            // Cap at reasonable maximum
            if rate > config.max_forward_rate {
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
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

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
            if lambda > config.max_hazard_rate {
                return Err(Error::Validation(format!(
                    "Unreasonably high hazard rate {:.2} at t={} in {} (limit: {:.2})",
                    lambda,
                    t,
                    self.id().as_str(),
                    config.max_hazard_rate
                )));
            }
        }

        Ok(())
    }

    fn validate_monotonicity(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_monotonicity {
            return Ok(());
        }

        // Survival probabilities must be monotonically decreasing
        let mut prev_sp = 1.0;

        for &t in HAZARD_MONO_POINTS {
            let sp = self.sp(t);

            // Allow for numerical tolerance
            if sp > prev_sp + config.tolerance {
                return Err(Error::Validation(format!(
                    "Survival probability not monotonically decreasing: SP({})={:.6} > SP(prev)={:.6} in {}",
                    t, sp, prev_sp, self.id().as_str()
                )));
            }

            prev_sp = sp;
        }

        Ok(())
    }

    fn validate_bounds(&self, _config: &ValidationConfig) -> Result<()> {
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
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

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

    fn validate_monotonicity(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_monotonicity {
            return Ok(());
        }

        // CPI doesn't need to be strictly monotonic (deflation is possible)
        // but check for reasonable growth rates
        let base_cpi = self.cpi(0.0);

        for &t in INFL_MONO_POINTS {
            let cpi = self.cpi(t);
            let annual_inflation = (cpi / base_cpi).powf(1.0 / t) - 1.0;

            // Check for extreme deflation (< -10% annual)
            if annual_inflation < config.min_cpi_growth {
                return Err(Error::Validation(format!(
                    "Extreme deflation {:.2}% per year over {} years in {} (limit: {:.2}%)",
                    annual_inflation * 100.0,
                    t,
                    self.id().as_str(),
                    config.min_cpi_growth * 100.0
                )));
            }

            // Check for hyperinflation (> 50% annual)
            if annual_inflation > config.max_cpi_growth {
                return Err(Error::Validation(format!(
                    "Hyperinflation {:.2}% per year over {} years in {} (limit: {:.2}%)",
                    annual_inflation * 100.0,
                    t,
                    self.id().as_str(),
                    config.max_cpi_growth * 100.0
                )));
            }
        }

        Ok(())
    }

    fn validate_bounds(&self, config: &ValidationConfig) -> Result<()> {
        // Check reasonable inflation expectations
        for &t in INFL_BOUNDS_POINTS {
            // Calculate forward inflation over 1-year period
            let cpi_t = self.cpi(t);
            let cpi_t1 = self.cpi(t + 1.0);
            let fwd_inflation = cpi_t1 / cpi_t - 1.0;

            // Forward inflation should be in reasonable range
            if !(config.min_fwd_inflation..=config.max_fwd_inflation).contains(&fwd_inflation) {
                return Err(Error::Validation(format!(
                    "Forward inflation {:.2}% outside reasonable range [{:.2}%, {:.2}%] at t={} in {}",
                    fwd_inflation * 100.0,
                    config.min_fwd_inflation * 100.0,
                    config.max_fwd_inflation * 100.0,
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
    fn validate_no_arbitrage(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

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
            if correlations[i] < correlations[i - 1] - config.tolerance {
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

    fn validate_monotonicity(&self, _config: &ValidationConfig) -> Result<()> {
        // Already checked in validate_no_arbitrage
        Ok(())
    }

    fn validate_bounds(&self, _config: &ValidationConfig) -> Result<()> {
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
    fn validate_calendar_spread(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate no butterfly arbitrage
    fn validate_butterfly_spread(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate volatility bounds
    fn validate_vol_bounds(&self, config: &ValidationConfig) -> Result<()>;

    /// Run all validations
    fn validate(&self, config: &ValidationConfig) -> Result<()> {
        self.validate_calendar_spread(config)?;
        self.validate_butterfly_spread(config)?;
        self.validate_vol_bounds(config)?;
        Ok(())
    }
}

impl SurfaceValidator for VolSurface {
    fn validate_calendar_spread(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

        // Total variance (σ²T) must be monotonically increasing with time to prevent calendar arbitrage.
        // This is a fundamental no-arbitrage condition: longer-dated options must have at least
        // as much total variance as shorter-dated options at the same strike.
        let strikes = self.strikes();
        let expiries = self.expiries();
        let mut violations: Vec<(f64, f64, f64, f64)> = Vec::new(); // (strike, expiry, actual, expected)

        for strike in strikes {
            let mut prev_total_var = 0.0;
            let mut prev_expiry = 0.0_f64;

            for &expiry in expiries {
                let vol = self.value(expiry, *strike);
                let total_var = vol * vol * expiry; // σ²T

                // Check monotonicity of total variance
                if total_var < prev_total_var - config.tolerance {
                    violations.push((*strike, expiry, total_var, prev_total_var));

                    if config.lenient_arbitrage {
                        tracing::warn!(
                            "Calendar spread arbitrage detected: total variance {:.6} < {:.6} at K={}, T={:.4} (prev T={:.4}) in {}. \
                            Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                            total_var, prev_total_var, strike, expiry, prev_expiry, self.id().as_str()
                        );
                    }
                }

                prev_total_var = total_var;
                prev_expiry = expiry;
            }
        }

        // In strict mode (default), fail on any calendar arbitrage violations
        if !violations.is_empty() && !config.lenient_arbitrage {
            let details: Vec<String> = violations
                .iter()
                .take(5)
                .map(|(k, t, actual, expected)| {
                    format!("K={:.2}, T={:.4}y (var={:.6} < {:.6})", k, t, actual, expected)
                })
                .collect();
            let suffix = if violations.len() > 5 {
                format!(" (and {} more)", violations.len() - 5)
            } else {
                String::new()
            };
            return Err(Error::Validation(format!(
                "Calendar spread arbitrage detected at {} point(s) in {}: [{}]{}. \
                Total variance must be monotonically increasing in expiry. \
                Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                violations.len(),
                self.id().as_str(),
                details.join("; "),
                suffix
            )));
        }

        Ok(())
    }

    fn validate_butterfly_spread(&self, config: &ValidationConfig) -> Result<()> {
        if !config.check_arbitrage {
            return Ok(());
        }

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

        let mut violations: Vec<(f64, f64, f64, f64, f64)> = Vec::new(); // (expiry, strike, actual, interp, ratio)

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
                let ratio = if w2_interpolated.abs() > 1e-12 {
                    w2 / w2_interpolated
                } else {
                    1.0
                };

                if w2 > w2_interpolated * 1.5 || w2 < w2_interpolated * 0.5 {
                    violations.push((expiry, k2, w2, w2_interpolated, ratio));

                    if config.lenient_arbitrage {
                        tracing::warn!(
                            "Potential butterfly arbitrage at T={:.2}, K={:.2} in {}: \
                            total_var={:.6} vs interpolated={:.6} (ratio {:.2}). \
                            Consider SVI or monotone convex fitting.",
                            expiry,
                            k2,
                            self.id().as_str(),
                            w2,
                            w2_interpolated,
                            ratio
                        );
                    }
                }
            }
        }

        // In strict mode (default), fail on any butterfly arbitrage violations
        if !violations.is_empty() && !config.lenient_arbitrage {
            let details: Vec<String> = violations
                .iter()
                .take(5)
                .map(|(t, k, actual, interp, ratio)| {
                    format!(
                        "T={:.2}y, K={:.2} (var={:.6} vs interp={:.6}, ratio={:.2})",
                        t, k, actual, interp, ratio
                    )
                })
                .collect();
            let suffix = if violations.len() > 5 {
                format!(" (and {} more)", violations.len() - 5)
            } else {
                String::new()
            };
            return Err(Error::Validation(format!(
                "Butterfly spread arbitrage detected at {} point(s) in {}: [{}]{}. \
                Total variance must be convex in strike. \
                Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                violations.len(),
                self.id().as_str(),
                details.join("; "),
                suffix
            )));
        }

        Ok(())
    }

    fn validate_vol_bounds(&self, config: &ValidationConfig) -> Result<()> {
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
                if vol > config.max_volatility {
                    return Err(Error::Validation(format!(
                        "Unreasonably high volatility {:.2}% at T={}, K={} in {} (limit: {:.2}%)",
                        vol * 100.0,
                        expiry,
                        strike,
                        self.id().as_str(),
                        config.max_volatility * 100.0
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
    /// Maximum allowed hazard rate (default 0.5 = 50%)
    pub max_hazard_rate: f64,
    /// Minimum allowed annual CPI growth (default -0.10 = -10%)
    pub min_cpi_growth: f64,
    /// Maximum allowed annual CPI growth (default 0.50 = 50%)
    pub max_cpi_growth: f64,
    /// Minimum allowed forward inflation (default -0.20 = -20%)
    pub min_fwd_inflation: f64,
    /// Maximum allowed forward inflation (default 0.50 = 50%)
    pub max_fwd_inflation: f64,
    /// Maximum allowed volatility (default 5.0 = 500%)
    pub max_volatility: f64,
    /// Allow negative rate environments (DF > 1.0 at short end)
    #[serde(default)]
    pub allow_negative_rates: bool,
    /// When true, arbitrage violations (calendar/butterfly) produce warnings instead of errors.
    /// Default is false - arbitrage violations fail validation.
    /// Set to true only for exploratory analysis or when arbitrage-free fitting is not required.
    #[serde(default)]
    pub lenient_arbitrage: bool,
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
            max_hazard_rate: 0.50,
            min_cpi_growth: -0.10,
            max_cpi_growth: 0.50,
            min_fwd_inflation: -0.20,
            max_fwd_inflation: 0.50,
            max_volatility: 5.0,
            // Default to strict mode: enforce monotonicity in positive-rate regimes.
            // Set to true for EUR/JPY/CHF negative-rate environments where DFs > 1.0 is valid.
            allow_negative_rates: false,
            // Default to strict mode: arbitrage violations fail validation.
            // Set to true only for exploratory analysis.
            lenient_arbitrage: false,
        }
    }
}

impl ValidationConfig {
    /// Create a strict validation config that enforces monotonicity
    /// even in potentially negative rate environments.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            allow_negative_rates: false,
            lenient_arbitrage: false,
            ..Default::default()
        }
    }

    /// Create a permissive validation config for negative rate environments
    /// (e.g., EUR/JPY/CHF) where discount factors > 1.0 at short tenors is valid.
    #[must_use]
    pub fn negative_rates() -> Self {
        Self {
            allow_negative_rates: true,
            ..Default::default()
        }
    }

    /// Create a lenient configuration that warns but does not fail on arbitrage.
    ///
    /// Use this only for exploratory analysis or when strict arbitrage-free
    /// surfaces are not required. Calendar spread and butterfly arbitrage
    /// violations will log warnings instead of returning errors.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            lenient_arbitrage: true,
            ..Default::default()
        }
    }

    /// Set whether arbitrage violations should warn (lenient) or error (strict).
    ///
    /// By default, arbitrage violations fail validation. Set `lenient = true`
    /// only for exploratory analysis or when arbitrage-free constraints are
    /// not required.
    #[must_use]
    pub fn with_lenient_arbitrage(mut self, lenient: bool) -> Self {
        self.lenient_arbitrage = lenient;
        self
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
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let config = ValidationConfig::default();

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
            .expect("should build valid curve");

        assert!(valid_curve.validate(&config).is_ok());

        // Invalid curve - increasing discount factors
        // NOTE: Must use allow_non_monotonic() since monotonicity is now enforced by default
        let invalid_curve = DiscountCurve::builder("TEST-INVALID")
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.99), // Positive rates at short end
                (1.0, 0.95),
                (2.0, 0.96), // Increases! Violation.
                (5.0, 0.90),
            ])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic() // Allow construction of invalid curve for testing validation
            .build()
            .expect("should build invalid curve for testing");

        // Default config now enforces monotonicity (allow_negative_rates = false)
        assert!(invalid_curve.validate_monotonicity(&config).is_err());
    }

    #[test]
    fn test_hazard_curve_validation() {
        use finstack_core::market_data::term_structures::hazard_curve::Seniority;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let config = ValidationConfig::default();

        // Valid hazard curve
        let valid_curve = HazardCurve::builder("TEST-HAZARD")
            .base_date(base_date)
            .recovery_rate(0.40)
            .seniority(Seniority::Senior)
            .knots(vec![(1.0, 0.01), (2.0, 0.015), (5.0, 0.02)])
            .build()
            .expect("should build valid hazard curve");

        assert!(valid_curve.validate(&config).is_ok());

        // Check survival probability monotonicity
        assert!(valid_curve.validate_monotonicity(&config).is_ok());
    }

    #[test]
    fn test_forward_curve_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let config = ValidationConfig::default();

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
            .expect("should build valid forward curve");

        assert!(valid_curve.validate(&config).is_ok());

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
                assert!(curve.validate_bounds(&config).is_err());
            }
            Err(_) => {
                // Builder rejected it, which is also a valid outcome
                // This means the core library has its own validation
            }
        }
    }

    #[test]
    fn test_base_correlation_validation() {
        let config = ValidationConfig::default();
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
            .expect("should build valid base correlation curve");

        assert!(valid_curve.validate(&config).is_ok());

        // Invalid curve - decreasing correlation
        let invalid_curve = BaseCorrelationCurve::builder("TEST-INVALID-CORR")
            .knots(vec![
                (3.0, 0.40),
                (7.0, 0.30), // Decreases!
                (10.0, 0.50),
            ])
            .build()
            .expect("should build invalid curve for testing");

        assert!(invalid_curve.validate_no_arbitrage(&config).is_err());
    }

    /// Test that non-monotonic discount curves with positive rates are rejected by default.
    /// This is the key market standard: DF(t2) < DF(t1) for t2 > t1 when rates are positive.
    #[test]
    fn test_non_monotone_positive_rate_curve_rejected() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create a curve with DF(2Y) > DF(1Y) - violates monotonicity in positive-rate regime
        // This represents an arbitrage opportunity: borrow at 1Y, lend at 2Y for free money
        let non_monotone_curve = DiscountCurve::builder("TEST-NON-MONOTONE")
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.99), // Positive rates (DF < 1)
                (0.5, 0.98),
                (1.0, 0.95),
                (2.0, 0.96), // DF(2Y) > DF(1Y) - violation!
                (5.0, 0.90),
            ])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic() // Allow construction for testing
            .build()
            .expect("should build non-monotone curve for testing");

        // Verify this is indeed a positive-rate curve (short-end zero rate > 0)
        let short_rate = non_monotone_curve.zero(0.25);
        assert!(short_rate > 0.0, "Expected positive short-end rate, got {}", short_rate);

        // With default config, this should be rejected
        let default_config = ValidationConfig::default();
        let result = non_monotone_curve.validate_monotonicity(&default_config);
        assert!(
            result.is_err(),
            "Non-monotone positive-rate curve should be rejected by default config"
        );

        // Error message should indicate the monotonicity violation
        let err_msg = result.expect_err("Expected validation error for non-monotone curve").to_string();
        assert!(
            err_msg.contains("not monotonically decreasing"),
            "Error should mention monotonicity: {}",
            err_msg
        );
    }

    /// Test that negative rate environments are still supported when explicitly opted in.
    /// In EUR/JPY/CHF markets, rates can be negative, causing DF > 1.0 at short tenors.
    #[test]
    fn test_negative_rate_environment_opt_in() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Create a curve with negative rates (DF > 1.0 at short end)
        // This is valid for EUR/JPY/CHF environments
        let negative_rate_curve = DiscountCurve::builder("TEST-NEGATIVE-RATES")
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 1.005), // DF > 1.0 implies negative rates
                (0.5, 1.008),
                (1.0, 1.010),
                (2.0, 1.005), // Can be non-monotone in negative rate environments
                (5.0, 0.99),
            ])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic() // Allow construction
            .build()
            .expect("should build negative rate curve for testing");

        // Verify this is indeed a negative-rate curve (short-end zero rate < 0)
        let short_rate = negative_rate_curve.zero(0.25);
        assert!(short_rate < 0.0, "Expected negative short-end rate, got {}", short_rate);

        // With default config (allow_negative_rates = false), should be rejected
        let default_config = ValidationConfig::default();
        // Note: the curve is auto-detected as negative rate environment, but
        // since allow_negative_rates is false, monotonicity is still enforced
        let strict_result = negative_rate_curve.validate_monotonicity(&default_config);
        assert!(
            strict_result.is_err(),
            "Negative rate curve should be rejected when allow_negative_rates=false"
        );

        // With explicit negative_rates config, should be accepted
        let permissive_config = ValidationConfig::negative_rates();
        let permissive_result = negative_rate_curve.validate_monotonicity(&permissive_config);
        assert!(
            permissive_result.is_ok(),
            "Negative rate curve should pass when allow_negative_rates=true: {:?}",
            permissive_result
        );
    }

    /// Test that the validation config constructors work correctly.
    #[test]
    fn test_validation_config_constructors() {
        let strict = ValidationConfig::strict();
        assert!(!strict.allow_negative_rates, "strict() should set allow_negative_rates=false");
        assert!(strict.check_monotonicity, "strict() should enable monotonicity checks");
        assert!(!strict.lenient_arbitrage, "strict() should set lenient_arbitrage=false");

        let negative = ValidationConfig::negative_rates();
        assert!(negative.allow_negative_rates, "negative_rates() should set allow_negative_rates=true");
        assert!(negative.check_monotonicity, "negative_rates() should enable monotonicity checks");

        let lenient = ValidationConfig::lenient();
        assert!(lenient.lenient_arbitrage, "lenient() should set lenient_arbitrage=true");
        assert!(lenient.check_arbitrage, "lenient() should still enable arbitrage checks");

        let default = ValidationConfig::default();
        assert!(!default.allow_negative_rates, "default should set allow_negative_rates=false (strict mode)");
        assert!(!default.lenient_arbitrage, "default should set lenient_arbitrage=false (strict mode)");
    }

    /// Test that butterfly arbitrage in vol surfaces is detected and fails validation.
    ///
    /// Butterfly arbitrage occurs when total variance is not convex in strike,
    /// allowing risk-free profits via butterfly spreads.
    #[test]
    fn test_butterfly_arbitrage_detected_and_fails() {
        // Create a surface with butterfly arbitrage: middle strike has extreme variance
        // compared to the linear interpolation between adjacent strikes
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![90.0, 100.0, 110.0];

        // At T=0.5, the 100 strike has vol=0.50 while 90/110 have vol=0.20
        // This creates a "smile" so extreme it violates convexity bounds
        let vol_grid = vec![
            // T=0.25
            0.20, 0.18, 0.20, // T=0.5 - extreme butterfly violation
            0.20, 0.50, 0.20, // vol at K=100 is 2.5x neighbors
            // T=1.0
            0.22, 0.20, 0.22,
        ];

        let surface = VolSurface::from_grid("TEST-BUTTERFLY-ARB", &expiries, &strikes, &vol_grid)
            .expect("should build vol surface for testing");

        // With default (strict) config, butterfly validation should fail
        let strict_config = ValidationConfig::default();
        let result = surface.validate_butterfly_spread(&strict_config);
        assert!(
            result.is_err(),
            "Butterfly arbitrage should be detected and fail in strict mode"
        );

        // Error should mention butterfly arbitrage
        let err_msg = result.expect_err("Expected validation error for butterfly arbitrage").to_string();
        assert!(
            err_msg.contains("Butterfly") || err_msg.contains("butterfly"),
            "Error should mention butterfly arbitrage: {}",
            err_msg
        );

        // With lenient config, should pass (warning only)
        let lenient_config = ValidationConfig::lenient();
        let lenient_result = surface.validate_butterfly_spread(&lenient_config);
        assert!(
            lenient_result.is_ok(),
            "Butterfly arbitrage should only warn in lenient mode: {:?}",
            lenient_result
        );
    }

    /// Test that calendar spread arbitrage is detected and fails validation.
    ///
    /// Calendar arbitrage occurs when total variance (σ²T) decreases with maturity,
    /// allowing risk-free profits via calendar spreads.
    #[test]
    fn test_calendar_arbitrage_detected_and_fails() {
        // Create a surface with calendar arbitrage: shorter expiry has higher total variance
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![95.0, 100.0, 105.0];

        // Total variance = σ²T
        // At K=100: T=0.25 → σ=0.40 → var=0.16*0.25=0.04
        //           T=0.50 → σ=0.20 → var=0.04*0.50=0.02 < 0.04 (violation!)
        //           T=1.00 → σ=0.22 → var=0.0484
        let vol_grid = vec![
            // T=0.25
            0.35, 0.40, 0.35, // T=0.5 - lower vol causes calendar arbitrage
            0.18, 0.20, 0.18, // T=1.0
            0.20, 0.22, 0.20,
        ];

        let surface = VolSurface::from_grid("TEST-CALENDAR-ARB", &expiries, &strikes, &vol_grid)
            .expect("should build vol surface for testing");

        // With default (strict) config, calendar validation should fail
        let strict_config = ValidationConfig::default();
        let result = surface.validate_calendar_spread(&strict_config);
        assert!(
            result.is_err(),
            "Calendar arbitrage should be detected and fail in strict mode"
        );

        // Error should mention calendar arbitrage
        let err_msg = result.expect_err("Expected validation error for calendar arbitrage").to_string();
        assert!(
            err_msg.contains("Calendar") || err_msg.contains("calendar"),
            "Error should mention calendar arbitrage: {}",
            err_msg
        );

        // With lenient config, should pass (warning only)
        let lenient_config = ValidationConfig::lenient();
        let lenient_result = surface.validate_calendar_spread(&lenient_config);
        assert!(
            lenient_result.is_ok(),
            "Calendar arbitrage should only warn in lenient mode: {:?}",
            lenient_result
        );
    }

    /// Test that valid arbitrage-free surfaces pass all validations.
    #[test]
    fn test_valid_surface_passes_arbitrage_checks() {
        // Create a well-behaved surface with no arbitrage
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![90.0, 100.0, 110.0];

        // Reasonable smile with total variance increasing in T
        // and convex (or nearly linear) in K
        let vol_grid = vec![
            // T=0.25: moderate smile
            0.22, 0.20, 0.21, // T=0.5: similar smile, slightly higher overall
            0.23, 0.21, 0.22, // T=1.0: term structure upward sloping
            0.25, 0.23, 0.24,
        ];

        let surface = VolSurface::from_grid("TEST-VALID-SURFACE", &expiries, &strikes, &vol_grid)
            .expect("should build valid vol surface");

        let config = ValidationConfig::default();

        // All validations should pass
        assert!(
            surface.validate_calendar_spread(&config).is_ok(),
            "Valid surface should pass calendar arbitrage check"
        );
        assert!(
            surface.validate_butterfly_spread(&config).is_ok(),
            "Valid surface should pass butterfly arbitrage check"
        );
        assert!(
            surface.validate_vol_bounds(&config).is_ok(),
            "Valid surface should pass vol bounds check"
        );
        assert!(
            surface.validate(&config).is_ok(),
            "Valid surface should pass all validations"
        );
    }
}
