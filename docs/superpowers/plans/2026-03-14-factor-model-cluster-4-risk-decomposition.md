# Cluster 4: Risk Decomposition — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `RiskMeasure`, `RiskDecomposer` trait, `RiskDecomposition` output types, and the two built-in decomposers (`ParametricDecomposer` and `SimulationDecomposer`) in `finstack/portfolio`.

**Architecture:** All decomposition logic lives in `finstack/portfolio/src/factor_model/`. The `RiskMeasure` enum goes in core (it's a config type). Output types (`RiskDecomposition`, `FactorContribution`, etc.) live in portfolio alongside the decomposers. The `ParametricDecomposer` is pure linear algebra on matrices; the `SimulationDecomposer` generates Monte Carlo scenarios via Cholesky factorization.

**Tech Stack:** Rust, standard library for linear algebra (no external dependency needed for small matrices)

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Section 3

**Depends on:** Cluster 1 (FactorId, FactorCovarianceMatrix), Cluster 3 (SensitivityMatrix)

---

## Task 1: Add `RiskMeasure` to core

**Files:**

- Modify: `finstack/core/src/factor_model/config.rs`
- Modify: `finstack/core/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn test_risk_measure_serde() {
    let m = RiskMeasure::VaR { confidence: 0.99 };
    let json = serde_json::to_string(&m).unwrap();
    let back: RiskMeasure = serde_json::from_str(&json).unwrap();
    assert_eq!(m, back);
}

#[test]
fn test_risk_measure_default() {
    assert_eq!(RiskMeasure::default(), RiskMeasure::Variance);
}
```

- [ ] **Step 2: Implement RiskMeasure**

Add to `finstack/core/src/factor_model/config.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiskMeasure {
    Variance,
    Volatility,
    VaR { confidence: f64 },
    ExpectedShortfall { confidence: f64 },
}

impl Default for RiskMeasure {
    fn default() -> Self {
        Self::Variance
    }
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(factor-model): add RiskMeasure enum"
```

---

## Task 2: Create decomposition output types

**Files:**

- Create: `finstack/portfolio/src/factor_model/mod.rs`
- Create: `finstack/portfolio/src/factor_model/types.rs`
- Modify: `finstack/portfolio/src/lib.rs` — add `pub mod factor_model;`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{FactorId, RiskMeasure};

    #[test]
    fn test_risk_decomposition_total_matches_sum() {
        let decomp = RiskDecomposition {
            total_risk: 100.0,
            measure: RiskMeasure::Variance,
            factor_contributions: vec![
                FactorContribution {
                    factor_id: FactorId::new("Rates"),
                    absolute_risk: 60.0,
                    relative_risk: 0.6,
                    marginal_risk: 0.3,
                },
                FactorContribution {
                    factor_id: FactorId::new("Credit"),
                    absolute_risk: 40.0,
                    relative_risk: 0.4,
                    marginal_risk: 0.2,
                },
            ],
            residual_risk: 0.0,
            position_factor_contributions: vec![],
        };

        let sum: f64 = decomp.factor_contributions.iter().map(|c| c.absolute_risk).sum();
        assert!((sum + decomp.residual_risk - decomp.total_risk).abs() < 1e-10);
    }

    #[test]
    fn test_relative_risk_sums_to_one() {
        let decomp = RiskDecomposition {
            total_risk: 100.0,
            measure: RiskMeasure::Variance,
            factor_contributions: vec![
                FactorContribution {
                    factor_id: FactorId::new("Rates"),
                    absolute_risk: 60.0,
                    relative_risk: 0.6,
                    marginal_risk: 0.3,
                },
                FactorContribution {
                    factor_id: FactorId::new("Credit"),
                    absolute_risk: 40.0,
                    relative_risk: 0.4,
                    marginal_risk: 0.2,
                },
            ],
            residual_risk: 0.0,
            position_factor_contributions: vec![],
        };

        let sum_rel: f64 = decomp.factor_contributions.iter().map(|c| c.relative_risk).sum();
        assert!((sum_rel - 1.0).abs() < 1e-10);
    }
}
```

- [ ] **Step 2: Implement output types**

```rust
use finstack_core::factor_model::{FactorId, RiskMeasure};

#[derive(Debug, Clone)]
pub struct RiskDecomposition {
    pub total_risk: f64,
    pub measure: RiskMeasure,
    pub factor_contributions: Vec<FactorContribution>,
    pub residual_risk: f64,
    pub position_factor_contributions: Vec<PositionFactorContribution>,
}

