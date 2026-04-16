//! Innovation distributions for GARCH likelihood computation.
//!
//! Provides Gaussian and Student-t standardized innovation distributions,
//! including log-PDF evaluation and expected absolute value calculations
//! required by EGARCH models.

use finstack_core::math::special_functions::ln_gamma;

/// Innovation distribution for GARCH likelihood.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InnovationDist {
    /// Standard normal innovations.
    Gaussian,
    /// Student-t innovations with estimated degrees of freedom.
    /// The `f64` field is the degrees-of-freedom parameter (nu > 2).
    StudentT(f64),
}

impl InnovationDist {
    /// Log-PDF of a standardized innovation z under this distribution.
    ///
    /// For Gaussian: -0.5 * (ln(2pi) + z^2)
    /// For Student-t(nu): ln(Gamma((nu+1)/2)) - ln(Gamma(nu/2))
    ///                     - 0.5*ln(pi*(nu-2)) - ((nu+1)/2)*ln(1 + z^2/(nu-2))
    #[must_use]
    pub fn log_pdf(self, z: f64) -> f64 {
        match self {
            Self::Gaussian => -0.5 * (std::f64::consts::TAU.ln() + z * z),
            Self::StudentT(nu) => {
                let half_nu_plus_1 = 0.5 * (nu + 1.0);
                let half_nu = 0.5 * nu;
                ln_gamma(half_nu_plus_1)
                    - ln_gamma(half_nu)
                    - 0.5 * (std::f64::consts::PI * (nu - 2.0)).ln()
                    - half_nu_plus_1 * (1.0 + z * z / (nu - 2.0)).ln()
            }
        }
    }

    /// Expected value of |z| under this distribution.
    ///
    /// Used by EGARCH for the centering term E\[|z|\].
    /// Gaussian: sqrt(2/pi). Student-t(nu): computed from Beta function.
    #[must_use]
    pub fn expected_abs(self) -> f64 {
        match self {
            Self::Gaussian => {
                // E[|Z|] = sqrt(2/pi) for Z ~ N(0,1)
                (2.0 / std::f64::consts::PI).sqrt()
            }
            Self::StudentT(nu) => {
                // E[|T|] = sqrt(nu-2) * Gamma((nu-1)/2) / (sqrt(pi) * Gamma(nu/2))
                // for T ~ standardized t with variance 1 (i.e., T/sqrt(nu/(nu-2)) has var 1)
                // For the standardized t with unit variance:
                // E[|Z|] = sqrt((nu-2)/pi) * Gamma((nu-1)/2) / Gamma(nu/2)
                let half_nu = 0.5 * nu;
                let half_nu_minus_1 = 0.5 * (nu - 1.0);
                ((nu - 2.0) / std::f64::consts::PI).sqrt()
                    * (ln_gamma(half_nu_minus_1) - ln_gamma(half_nu)).exp()
            }
        }
    }

    /// Number of distribution parameters to estimate (0 for Gaussian, 1 for Student-t).
    #[must_use]
    pub fn num_params(self) -> usize {
        match self {
            Self::Gaussian => 0,
            Self::StudentT(_) => 1,
        }
    }

    /// Lower bound on degrees of freedom (Student-t only; must be > 2 for finite variance).
    #[must_use]
    pub fn dof_lower_bound() -> f64 {
        2.01
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_log_pdf_at_zero() {
        let lp = InnovationDist::Gaussian.log_pdf(0.0);
        let expected = -0.5 * std::f64::consts::TAU.ln();
        assert!((lp - expected).abs() < 1e-12);
    }

    #[test]
    fn gaussian_log_pdf_symmetry() {
        let lp_pos = InnovationDist::Gaussian.log_pdf(1.5);
        let lp_neg = InnovationDist::Gaussian.log_pdf(-1.5);
        assert!((lp_pos - lp_neg).abs() < 1e-12);
    }

    #[test]
    fn gaussian_expected_abs() {
        let ea = InnovationDist::Gaussian.expected_abs();
        let expected = (2.0 / std::f64::consts::PI).sqrt();
        assert!((ea - expected).abs() < 1e-12);
    }

    #[test]
    fn student_t_log_pdf_finite() {
        let dist = InnovationDist::StudentT(5.0);
        let lp = dist.log_pdf(0.0);
        assert!(lp.is_finite());
        assert!(lp < 0.0);
    }

    #[test]
    fn student_t_heavier_tails() {
        let z = 3.0;
        let gauss_lp = InnovationDist::Gaussian.log_pdf(z);
        let t_lp = InnovationDist::StudentT(5.0).log_pdf(z);
        // Student-t should assign more probability to tails
        assert!(t_lp > gauss_lp);
    }

    #[test]
    fn student_t_expected_abs_positive() {
        let ea = InnovationDist::StudentT(5.0).expected_abs();
        assert!(ea > 0.0);
        assert!(ea.is_finite());
    }

    #[test]
    fn num_params_correct() {
        assert_eq!(InnovationDist::Gaussian.num_params(), 0);
        assert_eq!(InnovationDist::StudentT(5.0).num_params(), 1);
    }
}
