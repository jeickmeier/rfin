//! Multi-dimensional Ornstein–Uhlenbeck (OU) process for generic time series.
//!
//! State dynamics for each component i:
//! dX_i = κ_i (θ_i - X_i) dt + σ_i dW_i
//! with optional correlation across the driving Brownian motions.

use super::super::paths::ProcessParams;
use super::super::traits::StochasticProcess;
use super::metadata::ProcessMetadata;

/// Parameters for multi-dimensional OU.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MultiOuParams {
    /// Mean reversion speeds κ_i (>0)
    pub kappas: Vec<f64>,
    /// Long-run means θ_i
    pub thetas: Vec<f64>,
    /// Volatilities σ_i (>=0)
    pub sigmas: Vec<f64>,
    /// Optional correlation matrix (n x n, row-major)
    pub correlation: Option<Vec<f64>>,
}

impl MultiOuParams {
    /// Create parameters; vectors must have equal length.
    pub fn new(
        kappas: Vec<f64>,
        thetas: Vec<f64>,
        sigmas: Vec<f64>,
        correlation: Option<Vec<f64>>,
    ) -> Self {
        let n = kappas.len();
        assert_eq!(thetas.len(), n, "thetas length must match kappas");
        assert_eq!(sigmas.len(), n, "sigmas length must match kappas");
        if let Some(ref corr) = correlation {
            assert_eq!(corr.len(), n * n, "correlation must be n x n");
        }
        Self {
            kappas,
            thetas,
            sigmas,
            correlation,
        }
    }
}

/// Multi-dimensional OU process.
#[derive(Clone, Debug)]
pub struct MultiOuProcess {
    params: MultiOuParams,
}

impl MultiOuProcess {
    pub fn new(params: MultiOuParams) -> Self {
        Self { params }
    }

    pub fn dim(&self) -> usize {
        self.params.kappas.len()
    }

    pub fn correlation(&self) -> Option<&[f64]> {
        self.params.correlation.as_deref()
    }
}

impl StochasticProcess for MultiOuProcess {
    fn dim(&self) -> usize {
        self.dim()
    }

    fn num_factors(&self) -> usize {
        self.dim()
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        for i in 0..self.dim() {
            out[i] = self.params.kappas[i] * (self.params.thetas[i] - x[i]);
        }
    }

    fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
        out.copy_from_slice(&self.params.sigmas);
    }

    fn is_diagonal(&self) -> bool {
        self.params.correlation.is_none()
    }
}

impl ProcessMetadata for MultiOuProcess {
    fn metadata(&self) -> ProcessParams {
        let mut p = ProcessParams::new("MultiOU");
        for i in 0..self.dim() {
            p.add_param(format!("kappa_{}", i), self.params.kappas[i]);
            p.add_param(format!("theta_{}", i), self.params.thetas[i]);
            p.add_param(format!("sigma_{}", i), self.params.sigmas[i]);
        }
        let p = if let Some(ref corr) = self.params.correlation {
            p.with_correlation(corr.clone())
        } else {
            p
        };
        let factor_names: Vec<String> = (0..self.dim()).map(|i| format!("x_{}", i)).collect();
        p.with_factors(factor_names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_ou_drift_diffusion() {
        let params = MultiOuParams::new(vec![2.0, 1.0], vec![1.0, -1.0], vec![0.3, 0.4], None);
        let proc = MultiOuProcess::new(params);
        let x = [0.0, 0.0];
        let mut mu = [0.0, 0.0];
        let mut sig = [0.0, 0.0];
        proc.drift(0.0, &x, &mut mu);
        proc.diffusion(0.0, &x, &mut sig);
        assert!((mu[0] - 2.0 * (1.0 - 0.0)).abs() < 1e-12);
        assert!((mu[1] - 1.0 * (-1.0 - 0.0)).abs() < 1e-12);
        assert!((sig[0] - 0.3).abs() < 1e-12);
        assert!((sig[1] - 0.4).abs() < 1e-12);
    }
}
