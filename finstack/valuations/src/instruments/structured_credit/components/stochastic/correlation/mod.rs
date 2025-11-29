//! Correlation structures for structured credit.
//!
//! This module provides correlation specifications that capture:
//! - Asset correlation (intra-pool default correlation)
//! - Prepay-default correlation (typically negative)
//! - Sector correlation (intra vs inter-sector)
//!
//! # Industry Standard Calibrations
//!
//! ## RMBS
//! - Asset correlation: 5-10% (diversified mortgage pools)
//! - Prepay-default correlation: -20% to -40% (refi incentive vs credit)
//!
//! ## CLO
//! - Intra-sector correlation: 25-35%
//! - Inter-sector correlation: 10-15%
//! - Prepay-default correlation: -15% to -25%
//!
//! ## CMBS
//! - Asset correlation: 15-25% (concentrated property types)
//! - Prepay-default correlation: -10% to -20%

/// Correlation structure specification.
///
/// Captures the various correlation parameters needed for
/// stochastic structured credit modeling.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "structure", deny_unknown_fields))]
pub enum CorrelationStructure {
    /// Flat (homogeneous) correlation structure.
    ///
    /// All assets have the same pairwise correlation.
    Flat {
        /// Asset (default) correlation
        asset_correlation: f64,
        /// Correlation between prepayment and default factors
        prepay_default_correlation: f64,
    },

    /// Sectored correlation structure.
    ///
    /// Different correlation within vs. between sectors.
    Sectored {
        /// Intra-sector asset correlation
        intra_sector: f64,
        /// Inter-sector asset correlation
        inter_sector: f64,
        /// Prepay-default correlation
        prepay_default: f64,
    },

    /// Full correlation matrix.
    ///
    /// Custom correlation structure for bespoke deals.
    Matrix {
        /// Flattened correlation matrix (row-major)
        correlations: Vec<f64>,
        /// Labels for matrix rows/columns
        labels: Vec<String>,
    },
}

impl Default for CorrelationStructure {
    fn default() -> Self {
        CorrelationStructure::Flat {
            asset_correlation: 0.10,
            prepay_default_correlation: -0.20,
        }
    }
}

impl CorrelationStructure {
    /// Create a flat correlation structure.
    pub fn flat(asset_correlation: f64, prepay_default_correlation: f64) -> Self {
        CorrelationStructure::Flat {
            asset_correlation: asset_correlation.clamp(0.0, 0.99),
            prepay_default_correlation: prepay_default_correlation.clamp(-0.99, 0.99),
        }
    }

    /// Create a sectored correlation structure.
    pub fn sectored(intra_sector: f64, inter_sector: f64, prepay_default: f64) -> Self {
        CorrelationStructure::Sectored {
            intra_sector: intra_sector.clamp(0.0, 0.99),
            inter_sector: inter_sector.clamp(0.0, 0.99),
            prepay_default: prepay_default.clamp(-0.99, 0.99),
        }
    }

    /// Create a custom matrix correlation structure.
    pub fn matrix(correlations: Vec<f64>, labels: Vec<String>) -> Self {
        // Validate matrix is square
        let n = labels.len();
        let expected_size = n * n;
        let corrs = if correlations.len() == expected_size {
            correlations
        } else {
            // Create identity matrix as fallback
            let mut identity = vec![0.0; expected_size];
            for i in 0..n {
                identity[i * n + i] = 1.0;
            }
            identity
        };

        CorrelationStructure::Matrix {
            correlations: corrs,
            labels,
        }
    }

    /// RMBS standard correlation structure.
    ///
    /// Low asset correlation (diversified pool), moderate negative
    /// prepay-default correlation (refi vs credit).
    pub fn rmbs_standard() -> Self {
        CorrelationStructure::Flat {
            asset_correlation: 0.05,
            prepay_default_correlation: -0.30,
        }
    }

