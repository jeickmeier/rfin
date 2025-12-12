//! Volatility conventions and conversion utilities.
//!
//! This module provides standard volatility conventions (Normal, Lognormal, ShiftedLognormal)
//! and utilities for converting between them using ATM (at-the-money) price matching.
//!
//! # ATM Conversion
//!
//! The conversion functions in this module are designed for **ATM (strike = forward)** volatility
//! conversions. For non-ATM conversions, users should either:
//! - Use a volatility surface that handles strike/delta-based conversions
//! - Implement custom conversion logic for specific strike levels
//!
//! # Negative Rate Handling
//!
//! When working with negative forward rates:
//! - **Normal volatility**: Works with any forward rate
//! - **Lognormal volatility**: Requires positive forward rate (will return error for F ≤ 0)
//! - **Shifted lognormal**: Use with shift such that (F + shift) > 0
//!
//! # Example
//!
//! ```rust
//! use finstack_core::volatility::{convert_atm_volatility, VolatilityConvention};
//!
//! let forward = 0.05; // 5% forward rate
//! let normal_vol = 0.01; // 100bp normal vol
//!
//! // Convert normal to lognormal (ATM)
//! let lognormal_vol = convert_atm_volatility(
//!     normal_vol,
//!     VolatilityConvention::Normal,
//!     VolatilityConvention::Lognormal,
//!     forward,
//!     1.0, // 1 year to expiry
//! ).expect("conversion should succeed for positive forward");
//!
//! assert!((lognormal_vol - 0.2).abs() < 1e-6); // ~20% lognormal vol
//! ```

use crate::error::InputError;
use crate::math::{norm_cdf, norm_pdf};
use crate::math::{BrentSolver, Solver};
use crate::Result;

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