#[derive(Debug, Clone)]
pub struct FactorContribution {
    pub factor_id: FactorId,
    pub absolute_risk: f64,
    pub relative_risk: f64,
    pub marginal_risk: f64,
}

#[derive(Debug, Clone)]
pub struct PositionFactorContribution {
    pub position_id: String,
    pub factor_id: FactorId,
    pub risk_contribution: f64,
}
```

- [ ] **Step 3: Register modules**

- [ ] **Step 4: Run tests, commit**

```bash
git commit -m "feat(factor-model): add RiskDecomposition output types"
```

---

## Task 3: Create `RiskDecomposer` trait

**Files:**

- Create: `finstack/portfolio/src/factor_model/traits.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

- [ ] **Step 1: Define trait**

```rust
use super::types::RiskDecomposition;
use finstack_core::factor_model::{FactorCovarianceMatrix, RiskMeasure};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use finstack_core::Result;

pub trait RiskDecomposer: Send + Sync {
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        position_weights: &[f64],
        measure: &RiskMeasure,
    ) -> Result<RiskDecomposition>;
}
```

- [ ] **Step 2: Build, commit**

```bash
git commit -m "feat(factor-model): add RiskDecomposer trait"
```

---

## Task 4: Implement `ParametricDecomposer`

**Files:**

- Create: `finstack/portfolio/src/factor_model/parametric.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

This is the core mathematical component. It implements Euler decomposition for variance, and scales to VaR/ES under normality.

- [ ] **Step 1: Write failing tests with hand-computed expected values**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{FactorCovarianceMatrix, FactorId, RiskMeasure};
    use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

    /// 2 factors, 2 positions, known covariance.
    /// Factor 1 (Rates): σ² = 0.04 (σ = 0.2)
    /// Factor 2 (Credit): σ² = 0.09 (σ = 0.3)
    /// Correlation: ρ = 0.5, covariance = 0.5 * 0.2 * 0.3 = 0.03
    fn test_setup() -> (SensitivityMatrix, FactorCovarianceMatrix, Vec<f64>) {
        let mut sens = SensitivityMatrix::zeros(
            vec!["pos-A".into(), "pos-B".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        // pos-A: delta to Rates=100, delta to Credit=0
        sens.set_delta(0, 0, 100.0);
        sens.set_delta(0, 1, 0.0);
        // pos-B: delta to Rates=0, delta to Credit=50
        sens.set_delta(1, 0, 0.0);
        sens.set_delta(1, 1, 50.0);

        let cov = FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
            vec![0.04, 0.03, 0.03, 0.09],
        ).unwrap();

        // Equal weights
        let weights = vec![1.0, 1.0];

        (sens, cov, weights)
    }

    #[test]
    fn test_parametric_variance_decomposition() {
        let (sens, cov, weights) = test_setup();
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sens, &cov, &weights, &RiskMeasure::Variance).unwrap();

        // Portfolio factor exposures: e = W' × S
        // e_rates = 1.0 * 100 + 1.0 * 0 = 100
        // e_credit = 1.0 * 0 + 1.0 * 50 = 50
        //
        // Portfolio variance: σ² = e' × Σ × e
        // = 100² * 0.04 + 2 * 100 * 50 * 0.03 + 50² * 0.09
        // = 400 + 300 + 225 = 925
        assert!((result.total_risk - 925.0).abs() < 1e-6);

        // Component variance (Euler):
        // (Σ × e)_rates = 0.04 * 100 + 0.03 * 50 = 4.0 + 1.5 = 5.5
        // (Σ × e)_credit = 0.03 * 100 + 0.09 * 50 = 3.0 + 4.5 = 7.5
        // CR_rates = e_rates * (Σ × e)_rates = 100 * 5.5 = 550
        // CR_credit = e_credit * (Σ × e)_credit = 50 * 7.5 = 375
        // Total: 550 + 375 = 925 ✓

        let rates_contrib = result.factor_contributions.iter()
            .find(|c| c.factor_id == FactorId::new("Rates")).unwrap();
        let credit_contrib = result.factor_contributions.iter()
            .find(|c| c.factor_id == FactorId::new("Credit")).unwrap();

        assert!((rates_contrib.absolute_risk - 550.0).abs() < 1e-6);
        assert!((credit_contrib.absolute_risk - 375.0).abs() < 1e-6);

        // Relative: 550/925 ≈ 0.5946, 375/925 ≈ 0.4054
        assert!((rates_contrib.relative_risk - 550.0 / 925.0).abs() < 1e-6);
        assert!((credit_contrib.relative_risk - 375.0 / 925.0).abs() < 1e-6);

        // Sum of relative must be 1.0
        let sum_rel: f64 = result.factor_contributions.iter().map(|c| c.relative_risk).sum();
        assert!((sum_rel - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parametric_volatility_decomposition() {
        let (sens, cov, weights) = test_setup();
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sens, &cov, &weights, &RiskMeasure::Volatility).unwrap();

        // σ = sqrt(925) ≈ 30.414
        assert!((result.total_risk - 925.0_f64.sqrt()).abs() < 1e-3);

        // Component vol: CR_i / σ (Euler for volatility = component_variance / σ)
        let rates_contrib = result.factor_contributions.iter()
            .find(|c| c.factor_id == FactorId::new("Rates")).unwrap();

        // 550 / sqrt(925) ≈ 18.08
        assert!((rates_contrib.absolute_risk - 550.0 / 925.0_f64.sqrt()).abs() < 1e-2);
    }

    #[test]
    fn test_parametric_var_decomposition() {
        let (sens, cov, weights) = test_setup();
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sens, &cov, &weights, &RiskMeasure::VaR { confidence: 0.99 }).unwrap();

        // VaR = σ × z_0.99 where z_0.99 ≈ 2.326
        let sigma = 925.0_f64.sqrt();
        let z = 2.3263; // Φ⁻¹(0.99)
        let expected_var = sigma * z;
        assert!((result.total_risk - expected_var).abs() < 0.1);
    }

    #[test]
    fn test_zero_risk_portfolio() {
        let sens = SensitivityMatrix::zeros(
            vec!["cash".into()],
            vec![FactorId::new("Rates")],
        );
        let cov = FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates")],
            vec![0.04],
        ).unwrap();
        let weights = vec![1.0];

        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sens, &cov, &weights, &RiskMeasure::Variance).unwrap();

        assert!((result.total_risk).abs() < 1e-12);
        // All relative risks should be 0 (not NaN)
        for c in &result.factor_contributions {
            assert!((c.relative_risk).abs() < 1e-12);
            assert!(!c.relative_risk.is_nan());
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement ParametricDecomposer**

```rust
use super::traits::RiskDecomposer;
use super::types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
use finstack_core::factor_model::{FactorCovarianceMatrix, FactorId, RiskMeasure};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use finstack_core::Result;