    /// CLO standard correlation structure.
    ///
    /// Sectored structure with higher intra-sector correlation.
    pub fn clo_standard() -> Self {
        CorrelationStructure::Sectored {
            intra_sector: 0.30,
            inter_sector: 0.10,
            prepay_default: -0.20,
        }
    }

    /// CMBS standard correlation structure.
    ///
    /// Moderate correlation (concentrated property types).
    pub fn cmbs_standard() -> Self {
        CorrelationStructure::Flat {
            asset_correlation: 0.20,
            prepay_default_correlation: -0.15,
        }
    }

    /// ABS auto loan standard correlation structure.
    pub fn abs_auto_standard() -> Self {
        CorrelationStructure::Flat {
            asset_correlation: 0.08,
            prepay_default_correlation: -0.10,
        }
    }

    /// Get the asset (default) correlation.
    ///
    /// For sectored structures, returns the average correlation.
    pub fn asset_correlation(&self) -> f64 {
        match self {
            CorrelationStructure::Flat {
                asset_correlation, ..
            } => *asset_correlation,
            CorrelationStructure::Sectored {
                intra_sector,
                inter_sector,
                ..
            } => {
                // Average correlation (simplified)
                0.5 * intra_sector + 0.5 * inter_sector
            }
            CorrelationStructure::Matrix { correlations, .. } => {
                // Average off-diagonal correlation
                let n = (correlations.len() as f64).sqrt() as usize;
                if n < 2 {
                    return 0.0;
                }
                let mut sum = 0.0;
                let mut count = 0;
                for i in 0..n {
                    for j in 0..n {
                        if i != j {
                            sum += correlations[i * n + j];
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    sum / count as f64
                } else {
                    0.0
                }
            }
        }
    }

    /// Get the prepay-default correlation.
    pub fn prepay_default_correlation(&self) -> f64 {
        match self {
            CorrelationStructure::Flat {
                prepay_default_correlation,
                ..
            } => *prepay_default_correlation,
            CorrelationStructure::Sectored { prepay_default, .. } => *prepay_default,
            CorrelationStructure::Matrix { .. } => -0.20, // Default assumption
        }
    }

    /// Get the intra-sector correlation (for sectored structures).
    pub fn intra_sector_correlation(&self) -> Option<f64> {
        match self {
            CorrelationStructure::Sectored { intra_sector, .. } => Some(*intra_sector),
            _ => None,
        }
    }

    /// Get the inter-sector correlation (for sectored structures).
    pub fn inter_sector_correlation(&self) -> Option<f64> {
        match self {
            CorrelationStructure::Sectored { inter_sector, .. } => Some(*inter_sector),
            _ => None,
        }
    }

    /// Check if this is a flat correlation structure.
    pub fn is_flat(&self) -> bool {
        matches!(self, CorrelationStructure::Flat { .. })
    }

    /// Check if this is a sectored correlation structure.
    pub fn is_sectored(&self) -> bool {
        matches!(self, CorrelationStructure::Sectored { .. })
    }

    /// Get correlation between two assets.
    ///
    /// # Arguments
    /// * `i` - First asset index
    /// * `j` - Second asset index
    /// * `same_sector` - Whether assets are in the same sector
    pub fn pairwise_correlation(&self, i: usize, j: usize, same_sector: bool) -> f64 {
        if i == j {
            return 1.0;
        }

        match self {
            CorrelationStructure::Flat {
                asset_correlation, ..
            } => *asset_correlation,
            CorrelationStructure::Sectored {
                intra_sector,
                inter_sector,
                ..
            } => {
                if same_sector {
                    *intra_sector
                } else {
                    *inter_sector
                }
            }
            CorrelationStructure::Matrix { correlations, .. } => {
                let n = (correlations.len() as f64).sqrt() as usize;
                if i < n && j < n {
                    correlations[i * n + j]
                } else {
                    0.0
                }
            }
        }
    }

    /// Get the prepayment factor loading.
    ///
    /// Derived from prepay-default correlation and asset correlation.
    /// Factor loading represents how much prepayment responds to systematic factor.
    pub fn prepay_factor_loading(&self) -> f64 {
        let asset_corr = self.asset_correlation();
        let prepay_default_corr = self.prepay_default_correlation();

        // Factor loading from correlation:
        // ρ = β_prepay * β_default (approximately)
        // Assuming default factor loading ≈ sqrt(asset_correlation)
        // prepay_factor_loading = prepay_default_correlation / sqrt(asset_correlation)
        let default_loading = asset_corr.sqrt();
        if default_loading > 0.001 {
            // Prepay loading with opposite sign (prepays typically anti-correlate with defaults)
            (prepay_default_corr / default_loading).clamp(-1.0, 1.0)
        } else {
            // Low default correlation → default to weak prepay factor loading
            -0.3
        }
    }

    /// Get the default factor loading.
    ///
    /// Derived from asset correlation using single-factor model assumption.
    /// Factor loading β such that ρ ≈ β².
    pub fn default_factor_loading(&self) -> f64 {
        // For Gaussian copula: correlation ≈ factor_loading²
        let asset_corr = self.asset_correlation();
        asset_corr.sqrt()
    }

    /// Bump asset correlation by the given amount.
    ///
    /// For flat structures, bumps the single asset correlation.
    /// For sectored structures, bumps intra-sector by `delta` and inter-sector by `delta * 0.5`.
    /// For matrix structures, bumps all off-diagonal elements.
    ///
    /// # Arguments
    /// * `delta` - Amount to add to correlation (clamped to valid range)
    pub fn bump_asset(&self, delta: f64) -> Self {
        match self {
            CorrelationStructure::Flat {
                asset_correlation,
                prepay_default_correlation,
            } => {
                let new_asset = (asset_correlation + delta).clamp(0.0, 0.99);
                CorrelationStructure::flat(new_asset, *prepay_default_correlation)
            }
            CorrelationStructure::Sectored {
                intra_sector,
                inter_sector,
                prepay_default,
            } => {
                let new_intra = (intra_sector + delta).clamp(0.0, 0.99);
                let new_inter = (inter_sector + delta * 0.5).clamp(0.0, 0.99);
                CorrelationStructure::sectored(new_intra, new_inter, *prepay_default)
            }
            CorrelationStructure::Matrix { correlations, labels } => {
                let n = labels.len();
                let mut new_corrs = correlations.clone();
                for i in 0..n {
                    for j in 0..n {
                        if i != j {
                            let idx = i * n + j;
                            new_corrs[idx] = (new_corrs[idx] + delta).clamp(0.0, 0.99);
                        }
                    }
                }
                CorrelationStructure::matrix(new_corrs, labels.clone())
            }
        }
    }

    /// Bump prepay-default correlation by the given amount.
    ///
    /// # Arguments
    /// * `delta` - Amount to add to correlation (clamped to [-0.99, 0.99])
    pub fn bump_prepay_default(&self, delta: f64) -> Self {
        match self {
            CorrelationStructure::Flat {
                asset_correlation,
                prepay_default_correlation,
            } => {
                let new_pd = (prepay_default_correlation + delta).clamp(-0.99, 0.99);
                CorrelationStructure::flat(*asset_correlation, new_pd)
            }
            CorrelationStructure::Sectored {
                intra_sector,
                inter_sector,
                prepay_default,
            } => {
                let new_pd = (prepay_default + delta).clamp(-0.99, 0.99);
                CorrelationStructure::sectored(*intra_sector, *inter_sector, new_pd)
            }
            CorrelationStructure::Matrix { .. } => {
                // Matrix structure doesn't have explicit prepay-default correlation
                self.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_correlation() {
        let corr = CorrelationStructure::flat(0.10, -0.30);

        assert!((corr.asset_correlation() - 0.10).abs() < 1e-10);
        assert!((corr.prepay_default_correlation() - (-0.30)).abs() < 1e-10);
        assert!(corr.is_flat());
    }

    #[test]
    fn test_sectored_correlation() {
        let corr = CorrelationStructure::sectored(0.30, 0.10, -0.20);

        assert!(corr.is_sectored());
        assert_eq!(corr.intra_sector_correlation(), Some(0.30));
        assert_eq!(corr.inter_sector_correlation(), Some(0.10));
    }

    #[test]
    fn test_pairwise_correlation() {
        let sectored = CorrelationStructure::sectored(0.30, 0.10, -0.20);

        // Same sector
        assert!((sectored.pairwise_correlation(0, 1, true) - 0.30).abs() < 1e-10);
        // Different sector
        assert!((sectored.pairwise_correlation(0, 1, false) - 0.10).abs() < 1e-10);
        // Same asset
        assert!((sectored.pairwise_correlation(0, 0, true) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_standard_calibrations() {
        let rmbs = CorrelationStructure::rmbs_standard();
        assert!((rmbs.asset_correlation() - 0.05).abs() < 1e-10);
        assert!(rmbs.prepay_default_correlation() < 0.0);

        let clo = CorrelationStructure::clo_standard();
        assert!(clo.is_sectored());
        let intra = clo
            .intra_sector_correlation()
            .expect("CLO should have intra-sector correlation");
        let inter = clo
            .inter_sector_correlation()
            .expect("CLO should have inter-sector correlation");
        assert!(intra > inter);

        let cmbs = CorrelationStructure::cmbs_standard();
        assert!(cmbs.asset_correlation() > rmbs.asset_correlation());
    }

    #[test]
    fn test_clamping() {
        let corr = CorrelationStructure::flat(1.5, -1.5);

        assert!(corr.asset_correlation() <= 0.99);
        assert!(corr.prepay_default_correlation() >= -0.99);
    }

    #[test]
    fn test_default() {
        let corr = CorrelationStructure::default();
        assert!(corr.is_flat());
        assert!(corr.asset_correlation() > 0.0);
    }

    #[test]
    fn test_bump_asset_flat() {
        let corr = CorrelationStructure::flat(0.20, -0.30);
        let bumped = corr.bump_asset(0.05);

        assert!((bumped.asset_correlation() - 0.25).abs() < 1e-10);
        assert!((bumped.prepay_default_correlation() - (-0.30)).abs() < 1e-10);
    }

    #[test]
    fn test_bump_asset_sectored() {
        let corr = CorrelationStructure::sectored(0.30, 0.10, -0.20);
        let bumped = corr.bump_asset(0.10);

        // Intra bumps by full delta
        assert!((bumped.intra_sector_correlation().expect("should be sectored") - 0.40).abs() < 1e-10);
        // Inter bumps by half delta
        assert!((bumped.inter_sector_correlation().expect("should be sectored") - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_bump_asset_clamping() {
        let corr = CorrelationStructure::flat(0.95, -0.30);
        let bumped = corr.bump_asset(0.10);

        // Should clamp to 0.99
        assert!(bumped.asset_correlation() <= 0.99);
    }

    #[test]
    fn test_bump_prepay_default() {
        let corr = CorrelationStructure::flat(0.20, -0.30);
        let bumped = corr.bump_prepay_default(0.10);

        assert!((bumped.asset_correlation() - 0.20).abs() < 1e-10);
        assert!((bumped.prepay_default_correlation() - (-0.20)).abs() < 1e-10);
    }

    #[test]
    fn test_bump_prepay_default_clamping() {
        let corr = CorrelationStructure::flat(0.20, -0.95);
        let bumped = corr.bump_prepay_default(-0.10);

        // Should clamp to -0.99
        assert!(bumped.prepay_default_correlation() >= -0.99);
    }
}
