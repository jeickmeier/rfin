//! Credit Valuation Adjustment (CVA) calculation.
//!
//! CVA represents the expected loss due to counterparty default.

use super::exposure::ExposureProfile;

/// Survival curve interface.
///
/// Provides survival probability S(t) and hazard rate λ(t).
pub trait SurvivalCurve: Send + Sync {
    /// Survival probability from 0 to t.
    fn survival_probability(&self, t: f64) -> f64;

    /// Hazard rate at time t.
    fn hazard_rate(&self, t: f64) -> f64;

    /// Default probability in interval (s, t].
    fn default_probability(&self, s: f64, t: f64) -> f64 {
        self.survival_probability(s) - self.survival_probability(t)
    }
}

/// Flat hazard rate curve (constant intensity).
#[derive(Clone, Debug)]
pub struct FlatHazardCurve {
    /// Constant hazard rate λ
    pub lambda: f64,
}

impl FlatHazardCurve {
    /// Create a flat hazard curve.
    pub fn new(lambda: f64) -> Self {
        Self { lambda }
    }

    /// Create from credit spread and recovery rate.
    ///
    /// λ ≈ CDS spread / (1 - recovery)
    pub fn from_cds_spread(spread: f64, recovery: f64) -> Self {
        let lambda = spread / (1.0 - recovery).max(0.01);
        Self { lambda }
    }
}

impl SurvivalCurve for FlatHazardCurve {
    fn survival_probability(&self, t: f64) -> f64 {
        (-self.lambda * t).exp()
    }

    fn hazard_rate(&self, _t: f64) -> f64 {
        self.lambda
    }
}

/// CVA calculation result.
#[derive(Clone, Debug)]
pub struct CvaResult {
    /// Credit Valuation Adjustment
    pub cva: f64,
    /// Breakdown by time bucket
    pub time_buckets: Vec<f64>,
    /// Average Expected Positive Exposure
    pub average_epe: f64,
}

/// Calculate CVA from exposure profile and credit curves.
///
/// # Arguments
///
/// * `exposure_profile` - EE and PFE over time
/// * `survival_curve` - Counterparty survival probabilities
/// * `discount_curve` - Risk-free discount factors
/// * `recovery_rate` - Recovery rate on default (e.g., 0.40 for 40%)
///
/// # Returns
///
/// CVA result with total and breakdown
///
/// # Formula
///
/// ```text
/// CVA = LGD * Σ_i EE(t_i) * [S(t_{i-1}) - S(t_i)] * DF(t_i)
///     = (1 - R) * Σ_i EE(t_i) * PD(t_{i-1}, t_i) * DF(t_i)
/// ```
pub fn calculate_cva(
    exposure_profile: &ExposureProfile,
    survival_curve: &dyn SurvivalCurve,
    discount_factors: &[f64],
    recovery_rate: f64,
) -> CvaResult {
    assert_eq!(exposure_profile.times.len(), discount_factors.len());

    let lgd = 1.0 - recovery_rate; // Loss Given Default
    let num_points = exposure_profile.num_points();

    let mut cva_total = 0.0;
    let mut time_buckets = Vec::with_capacity(num_points);

    // CVA calculation over time buckets
    #[allow(clippy::needless_range_loop)]
    for i in 1..num_points {
        let t_prev = exposure_profile.times[i - 1];
        let t = exposure_profile.times[i];

        // Expected Exposure at time t
        let ee = exposure_profile.epe[i];

        // Default probability in interval (t_{i-1}, t_i]
        let pd = survival_curve.default_probability(t_prev, t);

        // Discount factor
        let df = discount_factors[i];

        // CVA contribution for this bucket
        let cva_bucket = lgd * ee * pd * df;

        cva_total += cva_bucket;
        time_buckets.push(cva_bucket);
    }

    CvaResult {
        cva: cva_total,
        time_buckets,
        average_epe: exposure_profile.average_epe(),
    }
}

/// Calculate DVA (Debit Valuation Adjustment).
///
/// DVA is the mirror of CVA from own default perspective.
///
/// # Formula
///
/// ```text
/// DVA = LGD_own * Σ_i ENE(t_i) * PD_own(t_{i-1}, t_i) * DF(t_i)
/// ```
pub fn calculate_dva(
    exposure_profile: &ExposureProfile,
    own_survival_curve: &dyn SurvivalCurve,
    discount_factors: &[f64],
    own_recovery_rate: f64,
) -> f64 {
    let lgd = 1.0 - own_recovery_rate;
    let num_points = exposure_profile.num_points();

    let mut dva_total = 0.0;

    #[allow(clippy::needless_range_loop)]
    for i in 1..num_points {
        let t_prev = exposure_profile.times[i - 1];
        let t = exposure_profile.times[i];

        // Expected Negative Exposure (we owe counterparty)
        let ene = exposure_profile.ene[i];

        // Own default probability
        let pd = own_survival_curve.default_probability(t_prev, t);

        // Discount factor
        let df = discount_factors[i];

        dva_total += lgd * ene * pd * df;
    }

    dva_total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_hazard_curve() {
        let curve = FlatHazardCurve::new(0.02); // 2% hazard rate

        // Survival at t=0 should be 1
        assert_eq!(curve.survival_probability(0.0), 1.0);

        // Survival at t=1 should be exp(-0.02)
        assert!((curve.survival_probability(1.0) - (-0.02_f64).exp()).abs() < 1e-10);

        // Hazard rate constant
        assert_eq!(curve.hazard_rate(0.5), 0.02);
    }

    #[test]
    fn test_flat_hazard_from_cds_spread() {
        let spread = 0.01; // 100 bp
        let recovery = 0.40;
        let curve = FlatHazardCurve::from_cds_spread(spread, recovery);

        // λ = spread / (1 - R) = 0.01 / 0.6 ≈ 0.01667
        assert!((curve.lambda - 0.01667).abs() < 0.001);
    }

    #[test]
    fn test_calculate_cva_simple() {
        // Simple exposure profile
        let times = vec![0.0, 1.0, 2.0];
        let mut profile = ExposureProfile::new(times);
        profile.epe = vec![0.0, 10.0, 5.0];
        profile.ene = vec![0.0, 0.0, 0.0];

        // Flat hazard and discount
        let survival = FlatHazardCurve::new(0.02);
        let discount_factors = vec![1.0, 0.95, 0.90];
        let recovery = 0.40;

        let result = calculate_cva(&profile, &survival, &discount_factors, recovery);

        println!("CVA Result:");
        println!("  Total CVA: {:.6}", result.cva);
        println!("  Average EPE: {:.6}", result.average_epe);

        // CVA should be positive
        assert!(result.cva > 0.0);

        // Average EPE
        assert!((result.average_epe - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_dva_simple() {
        let times = vec![0.0, 1.0, 2.0];
        let mut profile = ExposureProfile::new(times);
        profile.epe = vec![0.0, 0.0, 0.0];
        profile.ene = vec![0.0, 8.0, 12.0]; // We owe counterparty

        let own_survival = FlatHazardCurve::new(0.01);
        let discount_factors = vec![1.0, 0.95, 0.90];
        let own_recovery = 0.40;

        let dva = calculate_dva(&profile, &own_survival, &discount_factors, own_recovery);

        println!("DVA: {:.6}", dva);

        // DVA should be positive (benefit from our potential default)
        assert!(dva > 0.0);
    }
}