pub struct ParametricDecomposer;

impl ParametricDecomposer {
    /// Compute portfolio factor exposures: e_j = Σ_i (w_i × S_ij)
    fn portfolio_exposures(
        sensitivities: &SensitivityMatrix,
        weights: &[f64],
    ) -> Vec<f64> {
        let n_factors = sensitivities.n_factors();
        let mut exposures = vec![0.0; n_factors];
        for (pi, &w) in weights.iter().enumerate() {
            let row = sensitivities.position_deltas(pi);
            for (fi, &delta) in row.iter().enumerate() {
                exposures[fi] += w * delta;
            }
        }
        exposures
    }

    /// Compute Σ × e (covariance matrix times exposure vector).
    fn cov_times_exposure(
        cov: &FactorCovarianceMatrix,
        exposures: &[f64],
    ) -> Vec<f64> {
        let n = cov.n_factors();
        let cov_data = cov.as_slice();
        let mut result = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                result[i] += cov_data[i * n + j] * exposures[j];
            }
        }
        result
    }

    /// Normal quantile (Φ⁻¹) via rational approximation (Abramowitz & Stegun 26.2.23).
    fn normal_quantile(p: f64) -> f64 {
        // For p > 0.5, use symmetry
        if p < 0.5 {
            return -Self::normal_quantile(1.0 - p);
        }
        let t = (-2.0 * (1.0 - p).ln()).sqrt();
        let c0 = 2.515517;
        let c1 = 0.802853;
        let c2 = 0.010328;
        let d1 = 1.432788;
        let d2 = 0.189269;
        let d3 = 0.001308;
        t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t)
    }

    /// Standard normal PDF.
    fn normal_pdf(x: f64) -> f64 {
        (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }
}

