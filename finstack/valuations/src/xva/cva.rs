//! Credit Valuation Adjustment (CVA) calculation.
//!
//! Implements unilateral CVA — the expected loss due to counterparty default
//! on a portfolio of OTC derivatives.
//!
//! # Mathematical Framework
//!
//! Unilateral CVA is defined as:
//!
//! ```text
//! CVA = (1 - R) × ∫₀ᵀ EPE(t) × dPD(t)
//! ```
//!
//! In discrete form (numerical integration):
//!
//! ```text
//! CVA = (1 - R) × Σᵢ EPE(tᵢ) × [S(tᵢ₋₁) - S(tᵢ)] × DF(tᵢ)
//! ```
//!
//! where:
//! - `R` = recovery rate (typically 40% for senior unsecured)
//! - `EPE(t)` = expected positive exposure at time `t`
//! - `S(t)` = survival probability of counterparty at time `t`
//! - `DF(t)` = risk-free discount factor at time `t`
//! - `PD(t)` = marginal default probability in `[tᵢ₋₁, tᵢ]`
//!
//! # Interpretation
//!
//! - **Positive CVA** = cost to the desk (counterparty is expected to default
//!   while owing money)
//! - **CVA = 0** when recovery = 100%, or zero default probability, or zero exposure
//!
//! # Limitations
//!
//! This implementation computes **unilateral** CVA only:
//! - Does not account for own-default (DVA)
//! - Does not model wrong-way risk (exposure–default correlation)
//! - Uses point-in-time exposure (not stochastic)
//!
//! # References
//!
//! - Gregory, J. (2020). *The xVA Challenge*, 4th ed. Wiley. Chapter 14.
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models — Theory and Practice*.
//!   Springer. Chapter 21.
//! - Pykhtin, M. & Zhu, S. (2007). "A Guide to Modelling Counterparty Credit Risk."
//! - BCBS (2011). "Application of own credit risk adjustments to derivatives."
//! - Canabarro, E. & Duffie, D. (2004). "Measuring and Marking Counterparty Risk."

use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};

use super::types::{ExposureProfile, XvaResult};

