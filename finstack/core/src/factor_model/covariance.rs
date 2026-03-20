use super::FactorId;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// User-supplied factor covariance matrix with row-major storage.
///
/// Entries are expected to be on a consistent variance scale, typically annual
/// variance/covariance for the factor returns used by the risk engine. The
/// factor ID order is part of the contract: row `i`, column `j` corresponds to
/// `factor_ids[i]` and `factor_ids[j]`.
#[derive(Debug, Clone, Serialize)]
pub struct FactorCovarianceMatrix {
    factor_ids: Vec<FactorId>,
    n: usize,
    data: Vec<f64>,
    #[serde(skip)]
    index: HashMap<FactorId, usize>,
}

impl FactorCovarianceMatrix {
    /// Construct a covariance matrix with full validation.
    ///
    /// Validation checks:
    /// - `data.len() == n * n`
    /// - factor identifiers are unique
    /// - the matrix is symmetric within a small floating-point tolerance
    /// - the matrix is positive semi-definite according to a Cholesky-style test
    pub fn new(factor_ids: Vec<FactorId>, data: Vec<f64>) -> crate::Result<Self> {
        let n = factor_ids.len();
        if data.len() != n * n {
            return Err(crate::InputError::DimensionMismatch.into());
        }

        let index = Self::build_index(&factor_ids)?;

        for i in 0..n {
            for j in (i + 1)..n {
                let lhs = data[i * n + j];
                let rhs = data[j * n + i];
                if (lhs - rhs).abs() > 1e-12 {
                    return Err(crate::Error::Validation(format!(
                        "Covariance matrix is not symmetric at ({i}, {j})"
                    )));
                }
            }
        }

        if !Self::is_psd(&data, n) {
            return Err(crate::Error::Validation(
                "Covariance matrix is not positive semi-definite".to_string(),
            ));
        }

        Ok(Self {
            factor_ids,
            n,
            data,
            index,
        })
    }

    /// Construct a covariance matrix without validation.
    ///
    /// Use this only when the caller has already validated symmetry, PSD, and
    /// factor ordering externally.
    #[must_use]
    pub fn new_unchecked(factor_ids: Vec<FactorId>, data: Vec<f64>) -> Self {
        let n = factor_ids.len();
        let index = factor_ids
            .iter()
            .enumerate()
            .map(|(idx, factor_id)| (factor_id.clone(), idx))
            .collect();

        Self {
            factor_ids,
            n,
            data,
            index,
        }
    }

    /// Number of factors represented by the matrix.
    #[must_use]
    pub fn n_factors(&self) -> usize {
        self.n
    }

    /// Ordered factor identifiers corresponding to the matrix axes.
    #[must_use]
    pub fn factor_ids(&self) -> &[FactorId] {
        &self.factor_ids
    }

    /// Borrow the raw row-major covariance storage.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    /// Return the variance for a factor.
    #[must_use]
    pub fn variance(&self, factor: &FactorId) -> f64 {
        let idx = self.index[factor];
        self.data[idx * self.n + idx]
    }

    /// Return the covariance between two factors.
    #[must_use]
    pub fn covariance(&self, lhs: &FactorId, rhs: &FactorId) -> f64 {
        let i = self.index[lhs];
        let j = self.index[rhs];
        self.data[i * self.n + j]
    }

    /// Return the correlation between two factors.
    #[must_use]
    pub fn correlation(&self, lhs: &FactorId, rhs: &FactorId) -> f64 {
        let covariance = self.covariance(lhs, rhs);
        let variance_lhs = self.variance(lhs);
        let variance_rhs = self.variance(rhs);

        if variance_lhs <= 0.0 || variance_rhs <= 0.0 {
            return 0.0;
        }

        covariance / (variance_lhs.sqrt() * variance_rhs.sqrt())
    }

    fn build_index(factor_ids: &[FactorId]) -> crate::Result<HashMap<FactorId, usize>> {
        let mut index = HashMap::with_capacity(factor_ids.len());
        for (idx, factor_id) in factor_ids.iter().cloned().enumerate() {
            if index.insert(factor_id.clone(), idx).is_some() {
                return Err(crate::Error::Validation(format!(
                    "Duplicate factor id '{factor_id}' in covariance matrix"
                )));
            }
        }
        Ok(index)
    }