impl RiskDecomposer for ParametricDecomposer {
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        position_weights: &[f64],
        measure: &RiskMeasure,
    ) -> Result<RiskDecomposition> {
        let exposures = Self::portfolio_exposures(sensitivities, position_weights);
        let cov_e = Self::cov_times_exposure(covariance, &exposures);

        // Portfolio variance = e' × (Σ × e)
        let variance: f64 = exposures.iter().zip(cov_e.iter()).map(|(e, ce)| e * ce).sum();
        let sigma = variance.max(0.0).sqrt();

        // Component variance (Euler): CV_i = e_i × (Σ × e)_i
        let component_variances: Vec<f64> = exposures
            .iter()
            .zip(cov_e.iter())
            .map(|(e, ce)| e * ce)
            .collect();

        // Scale factor and total risk based on measure
        let (total_risk, scale) = match measure {
            RiskMeasure::Variance => (variance, 1.0),
            RiskMeasure::Volatility => {
                if sigma < 1e-15 {
                    (0.0, 0.0)
                } else {
                    (sigma, 1.0 / sigma)
                }
            }
            RiskMeasure::VaR { confidence } => {
                let z = Self::normal_quantile(*confidence);
                (sigma * z, z / sigma.max(1e-15))
            }
            RiskMeasure::ExpectedShortfall { confidence } => {
                let z = Self::normal_quantile(*confidence);
                let es_scale = Self::normal_pdf(z) / (1.0 - confidence);
                (sigma * es_scale, es_scale / sigma.max(1e-15))
            }
        };

        // Component risk = component_variance × scale
        let factor_ids = covariance.factor_ids();
        let factor_contributions: Vec<FactorContribution> = factor_ids
            .iter()
            .enumerate()
            .map(|(fi, fid)| {
                let abs_risk = component_variances[fi] * scale;
                let rel_risk = if total_risk.abs() > 1e-15 {
                    abs_risk / total_risk
                } else {
                    0.0
                };
                let marginal = cov_e[fi] * scale;

                FactorContribution {
                    factor_id: fid.clone(),
                    absolute_risk: abs_risk,
                    relative_risk: rel_risk,
                    marginal_risk: marginal,
                }
            })
            .collect();

        // Per-position, per-factor contributions
        let mut position_factor_contributions = Vec::new();
        for (pi, pos_id) in sensitivities.position_ids.iter().enumerate() {
            let row = sensitivities.position_deltas(pi);
            for (fi, fid) in factor_ids.iter().enumerate() {
                let contrib = position_weights[pi] * row[fi] * cov_e[fi] * scale;
                position_factor_contributions.push(PositionFactorContribution {
                    position_id: pos_id.clone(),
                    factor_id: fid.clone(),
                    risk_contribution: contrib,
                });
            }
        }

        Ok(RiskDecomposition {
            total_risk,
            measure: *measure,
            factor_contributions,
            residual_risk: 0.0, // Parametric has no residual by construction
            position_factor_contributions,
        })
    }
}
```

- [ ] **Step 4: Register in mod.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-portfolio factor_model::parametric --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/portfolio/src/factor_model/
git commit -m "feat(factor-model): add ParametricDecomposer with Euler variance decomposition"
```

---

## Task 5: Implement `SimulationDecomposer`

**Files:**

- Create: `finstack/portfolio/src/factor_model/simulation.rs`
- Modify: `finstack/portfolio/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{FactorCovarianceMatrix, FactorId, RiskMeasure};
    use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

    #[test]
    fn test_cholesky_2x2() {
        let data = vec![4.0, 2.0, 2.0, 5.0];
        let l = cholesky(&data, 2).unwrap();
        // L should be: [2, 0, 1, 2]
        assert!((l[0] - 2.0).abs() < 1e-10);
        assert!((l[1]).abs() < 1e-10);
        assert!((l[2] - 1.0).abs() < 1e-10);
        assert!((l[3] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simulation_es_converges_to_parametric() {
        // With enough samples and a linear model, simulation ES should
        // converge to the parametric ES.
        let mut sens = SensitivityMatrix::zeros(
            vec!["pos-A".into()],
            vec![FactorId::new("Rates")],
        );
        sens.set_delta(0, 0, 100.0);

        let cov = FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates")],
            vec![0.04],
        ).unwrap();

        let weights = vec![1.0];
        let decomposer = SimulationDecomposer::new(100_000, 42); // seed for reproducibility

        let result = decomposer.decompose(
            &sens, &cov, &weights,
            &RiskMeasure::ExpectedShortfall { confidence: 0.99 },
        ).unwrap();

        // Parametric ES: σ × φ(z) / (1 - α)
        // σ_portfolio = 100 * 0.2 = 20
        // z_0.99 ≈ 2.326
        // φ(2.326) ≈ 0.02665
        // ES = 20 * 0.02665 / 0.01 ≈ 53.3
        assert!((result.total_risk - 53.3).abs() < 2.0); // within 2.0 tolerance for MC
    }
}
```

