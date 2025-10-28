//! Exposure calculation for xVA.
//!
//! Computes Expected Exposure (EE), Expected Negative Exposure (ENE),
//! and Potential Future Exposure (PFE) from Monte Carlo paths.

/// Exposure profile over time.
///
/// Contains exposure metrics at discrete time points.
#[derive(Clone, Debug)]
pub struct ExposureProfile {
    /// Time points (in years)
    pub times: Vec<f64>,
    /// Expected Positive Exposure at each time
    pub epe: Vec<f64>,
    /// Expected Negative Exposure at each time
    pub ene: Vec<f64>,
    /// Potential Future Exposure at 95% confidence
    pub pfe_95: Vec<f64>,
    /// Potential Future Exposure at 99% confidence
    pub pfe_99: Vec<f64>,
}

impl ExposureProfile {
    /// Create a new exposure profile.
    pub fn new(times: Vec<f64>) -> Self {
        let n = times.len();
        Self {
            times,
            epe: vec![0.0; n],
            ene: vec![0.0; n],
            pfe_95: vec![0.0; n],
            pfe_99: vec![0.0; n],
        }
    }

    /// Get number of time points.
    pub fn num_points(&self) -> usize {
        self.times.len()
    }

    /// Get maximum EPE (peak exposure).
    pub fn max_epe(&self) -> f64 {
        self.epe
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Get time-averaged EPE.
    pub fn average_epe(&self) -> f64 {
        if self.epe.is_empty() {
            return 0.0;
        }
        self.epe.iter().sum::<f64>() / self.epe.len() as f64
    }
}

/// Calculate exposure profile from Monte Carlo paths.
///
/// # Arguments
///
/// * `path_values` - Matrix of path values [path][time]
/// * `times` - Time points corresponding to path values
///
/// # Returns
///
/// Exposure profile with EE, ENE, and PFE metrics
pub fn calculate_exposure_profile(path_values: &[Vec<f64>], times: Vec<f64>) -> ExposureProfile {
    let num_paths = path_values.len();
    let num_times = times.len();

    assert!(
        path_values.iter().all(|path| path.len() == num_times),
        "All paths must have same length as times"
    );

    let mut profile = ExposureProfile::new(times);

    // For each time point, compute exposure statistics across paths
    for t in 0..num_times {
        let mut values_at_t: Vec<f64> = path_values.iter().map(|path| path[t]).collect();

        // EE = E[max(V, 0)]
        let positive_exposures: Vec<f64> = values_at_t.iter().map(|&v| v.max(0.0)).collect();
        profile.epe[t] = positive_exposures.iter().sum::<f64>() / num_paths as f64;

        // ENE = E[max(-V, 0)] = E[min(V, 0)]
        let negative_exposures: Vec<f64> = values_at_t.iter().map(|&v| (-v).max(0.0)).collect();
        profile.ene[t] = negative_exposures.iter().sum::<f64>() / num_paths as f64;

        // PFE: percentiles of positive exposure
        values_at_t.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // 95th percentile
        let idx_95 = (num_paths as f64 * 0.95) as usize;
        profile.pfe_95[t] = values_at_t[idx_95.min(num_paths - 1)].max(0.0);

        // 99th percentile
        let idx_99 = (num_paths as f64 * 0.99) as usize;
        profile.pfe_99[t] = values_at_t[idx_99.min(num_paths - 1)].max(0.0);
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exposure_profile_creation() {
        let times = vec![0.0, 0.5, 1.0];
        let profile = ExposureProfile::new(times);

        assert_eq!(profile.num_points(), 3);
        assert_eq!(profile.times[0], 0.0);
        assert_eq!(profile.times[2], 1.0);
    }

    #[test]
    fn test_calculate_exposure_simple() {
        // Simple test with 3 paths
        let path_values = vec![
            vec![0.0, 10.0, 20.0],  // Path 1
            vec![0.0, -5.0, 15.0],  // Path 2
            vec![0.0, 15.0, -10.0], // Path 3
        ];
        let times = vec![0.0, 0.5, 1.0];

        let profile = calculate_exposure_profile(&path_values, times);

        // At t=0: all paths at 0
        assert_eq!(profile.epe[0], 0.0);

        // At t=0.5: values are [10, -5, 15], positives are [10, 0, 15]
        // EE = (10 + 0 + 15) / 3 = 8.33...
        assert!((profile.epe[1] - 8.333).abs() < 0.01);

        // ENE at t=0.5: negatives are [0, 5, 0]
        // ENE = 5 / 3 = 1.666...
        assert!((profile.ene[1] - 1.666).abs() < 0.01);

        println!("Exposure profile:");
        for (i, &t) in profile.times.iter().enumerate() {
            println!(
                "  t={:.1}: EE={:.2}, ENE={:.2}, PFE95={:.2}",
                t, profile.epe[i], profile.ene[i], profile.pfe_95[i]
            );
        }
    }

    #[test]
    fn test_max_and_average_epe() {
        let path_values = vec![
            vec![0.0, 5.0, 10.0],
            vec![0.0, 8.0, 3.0],
        ];
        let times = vec![0.0, 0.5, 1.0];

        let profile = calculate_exposure_profile(&path_values, times);

        // Max EPE
        let max_epe = profile.max_epe();
        assert!(max_epe > 0.0);

        // Average EPE
        let avg_epe = profile.average_epe();
        assert!(avg_epe > 0.0);
        assert!(avg_epe <= max_epe);

        println!("Max EPE: {:.2}, Average EPE: {:.2}", max_epe, avg_epe);
    }
}