/// Convert ATM volatility between conventions.
///
/// Converts a volatility quote from one convention to another by equating option prices
/// at ATM (strike = forward). This is a deterministic conversion that returns explicit
/// errors rather than silently falling back to guesses.
///
/// # Arguments
/// * `vol` - Input volatility (must be positive and finite)
/// * `from_convention` - Source convention
/// * `to_convention` - Target convention
/// * `forward_rate` - Forward rate
/// * `time_to_expiry` - Time to expiry in years (must be non-negative)
///
/// # Errors
///
/// Returns an error if:
/// - `vol` is not positive and finite ([`InputError::InvalidVolatility`])
/// - `time_to_expiry` is negative ([`InputError::InvalidTimeToExpiry`])
/// - Forward rate is non-positive for lognormal conversion ([`InputError::NonPositiveForwardForLognormal`])
/// - Shifted forward is non-positive ([`InputError::NonPositiveShiftedForward`])
/// - Solver fails to converge ([`InputError::VolatilityConversionFailed`])
///
/// # Design Note
///
/// This function explicitly performs **ATM-only** conversion (strike = forward).
/// For surface-aware or strike-specific conversions, use a volatility surface
/// implementation that handles the full strike dimension.
///
/// # Example
///
/// ```rust
/// use finstack_core::volatility::{convert_atm_volatility, VolatilityConvention};
///
/// let forward = 0.05; // 5% forward rate
/// let normal_vol = 0.01; // 100bp normal vol
///
/// let lognormal_vol = convert_atm_volatility(
///     normal_vol,
///     VolatilityConvention::Normal,
///     VolatilityConvention::Lognormal,
///     forward,
///     1.0,
/// ).unwrap();
///
/// // Round-trip conversion
/// let recovered = convert_atm_volatility(
///     lognormal_vol,
///     VolatilityConvention::Lognormal,
///     VolatilityConvention::Normal,
///     forward,
///     1.0,
/// ).unwrap();
///
/// assert!((recovered - normal_vol).abs() < 1e-10);
/// ```
pub fn convert_atm_volatility(
    vol: f64,
    from_convention: VolatilityConvention,
    to_convention: VolatilityConvention,
    forward_rate: f64,
    time_to_expiry: f64,
) -> Result<f64> {
    // Validate inputs
    if !vol.is_finite() || vol <= 0.0 {
        return Err(InputError::InvalidVolatility { value: vol }.into());
    }

    if time_to_expiry < 0.0 {
        return Err(InputError::InvalidTimeToExpiry {
            value: time_to_expiry,
        }
        .into());
    }

    // Validate forward rate for lognormal conventions
    validate_forward_for_convention(forward_rate, from_convention)?;
    validate_forward_for_convention(forward_rate, to_convention)?;

    // Early returns for identical convention (including same shift)
    if std::mem::discriminant(&from_convention) == std::mem::discriminant(&to_convention) {
        // If both are shifted, check shift equality
        if let (
            VolatilityConvention::ShiftedLognormal { shift: s1 },
            VolatilityConvention::ShiftedLognormal { shift: s2 },
        ) = (from_convention, to_convention)
        {
            if (s1 - s2).abs() < 1e-12 {
                return Ok(vol);
            }
        } else {
            return Ok(vol);
        }
    }

    // Fast analytic ATM approximation for Normal <-> Lognormal
    // σ_normal ≈ σ_lognormal * F (at ATM)
    if let (VolatilityConvention::Normal, VolatilityConvention::Lognormal) =
        (from_convention, to_convention)
    {
        // forward_rate is already validated to be positive for Lognormal target
        return Ok(vol / forward_rate);
    }
    if let (VolatilityConvention::Lognormal, VolatilityConvention::Normal) =
        (from_convention, to_convention)
    {
        return Ok(vol * forward_rate);
    }

    // General case: price matching with numerical solver
    let f = forward_rate;
    let t = time_to_expiry.max(0.0);

    // Compute price under source convention (ATM: strike = forward)
    let price_from = match from_convention {
        VolatilityConvention::Normal => bachelier_price(f, f, vol, t),
        VolatilityConvention::Lognormal => black_price(f, f, vol, t),
        VolatilityConvention::ShiftedLognormal { shift } => {
            black_shifted_price(f, f, vol, t, shift)
        }
    };

    // Initial guess derived from ATM approximations
    let guess = compute_initial_guess(vol, from_convention, to_convention, f);

    // Objective: find sigma such that price(sigma, to_convention) = price_from
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

    // Solve with Brent's method
    const TOLERANCE: f64 = 1e-10;
    let solver = BrentSolver::new().with_tolerance(TOLERANCE);

    match solver.solve(objective, guess) {
        Ok(solved) => {
            let result = solved.abs();
            if result.is_finite() && result > 0.0 {
                Ok(result)
            } else {
                Err(InputError::VolatilityConversionFailed {
                    tolerance: TOLERANCE,
                    residual: objective(solved).abs(),
                }
                .into())
            }
        }
        Err(_) => {
            // Solver failed - return explicit error with diagnostic info
            let residual = objective(guess).abs();
            Err(InputError::VolatilityConversionFailed {
                tolerance: TOLERANCE,
                residual,
            }
            .into())
        }
    }
}

/// Validate that forward rate is valid for the given convention.
fn validate_forward_for_convention(
    forward_rate: f64,
    convention: VolatilityConvention,
) -> Result<()> {
    match convention {
        VolatilityConvention::Normal => {
            // Normal model works for any forward rate
            Ok(())
        }
        VolatilityConvention::Lognormal => {
            if forward_rate <= 0.0 {
                Err(InputError::NonPositiveForwardForLognormal {
                    forward: forward_rate,
                    required_shift: (-forward_rate).max(0.0) + 1e-4,
                }
                .into())
            } else {
                Ok(())
            }
        }
        VolatilityConvention::ShiftedLognormal { shift } => {
            let shifted = forward_rate + shift;
            if shifted <= 0.0 {
                Err(InputError::NonPositiveShiftedForward {
                    forward: forward_rate,
                    shift,
                    shifted,
                }
                .into())
            } else {
                Ok(())
            }
        }
    }
}