    fn is_psd(data: &[f64], n: usize) -> bool {
        let eps = 1e-12;
        let mut lower = vec![0.0_f64; n * n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += lower[i * n + k] * lower[j * n + k];
                }

                if i == j {
                    let diagonal = data[i * n + i] - sum;
                    if diagonal < -eps {
                        return false;
                    }
                    lower[i * n + j] = diagonal.max(0.0).sqrt();
                } else {
                    let denominator = lower[j * n + j];
                    let value = data[i * n + j] - sum;
                    if denominator.abs() <= eps {
                        if value.abs() > eps {
                            return false;
                        }
                        lower[i * n + j] = 0.0;
                    } else {
                        lower[i * n + j] = value / denominator;
                    }
                }
            }
        }

        true
    }
}

impl<'de> Deserialize<'de> for FactorCovarianceMatrix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FactorCovarianceMatrixSerde {
            factor_ids: Vec<FactorId>,
            n: usize,
            data: Vec<f64>,
        }

        let helper = FactorCovarianceMatrixSerde::deserialize(deserializer)?;
        let index = FactorCovarianceMatrix::build_index(&helper.factor_ids)
            .map_err(serde::de::Error::custom)?;

        if helper.factor_ids.len() != helper.n {
            return Err(serde::de::Error::custom(
                "Factor covariance matrix factor count does not match n",
            ));
        }

        if helper.data.len() != helper.n * helper.n {
            return Err(serde::de::Error::custom(
                "Factor covariance matrix data length does not match n x n",
            ));
        }

        Ok(FactorCovarianceMatrix {
            factor_ids: helper.factor_ids,
            n: helper.n,
            data: helper.data,
            index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_factor_ids() -> Vec<FactorId> {
        vec![FactorId::new("Rates"), FactorId::new("Credit")]
    }

    #[test]
    fn test_valid_2x2_covariance() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        assert_eq!(covariance.n_factors(), 2);
    }

    #[test]
    fn test_variance_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        assert!((covariance.variance(&FactorId::new("Rates")) - 0.04).abs() < 1e-12);
        assert!((covariance.variance(&FactorId::new("Credit")) - 0.09).abs() < 1e-12);
    }

    #[test]
    fn test_covariance_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        let value = covariance.covariance(&FactorId::new("Rates"), &FactorId::new("Credit"));
        assert!((value - 0.01).abs() < 1e-12);
    }

    #[test]
    fn test_correlation_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        let correlation = covariance.correlation(&FactorId::new("Rates"), &FactorId::new("Credit"));
        assert!((correlation - (1.0 / 6.0)).abs() < 1e-10);
    }

    #[test]
    fn test_wrong_dimensions_rejected() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01];
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_asymmetric_matrix_rejected() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.02, 0.01, 0.09];
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_not_psd_rejected() {
        let ids = two_factor_ids();
        let data = vec![1.0, 3.0, 3.0, 1.0];
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_new_unchecked_skips_validation() {
        let ids = two_factor_ids();
        let data = vec![1.0, 3.0, 3.0, 1.0];
        let covariance = FactorCovarianceMatrix::new_unchecked(ids, data);
        assert_eq!(covariance.n_factors(), 2);
    }

    #[test]
    fn test_single_factor() {
        let ids = vec![FactorId::new("Only")];
        let data = vec![0.25];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        assert!((covariance.variance(&FactorId::new("Only")) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn test_as_slice() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data.clone());
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };
        assert_eq!(covariance.as_slice(), &data[..]);
    }

    #[test]
    fn test_serde_roundtrip() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let covariance_result = FactorCovarianceMatrix::new(ids, data);
        assert!(covariance_result.is_ok());
        let Ok(covariance) = covariance_result else {
            return;
        };

        let json_result = serde_json::to_string(&covariance);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<FactorCovarianceMatrix, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };
        assert_eq!(covariance.as_slice(), back.as_slice());
        assert_eq!(covariance.n_factors(), back.n_factors());
        assert_eq!(
            covariance.covariance(&FactorId::new("Rates"), &FactorId::new("Credit")),
            back.covariance(&FactorId::new("Rates"), &FactorId::new("Credit"))
        );
    }
}
