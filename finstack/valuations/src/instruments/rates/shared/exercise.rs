//! Exercise boundary protocol for LSMC-priced callable rate exotics.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_monte_carlo::traits::Payoff;

/// Additional contract a `Payoff` must implement to be priced via LSMC
/// in [`crate::instruments::rates::shared::hw1f_lsmc::RateExoticHw1fLsmcPricer`].
///
/// The harness handles path simulation, discounting, and backward regression;
/// each product implements the three product-specific hooks below.
pub trait ExerciseBoundaryPayoff: Payoff {
    /// The intrinsic value (i.e., "what the issuer receives on call") at the
    /// specified exercise-date index, evaluated along a single path whose
    /// state at that date is `short_rate`.
    ///
    /// For a note callable at par, this is typically `notional * call_price`
    /// minus the PV of future deterministic coupons available on-path.
    fn intrinsic_at(&self, exercise_idx: usize, short_rate: f64, currency: Currency) -> Money;

    /// Regression basis used for continuation-value estimation at the
    /// specified exercise date. Standard basis is `[1, r, r², t·r]`.
    /// Longer basis improves accuracy but adds variance.
    fn continuation_basis(&self, exercise_idx: usize, t_years: f64, short_rate: f64) -> Vec<f64>;

    /// Whether the path has reached a state where exercise is not allowed
    /// (e.g., knocked out). When `true`, the path is excluded from regression.
    fn is_path_inactive(&self) -> bool {
        false
    }
}

/// Standard degree-2 regression basis `[1, r, r², t·r]`.
pub fn standard_basis(t_years: f64, short_rate: f64) -> Vec<f64> {
    vec![
        1.0,
        short_rate,
        short_rate * short_rate,
        t_years * short_rate,
    ]
}

/// Degree-3 regression basis `[1, r, r², r³, t·r, t·r²]`.
pub fn extended_basis(t_years: f64, short_rate: f64) -> Vec<f64> {
    let r = short_rate;
    vec![1.0, r, r * r, r * r * r, t_years * r, t_years * r * r]
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn standard_basis_dimension() {
        assert_eq!(standard_basis(0.5, 0.03).len(), 4);
    }

    #[test]
    fn extended_basis_dimension() {
        assert_eq!(extended_basis(0.5, 0.03).len(), 6);
    }

    #[test]
    fn basis_values_are_finite() {
        for v in standard_basis(2.0, 0.04) {
            assert!(v.is_finite());
        }
    }
}
