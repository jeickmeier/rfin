//! Butterfly arbitrage check: strike-convexity of total variance.
//!
//! Verifies that call prices are convex in strike, or equivalently that the
//! implied probability density is non-negative. Operates in total-variance
//! space using discrete second differences on the strike grid.
//!
//! # Financial Background
//!
//! A butterfly spread at strike K consists of buying calls at K-d and K+d
//! while selling two calls at K. No-arbitrage requires this portfolio to have
//! non-negative value, which is equivalent to convexity of call prices in
//! strike. In total-variance space, this translates to non-negative second
//! derivative of w(k) with respect to strike.

use super::{classify_severity, ArbitrageCheck};
use crate::market_data::arbitrage::types::{ArbitrageType, ArbitrageViolation, ViolationLocation};
use crate::market_data::surfaces::VolSurface;

/// Checks that call prices are convex in strike (equivalently, that the
/// implied probability density is non-negative).
///
/// Operates in total-variance space: for each expiry slice, verifies that
/// w(k) is convex by checking the discrete second derivative at each
/// interior strike:
///
/// ```text
/// d2w = (w(k+dk) - 2*w(k) + w(k-dk)) / (dk * dk)
/// ```
///
/// A negative d2w at any point implies butterfly arbitrage.
pub struct ButterflyCheck {
    /// Tolerance below which a negative d2w is classified as Negligible.
    pub tolerance: f64,
}

impl Default for ButterflyCheck {
    fn default() -> Self {
        Self { tolerance: 1e-10 }
    }
}

impl ArbitrageCheck for ButterflyCheck {
    fn name(&self) -> &str {
        "Butterfly"
    }

    fn check(&self, surface: &VolSurface) -> Vec<ArbitrageViolation> {
        let expiries = surface.expiries();
        let strikes = surface.strikes();
        let mut violations = Vec::new();

        if strikes.len() < 3 {
            return violations;
        }

        for &t in expiries {
            // Compute total variance at each strike for this expiry
            let ws: Vec<f64> = strikes
                .iter()
                .map(|&k| {
                    let v = surface.value_clamped(t, k);
                    v * v * t
                })
                .collect();

            // Check discrete second derivative at each interior strike
            for i in 1..strikes.len() - 1 {
                let dk_minus = strikes[i] - strikes[i - 1];
                let dk_plus = strikes[i + 1] - strikes[i];
                let dk_avg = 0.5 * (dk_minus + dk_plus);

                // Non-uniform second derivative
                let d2w = (ws[i + 1] / dk_plus - ws[i] * (1.0 / dk_plus + 1.0 / dk_minus)
                    + ws[i - 1] / dk_minus)
                    / dk_avg;

                if d2w < -self.tolerance {
                    let magnitude = -d2w;
                    let severity = classify_severity(magnitude, 1e-8, 1e-5, 1e-3);
                    violations.push(ArbitrageViolation {
                        violation_type: ArbitrageType::Butterfly,
                        location: ViolationLocation {
                            strike: strikes[i],
                            expiry: t,
                            adjacent_expiry: None,
                        },
                        severity,
                        magnitude,
                        description: format!(
                            "Butterfly arbitrage at T={t:.4}, K={:.2}: d2w/dk2 = {d2w:.2e}",
                            strikes[i]
                        ),
                        suggested_fix: None,
                    });
                }
            }
        }

        violations
    }
}