/// Compute unilateral CVA from an exposure profile.
///
/// Numerically integrates the CVA formula:
///
/// ```text
/// CVA = (1 - R) × Σᵢ EPE(tᵢ) × [S(tᵢ₋₁) - S(tᵢ)] × DF(tᵢ)
/// ```
///
/// For each time bucket `[tᵢ₋₁, tᵢ]`:
/// 1. Get `EPE(tᵢ)` from the exposure profile
/// 2. Compute marginal default probability: `PD_i = S(tᵢ₋₁) - S(tᵢ)`
/// 3. Compute discount factor: `DF(tᵢ)` from the risk-free curve
/// 4. Accumulate: `(1-R) × EPE(tᵢ) × PD_i × DF(tᵢ)`
///
/// # Arguments
///
/// * `exposure_profile` - EPE/ENE profile from exposure simulation
/// * `counterparty_hazard_curve` - Hazard curve for the counterparty's credit
/// * `discount_curve` - Risk-free discount curve for present-valuing losses
/// * `recovery_rate` - Assumed recovery rate upon default (0 to 1)
///
/// # Returns
///
/// An [`XvaResult`] containing the CVA value and exposure metrics.
///
/// # Errors
///
/// Returns an error if:
/// - The exposure profile is empty
/// - Recovery rate is not in `[0, 1]`
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::xva::cva::compute_cva;
/// use finstack_valuations::xva::types::ExposureProfile;
/// use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
///
/// # fn example() -> finstack_core::Result<()> {
/// // ... construct profile, hazard_curve, discount_curve ...
/// # let profile = ExposureProfile { times: vec![], mtm_values: vec![], epe: vec![], ene: vec![] };
/// # let hazard_curve = todo!();
/// # let discount_curve = todo!();
/// let result = compute_cva(&profile, &hazard_curve, &discount_curve, 0.40)?;
/// println!("CVA = {:.2}", result.cva);
/// # Ok(())
/// # }
/// ```
pub fn compute_cva(
    exposure_profile: &ExposureProfile,
    counterparty_hazard_curve: &HazardCurve,
    discount_curve: &DiscountCurve,
    recovery_rate: f64,
) -> finstack_core::Result<XvaResult> {
    // Validate inputs
    if exposure_profile.times.is_empty() {
        return Err(finstack_core::Error::Validation(
            "CVA: exposure profile must not be empty".into(),
        ));
    }
    if !(0.0..=1.0).contains(&recovery_rate) {
        return Err(finstack_core::Error::Validation(format!(
            "CVA: recovery_rate {recovery_rate} must be in [0, 1]"
        )));
    }

    let lgd = 1.0 - recovery_rate; // Loss Given Default
    let n = exposure_profile.times.len();

    let mut cva = 0.0;
    let mut epe_profile = Vec::with_capacity(n);
    let mut ene_profile = Vec::with_capacity(n);
    let mut pfe_profile = Vec::with_capacity(n);
    let mut max_pfe: f64 = 0.0;

    // Effective EPE: non-decreasing version of EPE (Basel III SA-CCR)
    let mut effective_epe_running: f64 = 0.0;

    // Previous survival probability (S(t_{i-1})), starting at S(0) = 1.0
    let mut prev_survival = 1.0;

    for i in 0..n {
        let t = exposure_profile.times[i];
        let epe_t = exposure_profile.epe[i];
        let ene_t = exposure_profile.ene[i];

        // Survival probability at t_i
        let survival_t = counterparty_hazard_curve.sp(t);

        // Marginal default probability in [t_{i-1}, t_i]
        let marginal_pd = (prev_survival - survival_t).max(0.0);

        // Discount factor at t_i
        let df_t = discount_curve.df(t);

        // CVA contribution for this time bucket
        cva += lgd * epe_t * marginal_pd * df_t;

        // Effective EPE: non-decreasing
        effective_epe_running = effective_epe_running.max(epe_t);

        // In deterministic model, PFE = EPE (single scenario)
        let pfe_t = epe_t;
        max_pfe = max_pfe.max(pfe_t);

        epe_profile.push((t, epe_t));
        ene_profile.push((t, ene_t));
        pfe_profile.push((t, pfe_t));

        prev_survival = survival_t;
    }

    Ok(XvaResult {
        cva,
        epe_profile,
        ene_profile,
        pfe_profile,
        max_pfe,
        effective_epe: effective_epe_running,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use time::Month;

    /// Helper: build a flat hazard rate curve.
    fn flat_hazard_curve(lambda: f64) -> HazardCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        HazardCurve::builder("COUNTERPARTY")
            .base_date(base)
            .knots([(0.0, lambda), (30.0, lambda)])
            .build()
            .expect("HazardCurve should build")
    }

    /// Helper: build a flat discount curve.
    fn flat_discount_curve(rate: f64) -> DiscountCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        // Build knot points with DF = exp(-r * t)
        let knots: Vec<(f64, f64)> = (0..=60)
            .map(|i| {
                let t = i as f64 * 0.5;
                (t, (-rate * t).exp())
            })
            .collect();
        DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve should build")
    }

    /// Helper: build a uniform EPE profile.
    fn uniform_profile(epe_value: f64, times: &[f64]) -> ExposureProfile {
        ExposureProfile {
            times: times.to_vec(),
            mtm_values: times.iter().map(|_| epe_value).collect(),
            epe: times.iter().map(|_| epe_value).collect(),
            ene: times.iter().map(|_| 0.0).collect(),
        }
    }

    // ── CVA formula tests ────────────────────────────────────────

    #[test]
    fn cva_zero_hazard_rate_gives_zero_cva() {
        // With λ=0, survival is always 1.0, so marginal PD = 0 at every step
        // → CVA = 0
        let hazard = flat_hazard_curve(0.0);
        let discount = flat_discount_curve(0.05);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(1_000_000.0, &times);

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        assert!(
            result.cva.abs() < 1e-6,
            "CVA with zero hazard rate should be zero, got {}",
            result.cva
        );
    }

    #[test]
    fn cva_full_recovery_gives_zero_cva() {
        // With R=1.0, LGD = 0 → CVA = 0 regardless of default probability
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.05);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(1_000_000.0, &times);

        let result = compute_cva(&profile, &hazard, &discount, 1.0)
            .expect("CVA computation should work with R=1");

        assert!(
            result.cva.abs() < 1e-12,
            "CVA with 100% recovery should be zero, got {}",
            result.cva
        );
    }

    #[test]
    fn cva_zero_exposure_gives_zero_cva() {
        // With EPE = 0, CVA = 0 regardless of credit quality
        let hazard = flat_hazard_curve(0.05);
        let discount = flat_discount_curve(0.05);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(0.0, &times);

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        assert!(
            result.cva.abs() < 1e-12,
            "CVA with zero exposure should be zero, got {}",
            result.cva
        );
    }

    #[test]
    fn cva_positive_for_nonzero_exposure_and_default_risk() {
        // Non-trivial case: positive exposure, positive hazard rate
        let hazard = flat_hazard_curve(0.02); // ~2% annual default intensity
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(1_000_000.0, &times);

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        assert!(
            result.cva > 0.0,
            "CVA should be positive for non-zero exposure and hazard rate, got {}",
            result.cva
        );

        // Sanity check: CVA should be less than (1-R) * EPE * total default probability
        // Upper bound: (1-R) × EPE × (1 - S(T))
        let total_dp = 1.0 - hazard.sp(10.0); // last time = 10.0
        let upper_bound = 0.60 * 1_000_000.0 * total_dp;
        assert!(
            result.cva < upper_bound,
            "CVA {} should be less than upper bound {} = (1-R) × EPE × total_PD",
            result.cva,
            upper_bound
        );
    }

    #[test]
    fn cva_with_flat_hazard_analytical_check() {
        // For constant EPE, flat hazard rate λ, flat discount rate r:
        //
        // CVA ≈ (1-R) × EPE × Σᵢ [S(tᵢ₋₁) - S(tᵢ)] × DF(tᵢ)
        //
        // For fine grid, this converges to:
        // CVA ≈ (1-R) × EPE × ∫₀ᵀ λ×e^{-λt} × e^{-rt} dt
        //     = (1-R) × EPE × λ/(λ+r) × (1 - e^{-(λ+r)T})
        let lambda = 0.02;
        let r = 0.03;
        let recovery = 0.40;
        let epe = 1_000_000.0;
        let t_max = 10.0;

        // Fine grid for accurate numerical integration
        let dt: f64 = 0.25;
        let n_steps = (t_max / dt).round() as usize;
        let times: Vec<f64> = (1..=n_steps).map(|i| i as f64 * dt).collect();

        let hazard = flat_hazard_curve(lambda);
        let discount = flat_discount_curve(r);
        let profile = uniform_profile(epe, &times);

        let result = compute_cva(&profile, &hazard, &discount, recovery)
            .expect("CVA computation should work");

        // Analytical formula
        let lgd = 1.0 - recovery;
        let analytical = lgd * epe * lambda / (lambda + r) * (1.0 - (-(lambda + r) * t_max).exp());

        // Allow 5% relative tolerance due to discrete integration
        let rel_error = (result.cva - analytical).abs() / analytical;
        assert!(
            rel_error < 0.05,
            "CVA numerical ({:.2}) should be close to analytical ({:.2}), rel_error={:.4}",
            result.cva,
            analytical,
            rel_error
        );
    }

    #[test]
    fn cva_profile_lengths_match() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let profile = uniform_profile(100.0, &times);

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        assert_eq!(result.epe_profile.len(), times.len());
        assert_eq!(result.ene_profile.len(), times.len());
        assert_eq!(result.pfe_profile.len(), times.len());
    }

    #[test]
    fn effective_epe_is_non_decreasing() {
        // Effective EPE should be the running maximum of EPE
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: vec![100.0, 200.0, 50.0, 150.0, 80.0],
            epe: vec![100.0, 200.0, 50.0, 150.0, 80.0],
            ene: vec![0.0; 5],
        };

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        // Effective EPE should be max of all EPE values = 200
        assert!(
            (result.effective_epe - 200.0).abs() < 1e-12,
            "Effective EPE should be 200.0, got {}",
            result.effective_epe
        );
    }

    #[test]
    fn max_pfe_equals_max_epe_in_deterministic_model() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times = vec![1.0, 2.0, 3.0];
        let profile = ExposureProfile {
            times,
            mtm_values: vec![100.0, 300.0, 200.0],
            epe: vec![100.0, 300.0, 200.0],
            ene: vec![0.0; 3],
        };

        let result =
            compute_cva(&profile, &hazard, &discount, 0.40).expect("CVA computation should work");

        assert!(
            (result.max_pfe - 300.0).abs() < 1e-12,
            "Max PFE should equal max EPE in deterministic model, got {}",
            result.max_pfe
        );
    }

    #[test]
    fn cva_rejects_empty_profile() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let profile = ExposureProfile {
            times: vec![],
            mtm_values: vec![],
            epe: vec![],
            ene: vec![],
        };

        let result = compute_cva(&profile, &hazard, &discount, 0.40);
        assert!(result.is_err(), "Should reject empty profile");
    }

    #[test]
    fn cva_rejects_invalid_recovery_rate() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let profile = uniform_profile(100.0, &[1.0, 2.0]);

        assert!(compute_cva(&profile, &hazard, &discount, -0.1).is_err());
        assert!(compute_cva(&profile, &hazard, &discount, 1.5).is_err());
    }

    #[test]
    fn cva_higher_hazard_gives_higher_cva() {
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(1_000_000.0, &times);

        let hazard_low = flat_hazard_curve(0.01);
        let hazard_high = flat_hazard_curve(0.05);

        let cva_low = compute_cva(&profile, &hazard_low, &discount, 0.40)
            .expect("should work")
            .cva;
        let cva_high = compute_cva(&profile, &hazard_high, &discount, 0.40)
            .expect("should work")
            .cva;

        assert!(
            cva_high > cva_low,
            "Higher hazard rate should give higher CVA: low={cva_low}, high={cva_high}"
        );
    }

    #[test]
    fn cva_lower_recovery_gives_higher_cva() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();
        let profile = uniform_profile(1_000_000.0, &times);

        let cva_high_r = compute_cva(&profile, &hazard, &discount, 0.60)
            .expect("should work")
            .cva;
        let cva_low_r = compute_cva(&profile, &hazard, &discount, 0.20)
            .expect("should work")
            .cva;

        assert!(
            cva_low_r > cva_high_r,
            "Lower recovery should give higher CVA: R=0.20 → {cva_low_r}, R=0.60 → {cva_high_r}"
        );
    }
}
