//! Volatility conventions and conversion utilities.
//!
//! This module provides standard volatility conventions (Normal, Lognormal, ShiftedLognormal)
//! and utilities for converting between them using analytical approximations.

use crate::math::{norm_cdf, norm_pdf};
use crate::math::{BrentSolver, Solver};

/// Volatility quoting convention.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VolatilityConvention {
    /// Normal (absolute) volatility in basis points
    Normal,
    /// Lognormal (Black) volatility as percentage
    Lognormal,
    /// Shifted lognormal for negative rates
    ShiftedLognormal {
        /// Shift amount for negative rate handling
        shift: f64,
    },
}

/// Bachelier (normal) call price with unit annuity.
///
/// Computes the price of a call option under the Bachelier model assuming a unit annuity (PV01=1).
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility
/// * `t` - Time to expiry
pub fn bachelier_price(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    if sigma_n <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma_n * t.sqrt();
    if st <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let d = (forward - strike) / st;
    (forward - strike) * norm_cdf(d) + st * norm_pdf(d)
}

/// Black (lognormal) call price with unit annuity.
///
/// Computes the price of a call option under the Black model assuming a unit annuity (PV01=1).
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry
pub fn black_price(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    if sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    let d2 = d1 - st;
    forward * norm_cdf(d1) - strike * norm_cdf(d2)
}

/// Black with shift (for shifted lognormal) call price with unit annuity.
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry
/// * `shift` - Shift amount
pub fn black_shifted_price(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_price(forward + shift, strike + shift, sigma, t)
}

/// Convert volatility between conventions.
///
/// Converts a volatility quote from one convention to another by equating the option prices
/// (implied volatility).
///
/// # Arguments
/// * `vol` - Input volatility
/// * `from_convention` - Source convention
/// * `to_convention` - Target convention
/// * `forward_rate` - Forward rate
/// * `time_to_expiry` - Time to expiry in years
/// * `zero_threshold` - Threshold for considering a rate as zero (for analytic approximations)
pub fn convert_volatility(
    vol: f64,
    from_convention: VolatilityConvention,
    to_convention: VolatilityConvention,
    forward_rate: f64,
    time_to_expiry: f64,
    zero_threshold: f64,
) -> f64 {
    // Early returns for identical convention (including same shift)
    if std::mem::discriminant(&from_convention) == std::mem::discriminant(&to_convention) {
        // If both are shifted, check shift equality
        if let (
            VolatilityConvention::ShiftedLognormal { shift: s1 },
            VolatilityConvention::ShiftedLognormal { shift: s2 },
        ) = (from_convention, to_convention)
        {
            if (s1 - s2).abs() < 1e-12 {
                return vol;
            }
        } else {
            return vol;
        }
    }

    // Preserve fast ATM analytic mapping for Normal <-> Lognormal
    if let (VolatilityConvention::Normal, VolatilityConvention::Lognormal) =
        (from_convention, to_convention)
    {
        if forward_rate.abs() > zero_threshold {
            return vol / forward_rate;
        } else {
            return vol;
        }
    }
    if let (VolatilityConvention::Lognormal, VolatilityConvention::Normal) =
        (from_convention, to_convention)
    {
        return vol * forward_rate;
    }

    // Compute price under source convention with unit annuity
    let price_from = match from_convention {
        VolatilityConvention::Normal => {
            bachelier_price(forward_rate, forward_rate, vol, time_to_expiry)
        }
        VolatilityConvention::Lognormal => {
            black_price(forward_rate, forward_rate, vol, time_to_expiry)
        }
        VolatilityConvention::ShiftedLognormal { shift } => {
            black_shifted_price(forward_rate, forward_rate, vol, time_to_expiry, shift)
        }
    };

    // General-strike conversion: use the actual forward_rate as strike for ATM
    // For non-ATM strikes elsewhere in code, the caller provides strike-specific vols.
    // We preserve that behavior by using K = F here.
    let f = forward_rate;
    let t = time_to_expiry.max(0.0);

    // Invert price to target convention by solving for sigma
    let objective = |sigma: f64| -> f64 {
        let sigma_pos = sigma.abs();
        let p = match to_convention {
            VolatilityConvention::Normal => bachelier_price(f, f, sigma_pos, t),
            VolatilityConvention::Lognormal => black_price(f, f, sigma_pos, t),
            VolatilityConvention::ShiftedLognormal { shift } => {
                black_shifted_price(f, f, sigma_pos, t, shift)
            }
        };
        p - price_from
    };

    // Initial guesses derived from simple ATM approximations
    let mut guess = match (from_convention, to_convention) {
        (VolatilityConvention::Normal, VolatilityConvention::Lognormal) => {
            if f.abs() > zero_threshold {
                vol / f
            } else {
                vol
            }
        }
        (VolatilityConvention::Lognormal, VolatilityConvention::Normal) => vol * f,
        (VolatilityConvention::Normal, VolatilityConvention::ShiftedLognormal { shift }) => {
            if (f + shift).abs() > zero_threshold {
                vol / (f + shift)
            } else {
                vol
            }
        }
        (VolatilityConvention::ShiftedLognormal { shift }, VolatilityConvention::Normal) => {
            vol * (f + shift)
        }
        (
            VolatilityConvention::Lognormal,
            VolatilityConvention::ShiftedLognormal { shift },
        ) => {
            if (f + shift).abs() > zero_threshold {
                vol * f / (f + shift)
            } else {
                vol
            }
        }
        (
            VolatilityConvention::ShiftedLognormal { shift },
            VolatilityConvention::Lognormal,
        ) => {
            if f.abs() > zero_threshold {
                vol * (f + shift) / f
            } else {
                vol
            }
        }
        _ => vol,
    };
    if !guess.is_finite() || guess <= 0.0 {
        guess = (vol.abs() + 1e-6).max(1e-6);
    }

    // Solve with a robust 1D solver
    // Use Brent solver directly since solve_1d helper is not available in core
    let solver = BrentSolver::new().with_tolerance(1e-8);
    let solved = solver.solve(objective, guess).unwrap_or(guess);
    
    let out = solved.abs();
    if out.is_finite() && out > 0.0 {
        out
    } else {
        vol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_vs_lognormal_conversion() {
        let forward = 0.05; // 5% forward rate
        let normal_vol = 0.01; // 100bp normal vol
        let zero_threshold = 1e-8;

        // Convert to lognormal
        let lognormal_vol = convert_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
            zero_threshold,
        );

        assert!((lognormal_vol - 0.2).abs() < 1e-6); // Should be 20% lognormal vol

        // Convert back
        let recovered_normal = convert_volatility(
            lognormal_vol,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Normal,
            forward,
            1.0,
            zero_threshold,
        );

        assert!((recovered_normal - normal_vol).abs() < 1e-10);
    }
}