- [ ] **Step 2: Implement Cholesky and SimulationDecomposer**

```rust
use super::traits::RiskDecomposer;
use super::types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
use finstack_core::factor_model::{FactorCovarianceMatrix, RiskMeasure};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use finstack_core::Result;

/// Cholesky decomposition: returns lower-triangular L such that L × L' = A.
pub(crate) fn cholesky(data: &[f64], n: usize) -> Result<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i * n + k] * l[j * n + k];
            }
            if i == j {
                let diag = data[i * n + i] - sum;
                if diag < -1e-12 {
                    return Err(finstack_core::FinstackError::invalid_input(
                        "Matrix not PSD for Cholesky".into(),
                    ));
                }
                l[i * n + j] = diag.max(0.0).sqrt();
            } else {
                let denom = l[j * n + j];
                l[i * n + j] = if denom.abs() < 1e-15 {
                    0.0
                } else {
                    (data[i * n + j] - sum) / denom
                };
            }
        }
    }
    Ok(l)
}

pub struct SimulationDecomposer {
    n_scenarios: usize,
    seed: u64,
}

impl SimulationDecomposer {
    pub fn new(n_scenarios: usize, seed: u64) -> Self {
        Self { n_scenarios, seed }
    }

    /// Simple xoshiro256** PRNG for reproducibility without external deps.
    fn generate_standard_normals(&self, count: usize) -> Vec<f64> {
        // Box-Muller transform with a simple PRNG
        // For production, consider using rand crate; for now this is self-contained
        let mut state = [self.seed, self.seed.wrapping_mul(6364136223846793005), 0, 0];
        state[2] = state[0] ^ state[1];
        state[3] = state[1] ^ state[0];

        let mut normals = Vec::with_capacity(count);
        let mut i = 0;
        while normals.len() < count {
            // Simple LCG for uniform [0,1)
            state[0] = state[0].wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u1 = (state[0] >> 11) as f64 / (1u64 << 53) as f64;
            state[0] = state[0].wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u2 = (state[0] >> 11) as f64 / (1u64 << 53) as f64;

            if u1 > 1e-15 {
                let r = (-2.0 * u1.ln()).sqrt();
                let theta = 2.0 * std::f64::consts::PI * u2;
                normals.push(r * theta.cos());
                if normals.len() < count {
                    normals.push(r * theta.sin());
                }
            }
            i += 1;
        }
        normals
    }
}

impl RiskDecomposer for SimulationDecomposer {
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        position_weights: &[f64],
        measure: &RiskMeasure,
    ) -> Result<RiskDecomposition> {
        let n_factors = covariance.n_factors();
        let l = cholesky(covariance.as_slice(), n_factors)?;

        // Generate correlated factor scenarios
        let z = self.generate_standard_normals(self.n_scenarios * n_factors);

        // Compute portfolio P&L for each scenario
        // P&L = Σ_i w_i × Σ_j S_ij × factor_j
        let exposures: Vec<f64> = {
            let mut e = vec![0.0; n_factors];
            for (pi, &w) in position_weights.iter().enumerate() {
                let row = sensitivities.position_deltas(pi);
                for (fi, &delta) in row.iter().enumerate() {
                    e[fi] += w * delta;
                }
            }
            e
        };

        let mut portfolio_pnls = Vec::with_capacity(self.n_scenarios);
        let mut factor_pnls = vec![vec![0.0; self.n_scenarios]; n_factors]; // [factor][scenario]

        for s in 0..self.n_scenarios {
            // Correlated factors: f = L × z
            let mut factors = vec![0.0; n_factors];
            for i in 0..n_factors {
                for j in 0..=i {
                    factors[i] += l[i * n_factors + j] * z[s * n_factors + j];
                }
            }

            // Portfolio P&L
            let pnl: f64 = exposures.iter().zip(factors.iter()).map(|(e, f)| e * f).sum();
            portfolio_pnls.push(pnl);

            // Factor-level P&L (for component allocation)
            for fi in 0..n_factors {
                factor_pnls[fi][s] = exposures[fi] * factors[fi];
            }
        }

        // Sort scenarios by total P&L (losses are negative)
        let mut indices: Vec<usize> = (0..self.n_scenarios).collect();
        indices.sort_by(|&a, &b| portfolio_pnls[a].partial_cmp(&portfolio_pnls[b]).unwrap());

        let (total_risk, factor_contributions) = match measure {
            RiskMeasure::VaR { confidence } | RiskMeasure::ExpectedShortfall { confidence } => {
                let tail_start = ((1.0 - confidence) * self.n_scenarios as f64) as usize;
                let tail_start = tail_start.max(1);

                // VaR = -P&L at the quantile
                let var_idx = indices[tail_start - 1];
                let var = -portfolio_pnls[var_idx];

                // ES = average of losses beyond VaR
                let tail_pnls: Vec<f64> = indices[..tail_start]
                    .iter()
                    .map(|&i| -portfolio_pnls[i])
                    .collect();
                let es: f64 = tail_pnls.iter().sum::<f64>() / tail_pnls.len() as f64;

                let total = match measure {
                    RiskMeasure::VaR { .. } => var,
                    RiskMeasure::ExpectedShortfall { .. } => es,
                    _ => unreachable!(),
                };

                // Component ES via Euler: E[L_i | L > VaR]
                let component_es: Vec<f64> = (0..n_factors)
                    .map(|fi| {
                        let sum: f64 = indices[..tail_start]
                            .iter()
                            .map(|&i| -factor_pnls[fi][i])
                            .sum();
                        sum / tail_start as f64
                    })
                    .collect();

                // Component VaR via Hallerbach: scale by VaR/ES
                let components = match measure {
                    RiskMeasure::VaR { .. } => {
                        let ratio = if es.abs() > 1e-15 { var / es } else { 1.0 };
                        component_es.iter().map(|c| c * ratio).collect::<Vec<_>>()
                    }
                    RiskMeasure::ExpectedShortfall { .. } => component_es,
                    _ => unreachable!(),
                };

                let contribs = covariance.factor_ids().iter().enumerate().map(|(fi, fid)| {
                    FactorContribution {
                        factor_id: fid.clone(),
                        absolute_risk: components[fi],
                        relative_risk: if total.abs() > 1e-15 { components[fi] / total } else { 0.0 },
                        marginal_risk: components[fi], // approximate
                    }
                }).collect();

                (total, contribs)
            }
            RiskMeasure::Variance | RiskMeasure::Volatility => {
                // For variance/vol, just use the sample variance decomposition
                let mean_pnl: f64 = portfolio_pnls.iter().sum::<f64>() / self.n_scenarios as f64;
                let var: f64 = portfolio_pnls.iter()
                    .map(|p| (p - mean_pnl).powi(2))
                    .sum::<f64>() / (self.n_scenarios - 1) as f64;

                let total = match measure {
                    RiskMeasure::Variance => var,
                    RiskMeasure::Volatility => var.sqrt(),
                    _ => unreachable!(),
                };

                // Sample covariance between portfolio and each factor for Euler
                let factor_means: Vec<f64> = (0..n_factors)
                    .map(|fi| factor_pnls[fi].iter().sum::<f64>() / self.n_scenarios as f64)
                    .collect();

                let component_vars: Vec<f64> = (0..n_factors)
                    .map(|fi| {
                        let cov: f64 = portfolio_pnls.iter().enumerate()
                            .map(|(s, &p)| (p - mean_pnl) * (factor_pnls[fi][s] - factor_means[fi]))
                            .sum::<f64>() / (self.n_scenarios - 1) as f64;
                        cov // This is Cov(L_portfolio, L_factor_i) which equals component variance
                    })
                    .collect();

                let scale = match measure {
                    RiskMeasure::Volatility => if var.sqrt() > 1e-15 { 1.0 / var.sqrt() } else { 0.0 },
                    _ => 1.0,
                };

                let contribs = covariance.factor_ids().iter().enumerate().map(|(fi, fid)| {
                    let abs = component_vars[fi] * scale;
                    FactorContribution {
                        factor_id: fid.clone(),
                        absolute_risk: abs,
                        relative_risk: if total.abs() > 1e-15 { abs / total } else { 0.0 },
                        marginal_risk: abs,
                    }
                }).collect();

                (total, contribs)
            }
        };

        Ok(RiskDecomposition {
            total_risk,
            measure: *measure,
            factor_contributions,
            residual_risk: 0.0,
            position_factor_contributions: vec![], // Per-position MC allocation is expensive; skip for now
        })
    }
}
```

- [ ] **Step 3: Register in mod.rs**

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-portfolio factor_model::simulation --no-default-features`
Expected: Tests PASS (MC convergence test within tolerance)

- [ ] **Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/portfolio/src/factor_model/
git commit -m "feat(factor-model): add SimulationDecomposer with Monte Carlo Euler allocation"
```