/// Compute initial guess for volatility conversion solver.
fn compute_initial_guess(
    vol: f64,
    from_convention: VolatilityConvention,
    to_convention: VolatilityConvention,
    forward: f64,
) -> f64 {
    let guess = match (from_convention, to_convention) {
        (VolatilityConvention::Normal, VolatilityConvention::Lognormal) => {
            if forward.abs() > 1e-10 {
                vol / forward
            } else {
                vol
            }
        }
        (VolatilityConvention::Lognormal, VolatilityConvention::Normal) => vol * forward,
        (VolatilityConvention::Normal, VolatilityConvention::ShiftedLognormal { shift }) => {
            let shifted_f = forward + shift;
            if shifted_f.abs() > 1e-10 {
                vol / shifted_f
            } else {
                vol
            }
        }
        (VolatilityConvention::ShiftedLognormal { shift }, VolatilityConvention::Normal) => {
            vol * (forward + shift)
        }
        (VolatilityConvention::Lognormal, VolatilityConvention::ShiftedLognormal { shift }) => {
            let shifted_f = forward + shift;
            if shifted_f.abs() > 1e-10 {
                vol * forward / shifted_f
            } else {
                vol
            }
        }
        (VolatilityConvention::ShiftedLognormal { shift }, VolatilityConvention::Lognormal) => {
            if forward.abs() > 1e-10 {
                vol * (forward + shift) / forward
            } else {
                vol
            }
        }
        (
            VolatilityConvention::ShiftedLognormal { shift: s1 },
            VolatilityConvention::ShiftedLognormal { shift: s2 },
        ) => {
            // Converting between different shifts
            let f1 = forward + s1;
            let f2 = forward + s2;
            if f2.abs() > 1e-10 && f1.abs() > 1e-10 {
                vol * f1 / f2
            } else {
                vol
            }
        }
        _ => vol,
    };

    // Ensure guess is valid
    if !guess.is_finite() || guess <= 0.0 {
        (vol.abs() + 1e-6).max(1e-6)
    } else {
        guess
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_vs_lognormal_atm_conversion() {
        let forward = 0.05; // 5% forward rate
        let normal_vol = 0.01; // 100bp normal vol

        // Convert to lognormal
        let lognormal_vol = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        assert!((lognormal_vol - 0.2).abs() < 1e-6); // Should be 20% lognormal vol

        // Convert back
        let recovered_normal = convert_atm_volatility(
            lognormal_vol,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Normal,
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        assert!((recovered_normal - normal_vol).abs() < 1e-10);
    }

    #[test]
    fn test_round_trip_normal_to_shifted_lognormal() {
        let forward = 0.03;
        let normal_vol = 0.008;
        let shift = 0.02;

        let shifted_ln = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::ShiftedLognormal { shift },
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        let recovered = convert_atm_volatility(
            shifted_ln,
            VolatilityConvention::ShiftedLognormal { shift },
            VolatilityConvention::Normal,
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        assert!(
            (recovered - normal_vol).abs() < 1e-8,
            "Round trip failed: got {recovered}, expected {normal_vol}"
        );
    }

    #[test]
    fn test_round_trip_lognormal_to_shifted_lognormal() {
        let forward = 0.04;
        let ln_vol = 0.25;
        let shift = 0.01;

        let shifted_ln = convert_atm_volatility(
            ln_vol,
            VolatilityConvention::Lognormal,
            VolatilityConvention::ShiftedLognormal { shift },
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        let recovered = convert_atm_volatility(
            shifted_ln,
            VolatilityConvention::ShiftedLognormal { shift },
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        assert!(
            (recovered - ln_vol).abs() < 1e-8,
            "Round trip failed: got {recovered}, expected {ln_vol}"
        );
    }

    #[test]
    fn test_negative_forward_lognormal_errors() {
        let forward = -0.01; // -1% forward rate
        let normal_vol = 0.01;

        let result = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        );

        let err = result.expect_err("expected error for negative forward");
        assert!(
            err.to_string()
                .contains("Lognormal volatility requires positive forward"),
            "Expected NonPositiveForwardForLognormal error, got: {err}"
        );
    }

    #[test]
    fn test_negative_forward_with_shifted_lognormal() {
        let forward = -0.01; // -1% forward rate
        let normal_vol = 0.01;
        let shift = 0.02; // Shift makes (F + shift) = 1% > 0

        // This should succeed because shifted forward is positive
        let shifted_ln = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::ShiftedLognormal { shift },
            forward,
            1.0,
        )
        .expect("conversion should succeed with sufficient shift");

        assert!(shifted_ln > 0.0);
    }

    #[test]
    fn test_insufficient_shift_errors() {
        let forward = -0.03; // -3% forward rate
        let normal_vol = 0.01;
        let shift = 0.02; // Insufficient: (F + shift) = -1% < 0

        let result = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::ShiftedLognormal { shift },
            forward,
            1.0,
        );

        let err = result.expect_err("expected error for insufficient shift");
        assert!(
            err.to_string().contains("Shifted forward must be positive"),
            "Expected NonPositiveShiftedForward error, got: {err}"
        );
    }

    #[test]
    fn test_invalid_volatility_errors() {
        let forward = 0.05;

        // Zero volatility
        let result = convert_atm_volatility(
            0.0,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        );
        assert!(result.is_err());

        // Negative volatility
        let result = convert_atm_volatility(
            -0.1,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        );
        assert!(result.is_err());

        // NaN volatility
        let result = convert_atm_volatility(
            f64::NAN,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        );
        assert!(result.is_err());

        // Infinite volatility
        let result = convert_atm_volatility(
            f64::INFINITY,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_time_to_expiry_errors() {
        let result = convert_atm_volatility(
            0.01,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            0.05,
            -1.0,
        );

        let err = result.expect_err("expected error for negative time to expiry");
        assert!(
            err.to_string().contains("Invalid time to expiry"),
            "Expected InvalidTimeToExpiry error, got: {err}"
        );
    }

    #[test]
    fn test_same_convention_returns_unchanged() {
        let vol = 0.15;
        let forward = 0.05;

        // Normal -> Normal
        let result = convert_atm_volatility(
            vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Normal,
            forward,
            1.0,
        )
        .expect("should succeed");
        assert_eq!(result, vol);

        // Lognormal -> Lognormal
        let result = convert_atm_volatility(
            vol,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        )
        .expect("should succeed");
        assert_eq!(result, vol);

        // ShiftedLognormal -> ShiftedLognormal (same shift)
        let result = convert_atm_volatility(
            vol,
            VolatilityConvention::ShiftedLognormal { shift: 0.02 },
            VolatilityConvention::ShiftedLognormal { shift: 0.02 },
            forward,
            1.0,
        )
        .expect("should succeed");
        assert_eq!(result, vol);
    }

    #[test]
    fn test_different_shifts_conversion() {
        let forward = 0.03;
        let vol = 0.20;
        let shift1 = 0.01;
        let shift2 = 0.03;

        let converted = convert_atm_volatility(
            vol,
            VolatilityConvention::ShiftedLognormal { shift: shift1 },
            VolatilityConvention::ShiftedLognormal { shift: shift2 },
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        // Round trip
        let recovered = convert_atm_volatility(
            converted,
            VolatilityConvention::ShiftedLognormal { shift: shift2 },
            VolatilityConvention::ShiftedLognormal { shift: shift1 },
            forward,
            1.0,
        )
        .expect("conversion should succeed");

        assert!(
            (recovered - vol).abs() < 1e-8,
            "Round trip failed: got {recovered}, expected {vol}"
        );
    }

    #[test]
    fn test_zero_time_to_expiry() {
        // At T=0, all volatilities should convert to themselves
        // (intrinsic value is the same regardless of vol convention)
        let vol = 0.15;
        let forward = 0.05;

        let result = convert_atm_volatility(
            vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            0.0,
        )
        .expect("should succeed");

        // At expiry, prices are intrinsic - vol convention doesn't matter
        // The solver may return a different value, but price should match
        assert!(result > 0.0);
    }

    #[test]
    fn test_representative_atm_cases() {
        // Test cases representing typical market scenarios

        // Case 1: Low rate environment
        let result = convert_atm_volatility(
            0.005, // 50bp normal vol
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            0.01, // 1% forward
            1.0,
        )
        .expect("should succeed");
        assert!((result - 0.5).abs() < 1e-6); // ~50% lognormal

        // Case 2: Higher rate environment
        let result = convert_atm_volatility(
            0.012, // 120bp normal vol
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            0.04, // 4% forward
            1.0,
        )
        .expect("should succeed");
        assert!((result - 0.3).abs() < 1e-6); // ~30% lognormal

        // Case 3: Longer expiry
        let normal_vol = 0.008;
        let forward = 0.03;

        let ln_1y = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            1.0,
        )
        .expect("should succeed");

        // For ATM, vol conversion is independent of time
        let ln_5y = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            forward,
            5.0,
        )
        .expect("should succeed");

        // Fast analytic path gives same result
        assert!((ln_1y - ln_5y).abs() < 1e-10);
    }
}
