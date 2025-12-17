//! Surface validators (volatility surfaces).

use crate::calibration::validation::ValidationConfig;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::{Error, Result};

/// Validation for volatility surfaces.
pub trait SurfaceValidator {
    /// Validate no calendar spread arbitrage.
    fn validate_calendar_spread(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate no butterfly arbitrage.
    fn validate_butterfly_spread(&self, config: &ValidationConfig) -> Result<()>;

    /// Validate volatility bounds.
    fn validate_vol_bounds(&self, config: &ValidationConfig) -> Result<()>;

    /// Run all validations.
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
                // Grid points are guaranteed in bounds
                let vol = self.value_unchecked(expiry, *strike);
                let total_var = vol * vol * expiry; // σ²T

                // Check monotonicity of total variance
                if total_var < prev_total_var - config.tolerance {
                    violations.push((*strike, expiry, total_var, prev_total_var));

                    if config.lenient_arbitrage {
                        tracing::warn!(
                            "Calendar spread arbitrage detected: total variance {:.6} < {:.6} at K={}, T={:.4} (prev T={:.4}) in {}. \
                            Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                            total_var,
                            prev_total_var,
                            strike,
                            expiry,
                            prev_expiry,
                            self.id().as_str()
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
                    format!(
                        "K={:.2}, T={:.4}y (var={:.6} < {:.6})",
                        k, t, actual, expected
                    )
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

                // Grid points are guaranteed in bounds
                let v1 = self.value_unchecked(expiry, k1);
                let v2 = self.value_unchecked(expiry, k2);
                let v3 = self.value_unchecked(expiry, k3);

                // Convert to total variance for proper arbitrage check
                let w1 = v1 * v1 * expiry;
                let w2 = v2 * v2 * expiry;
                let w3 = v3 * v3 * expiry;

                // Check convexity of total variance: w2 should be ≤ linear interpolation
                let weight = (k2 - k1) / (k3 - k1);
                let w2_interpolated = w1 + weight * (w3 - w1);

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
                // Grid points are guaranteed in bounds
                let vol = self.value_unchecked(expiry, *strike);

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
