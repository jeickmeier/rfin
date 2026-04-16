//! Calendar spread arbitrage check: total variance monotonicity in expiry.
//!
//! Verifies that total variance is non-decreasing in maturity at each strike.
//! A violation means a longer-dated option is cheaper than a shorter-dated one,
//! allowing a riskless calendar spread profit.
//!
//! # Financial Background
//!
//! For European options with the same strike, a longer-dated option must be
//! worth at least as much as a shorter-dated one (assuming non-negative rates).
//! In total-variance space this means w(k, T2) >= w(k, T1) for T2 > T1.

use super::{classify_severity, ArbitrageCheck};
use crate::market_data::arbitrage::types::{ArbitrageType, ArbitrageViolation, ViolationLocation};
use crate::market_data::surfaces::VolSurface;

/// Checks that total variance is non-decreasing in expiry at each strike:
/// w(k, T2) >= w(k, T1) for T2 > T1.
///
/// A violation means a longer-dated option is cheaper than a shorter-dated
/// one at the same strike, allowing a riskless calendar spread profit.
pub struct CalendarSpreadCheck {
    /// Tolerance below which a decrease is classified as Negligible.
    pub tolerance: f64,
}

impl Default for CalendarSpreadCheck {
    fn default() -> Self {
        Self { tolerance: 1e-10 }
    }
}

impl ArbitrageCheck for CalendarSpreadCheck {
    fn name(&self) -> &str {
        "Calendar Spread"
    }

    fn check(&self, surface: &VolSurface) -> Vec<ArbitrageViolation> {
        let expiries = surface.expiries();
        let strikes = surface.strikes();
        let mut violations = Vec::new();

        if expiries.len() < 2 {
            return violations;
        }

        for &k in strikes {
            let mut prev_w = 0.0_f64;
            let mut prev_t = 0.0_f64;

            for &t in expiries {
                let v = surface.value_clamped(t, k);
                let w = v * v * t;

                if w < prev_w - self.tolerance {
                    let decrease = prev_w - w;
                    let severity = classify_severity(decrease, 1e-8, 1e-5, 1e-3);
                    violations.push(ArbitrageViolation {
                        violation_type: ArbitrageType::CalendarSpread,
                        location: ViolationLocation {
                            strike: k,
                            expiry: t,
                            adjacent_expiry: Some(prev_t),
                        },
                        severity,
                        magnitude: decrease,
                        description: format!(
                            "Calendar spread arbitrage at K={k:.2}: \
                            w(T={t:.4}) = {w:.6} < w(T={prev_t:.4}) = {prev_w:.6}"
                        ),
                        suggested_fix: None,
                    });
                }

                prev_w = w;
                prev_t = t;
            }
        }

        violations
    }
}
