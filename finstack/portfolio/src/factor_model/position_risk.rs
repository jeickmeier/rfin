//! Position-level VaR and ES decomposition via Euler allocation.
//!
//! This module provides parametric and historical decomposition of portfolio
//! VaR and Expected Shortfall into per-position contributions. The parametric
//! engine uses covariance-based Euler allocation (exact under normality); the
//! historical engine attributes tail losses from scenario P&L matrices.
//!
//! # Euler Decomposition Property
//!
//! Under the parametric (normal) assumption:
//! ```text
//! sum(component_var_i) == portfolio_var  (exact)
//! sum(component_es_i)  == portfolio_es   (exact)
//! ```
//!
//! Under historical simulation the Euler property holds approximately.
//!
//! # References
//!
//! - `docs/REFERENCES.md#tasche-2008-capital-allocation`
//! - `docs/REFERENCES.md#meucci-risk-and-asset-allocation`
//! - `docs/REFERENCES.md#litterman-1996-hotspots`

use crate::types::PositionId;
use serde::{Deserialize, Serialize};
use tracing::warn;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Method used for position-level VaR/ES decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DecompositionMethod {
    /// Covariance-based using normal distribution assumption.
    ///
    /// Fast (O(n^2) in positions). Exact Euler property.
    /// Requires a position-level return covariance matrix.
    Parametric,

    /// Full historical simulation with per-position P&L attribution.
    ///
    /// Slow (O(n * scenarios)). Approximate Euler property.
    /// Handles non-normality and non-linear positions.
    Historical,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for position-level VaR decomposition.
#[derive(Debug, Clone)]
pub struct DecompositionConfig {
    /// Confidence level for VaR and ES (e.g. 0.95, 0.99).
    pub confidence: f64,

    /// Decomposition method.
    pub method: DecompositionMethod,

    /// Whether to compute incremental VaR (expensive: one full repricing
    /// per position).
    pub compute_incremental: bool,
}

impl DecompositionConfig {
    /// Standard 95% parametric configuration.
    pub fn parametric_95() -> Self {
        Self {
            confidence: 0.95,
            method: DecompositionMethod::Parametric,
            compute_incremental: false,
        }
    }

    /// Standard 99% parametric configuration.
    pub fn parametric_99() -> Self {
        Self {
            confidence: 0.99,
            method: DecompositionMethod::Parametric,
            compute_incremental: false,
        }
    }

    /// Historical simulation configuration.
    pub fn historical(confidence: f64) -> Self {
        Self {
            confidence,
            method: DecompositionMethod::Historical,
            compute_incremental: false,
        }
    }

    /// Enable incremental VaR computation.
    pub fn with_incremental(mut self) -> Self {
        self.compute_incremental = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Per-position result structs
// ---------------------------------------------------------------------------

/// Risk decomposition result for a single portfolio position.
///
/// All monetary fields are in the same units as the portfolio VaR
/// (typically the portfolio's base currency).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionVarContribution {
    /// Position identifier.
    pub position_id: PositionId,

    /// Component VaR: position's Euler-allocated share of portfolio VaR.
    ///
    /// Sum of all component VaRs equals total portfolio VaR (exact under
    /// the parametric normal assumption; approximate for historical).
    ///
    /// Formula (parametric):
    /// ```text
    /// CVaR_i = w_i * (Sigma * w)_i / sigma_p * z_alpha
    /// ```
    pub component_var: f64,

    /// Component VaR as a fraction of total portfolio VaR.
    ///
    /// `relative_var = component_var / portfolio_var`. Sums to 1.0.
    /// A negative value indicates the position is a diversifier.
    pub relative_var: f64,

    /// Marginal VaR: per-unit sensitivity of portfolio VaR to this position.
    ///
    /// Formula (parametric):
    /// ```text
    /// MVaR_i = (Sigma * w)_i / sigma_p * z_alpha
    /// ```
    ///
    /// Used as gradient input for mean-variance optimization and
    /// risk-budgeting rebalancing.
    ///
    /// `None` when the engine cannot produce a true gradient (e.g.
    /// historical mode without finite-difference repricing); callers that
    /// need a marginal must choose a fallback or skip rebalancing.
    pub marginal_var: Option<f64>,

    /// Incremental VaR: change in portfolio VaR from removing this position.
    ///
    /// ```text
    /// IVaR_i = VaR(portfolio) - VaR(portfolio \ {i})
    /// ```
    ///
    /// Requires full repricing for each position removal. `None` if
    /// incremental VaR was not requested (it is expensive).
    pub incremental_var: Option<f64>,
}

/// Expected Shortfall decomposition result for a single portfolio position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionEsContribution {
    /// Position identifier.
    pub position_id: PositionId,

    /// Component ES: position's contribution to portfolio Expected Shortfall.
    ///
    /// Parametric: analytical formula using truncated normal moments.
    /// ```text
    /// CES_i = w_i * (Sigma * w)_i / sigma_p * phi(z_alpha) / (1 - alpha)
    /// ```
    ///
    /// Historical: average of position-level losses in tail scenarios.
    /// ```text
    /// CES_i = E[L_i | L_portfolio > VaR_portfolio]
    /// ```
    pub component_es: f64,

    /// Component ES as a fraction of total portfolio ES.
    pub relative_es: f64,

    /// Marginal ES: per-unit sensitivity of portfolio ES to this position.
    ///
    /// `None` when the engine cannot produce a true gradient (e.g.
    /// historical mode without finite-difference repricing).
    pub marginal_es: Option<f64>,
}

// ---------------------------------------------------------------------------
// Aggregate result
// ---------------------------------------------------------------------------

/// Complete position-level risk decomposition of a portfolio.
///
/// Contains VaR and ES decompositions for every position, along with
/// portfolio-level totals. All values are in the portfolio's base currency.
///
/// # Euler Decomposition Property
///
/// Under the parametric (normal) assumption:
/// ```text
/// sum(component_var_i) == portfolio_var  (exact)
/// sum(component_es_i)  == portfolio_es   (exact)
/// ```
///
/// Under historical simulation, the Euler property holds approximately.
///
/// # References
///
/// - Tasche (2008): Capital allocation with Euler's method. `docs/REFERENCES.md#tasche-2008-capital-allocation`
/// - Meucci (2005): Risk and Asset Allocation. `docs/REFERENCES.md#meucci-risk-and-asset-allocation`
/// - Litterman (1996): Hot Spots and Hedges. `docs/REFERENCES.md#litterman-1996-hotspots`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRiskDecomposition {
    /// Total portfolio VaR.
    pub portfolio_var: f64,

    /// Total portfolio Expected Shortfall.
    pub portfolio_es: f64,

    /// Confidence level used for both VaR and ES.
    pub confidence: f64,

    /// Method used for decomposition.
    pub method: DecompositionMethod,

    /// Per-position VaR decomposition.
    pub var_contributions: Vec<PositionVarContribution>,

    /// Per-position ES decomposition.
    pub es_contributions: Vec<PositionEsContribution>,

    /// Number of positions in the portfolio.
    pub n_positions: usize,

    /// Residual from Euler decomposition (should be near zero).
    ///
    /// `residual = portfolio_var - sum(component_var_i)`.
    /// Only meaningful for the parametric engine, where a non-zero residual
    /// signals a numerical issue (ill-conditioned covariance, floating-point
    /// accumulation error).
    ///
    /// `None` in historical mode: the Tasche scaling used there makes the
    /// residual algebraically zero by construction, so it carries no
    /// diagnostic information.
    pub euler_residual: Option<f64>,
}

// ---------------------------------------------------------------------------
// Stress attribution (historical)
// ---------------------------------------------------------------------------

/// Per-position attribution of portfolio losses in tail scenarios.
///
/// For each scenario that breaches the VaR threshold, reports which
/// positions contributed the most to the portfolio loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressAttribution {
    /// Portfolio VaR threshold (losses exceeding this are "tail events").
    pub var_threshold: f64,

    /// Number of tail scenarios analyzed.
    pub n_tail_scenarios: usize,

    /// Per-position average contribution to tail losses.
    ///
    /// Sorted by absolute contribution (largest risk driver first).
    pub position_contributions: Vec<StressPositionEntry>,

    /// Individual tail scenario breakdowns.
    ///
    /// Contains `n_tail_scenarios` entries, each with per-position P&L.
    /// Sorted by portfolio loss (worst first).
    pub tail_scenarios: Vec<TailScenarioBreakdown>,
}

/// Single position's contribution to tail stress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressPositionEntry {
    /// Position identifier.
    pub position_id: PositionId,

    /// Average P&L contribution in tail scenarios.
    pub avg_tail_pnl: f64,

    /// Fraction of total portfolio tail loss attributable to this position.
    pub pct_of_tail_loss: f64,

    /// Worst single-scenario P&L for this position.
    pub worst_scenario_pnl: f64,
}

/// Breakdown of a single tail scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailScenarioBreakdown {
    /// Scenario index in the original history.
    pub scenario_index: usize,

    /// Total portfolio P&L for this scenario.
    pub portfolio_pnl: f64,

    /// Per-position P&L contributions.
    pub position_pnls: Vec<(PositionId, f64)>,
}

// ---------------------------------------------------------------------------
// Shared math helpers
// ---------------------------------------------------------------------------

const VARIANCE_TOLERANCE: f64 = 1e-12;

use super::math::{normal_pdf, normal_quantile};

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_decomposition_inputs(
    weights: &[f64],
    covariance: &[f64],
    position_ids: &[PositionId],
    config: &DecompositionConfig,
) -> finstack_core::Result<()> {
    let n = weights.len();

    if n != position_ids.len() {
        return Err(finstack_core::Error::Validation(format!(
            "weights length ({n}) must match position_ids length ({})",
            position_ids.len()
        )));
    }

    if covariance.len() != n * n {
        return Err(finstack_core::Error::Validation(format!(
            "covariance length ({}) must be {}x{} = {}",
            covariance.len(),
            n,
            n,
            n * n
        )));
    }

    if config.confidence <= 0.0 || config.confidence >= 1.0 {
        return Err(finstack_core::Error::Validation(format!(
            "confidence must be in (0, 1), got {}",
            config.confidence
        )));
    }

    // Check finite entries.
    if covariance.iter().any(|v| !v.is_finite()) {
        return Err(finstack_core::Error::Validation(
            "covariance matrix entries must be finite".to_string(),
        ));
    }

    if weights.iter().any(|v| !v.is_finite()) {
        return Err(finstack_core::Error::Validation(
            "weight entries must be finite".to_string(),
        ));
    }

    // Check symmetry.
    for i in 0..n {
        for j in (i + 1)..n {
            if (covariance[i * n + j] - covariance[j * n + i]).abs() > VARIANCE_TOLERANCE {
                return Err(finstack_core::Error::Validation(format!(
                    "covariance matrix is not symmetric at ({i}, {j})"
                )));
            }
        }
    }

    // Positive semi-definiteness via Cholesky (only if n > 0).
    if n > 0 {
        finstack_core::math::linalg::cholesky_decomposition(covariance, n).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "covariance matrix is not positive semi-definite: {e}"
            ))
        })?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental VaR
// ---------------------------------------------------------------------------

/// Compute incremental VaR for all positions in O(n) total.
///
/// Textbook definition: incremental VaR for position `k` is the change in
/// portfolio VaR caused by removing position `k`, with the remaining
/// positions held at their existing weights (no renormalization):
///
/// ```text
///   variance_excl_k = w' Σ w  -  2 w_k (Σ w)_k  +  w_k²  Σ_{kk}
///   var_excl_k      = z_α · sqrt(max(variance_excl_k, 0))
///   incremental_k   = portfolio_var - var_excl_k
/// ```
///
/// This matches Jorion (2007) §7.2.3 and the definition used in standard
/// risk-system reference implementations. It differs from the older
/// implementation in this file, which renormalized the remaining weights
/// by `S - w_k` (where `S = Σ_i w_i`). The renormalized form silently
/// magnifies `var_excl_k` when `S - w_k` is small and produces
/// counter-intuitive negative incrementals for non-diversifying positions;
/// it is not a textbook quantity.
///
/// The `sigma_w` argument must equal `Σ w`; this keeps the routine
/// allocation- and matrix-free.
fn compute_incremental_var(
    weights: &[f64],
    sigma_w: &[f64],
    covariance: &[f64],
    portfolio_variance: f64,
    portfolio_var: f64,
    confidence: f64,
    n: usize,
) -> Vec<f64> {
    let z_alpha = normal_quantile(confidence);

    (0..n)
        .map(|k| {
            let w_k = weights[k];
            let sw_k = sigma_w[k];
            let cov_kk = covariance[k * n + k];

            let variance_excl =
                (portfolio_variance - 2.0 * w_k * sw_k + w_k * w_k * cov_kk).max(0.0);

            let var_excl = z_alpha * variance_excl.sqrt();
            portfolio_var - var_excl
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Parametric engine
// ---------------------------------------------------------------------------

/// Parametric (covariance-based) position-level VaR decomposer.
///
/// Uses the multivariate normal assumption to decompose VaR and ES
/// analytically via Euler allocation. Fast and exact under normality.
///
/// # Mathematical Background
///
/// Under the normal model, portfolio return `r_p = w'r` has:
/// ```text
/// sigma_p = sqrt(w' * Sigma * w)
/// VaR_p   = z_alpha * sigma_p  (zero-mean assumption for risk)
/// ```
///
/// The Euler decomposition exploits the positive homogeneity of VaR:
/// ```text
/// VaR_p = sum_i (w_i * dVaR/dw_i) = sum_i CVaR_i
/// ```
///
/// # References
///
/// - Litterman (1996): Hot Spots and Hedges. `docs/REFERENCES.md#litterman-1996-hotspots`
/// - Tasche (2008): Capital allocation with Euler's method. `docs/REFERENCES.md#tasche-2008-capital-allocation`
#[derive(Debug, Clone, Copy, Default)]
pub struct ParametricPositionDecomposer;

impl ParametricPositionDecomposer {
    /// Decompose portfolio VaR and ES into per-position contributions using Euler allocation.
    ///
    /// # Arguments
    ///
    /// * `weights` - Position weights as fraction of portfolio value (length `n_positions`).
    /// * `covariance` - Position-return covariance matrix (n x n, row-major, symmetric PSD).
    /// * `position_ids` - Position identifiers, aligned with `weights`.
    /// * `config` - Decomposition parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are inconsistent, the covariance matrix is invalid, or
    /// the confidence level is out of bounds.
    pub fn decompose_positions(
        &self,
        weights: &[f64],
        covariance: &[f64],
        position_ids: &[PositionId],
        config: &DecompositionConfig,
    ) -> finstack_core::Result<PositionRiskDecomposition> {
        validate_decomposition_inputs(weights, covariance, position_ids, config)?;

        let n = weights.len();

        // Empty portfolio.
        if n == 0 {
            return Ok(PositionRiskDecomposition {
                portfolio_var: 0.0,
                portfolio_es: 0.0,
                confidence: config.confidence,
                method: DecompositionMethod::Parametric,
                var_contributions: Vec::new(),
                es_contributions: Vec::new(),
                n_positions: 0,
                euler_residual: Some(0.0),
            });
        }

        let z_alpha = normal_quantile(config.confidence);
        let phi_z = normal_pdf(z_alpha);
        let es_multiplier = phi_z / (1.0 - config.confidence);

        // Sigma * w (matrix-vector product).
        let mut sigma_w = vec![0.0; n];
        for i in 0..n {
            let mut dot = 0.0;
            for j in 0..n {
                dot += covariance[i * n + j] * weights[j];
            }
            sigma_w[i] = dot;
        }

        // Portfolio variance = w' * Sigma * w.
        let mut raw_variance = 0.0;
        for i in 0..n {
            raw_variance += weights[i] * sigma_w[i];
        }
        if raw_variance < 0.0 {
            warn!(
                raw_variance,
                "parametric decomposer: w' Sigma w was negative after Cholesky validation; \
                 clamping to zero. Covariance matrix is likely numerically singular."
            );
        }
        let variance = raw_variance.max(0.0);
        let sigma_p = variance.sqrt();

        let portfolio_var = sigma_p * z_alpha;
        let portfolio_es = sigma_p * es_multiplier;

        // Guard against zero-risk portfolio to avoid division by zero.
        let inv_sigma = if sigma_p > VARIANCE_TOLERANCE.sqrt() {
            1.0 / sigma_p
        } else {
            warn!(
                sigma_p,
                "parametric decomposer: portfolio sigma below sqrt(tolerance); marginal and \
                 component contributions will be zero. Portfolio may be degenerate or all \
                 weights near zero."
            );
            0.0
        };

        // Per-position decomposition.
        let mut var_contributions = Vec::with_capacity(n);
        let mut es_contributions = Vec::with_capacity(n);

        for i in 0..n {
            // Component variance = w_i * (Sigma * w)_i.
            let cv_i = weights[i] * sigma_w[i];

            // Component VaR = CV_i / sigma_p * z_alpha.
            let component_var = cv_i * inv_sigma * z_alpha;

            // Marginal VaR = (Sigma * w)_i / sigma_p * z_alpha.
            let marginal_var = sigma_w[i] * inv_sigma * z_alpha;

            // Relative VaR = CVaR_i / VaR_p.
            let relative_var = if portfolio_var.abs() > VARIANCE_TOLERANCE {
                component_var / portfolio_var
            } else {
                0.0
            };

            // Component ES = CV_i / sigma_p * phi(z_alpha) / (1 - alpha).
            let component_es = cv_i * inv_sigma * es_multiplier;

            // Marginal ES = (Sigma * w)_i / sigma_p * phi(z_alpha) / (1 - alpha).
            let marginal_es = sigma_w[i] * inv_sigma * es_multiplier;

            // Relative ES = CES_i / ES_p.
            let relative_es = if portfolio_es.abs() > VARIANCE_TOLERANCE {
                component_es / portfolio_es
            } else {
                0.0
            };

            var_contributions.push(PositionVarContribution {
                position_id: position_ids[i].clone(),
                component_var,
                relative_var,
                marginal_var: Some(marginal_var),
                incremental_var: None,
            });

            es_contributions.push(PositionEsContribution {
                position_id: position_ids[i].clone(),
                component_es,
                relative_es,
                marginal_es: Some(marginal_es),
            });
        }

        // Incremental VaR (expensive leave-one-out).
        if config.compute_incremental && n > 1 {
            let incremental = compute_incremental_var(
                weights,
                &sigma_w,
                covariance,
                variance,
                portfolio_var,
                config.confidence,
                n,
            );
            for (contribution, ivar) in var_contributions.iter_mut().zip(incremental.into_iter()) {
                contribution.incremental_var = Some(ivar);
            }
        } else if config.compute_incremental && n == 1 {
            // Single-position portfolio: incremental VaR equals portfolio VaR.
            var_contributions[0].incremental_var = Some(portfolio_var);
        }

        // Euler residual (parametric only; meaningful as a numerical diagnostic).
        let sum_component_var: f64 = var_contributions.iter().map(|c| c.component_var).sum();
        let euler_residual = Some(portfolio_var - sum_component_var);

        Ok(PositionRiskDecomposition {
            portfolio_var,
            portfolio_es,
            confidence: config.confidence,
            method: DecompositionMethod::Parametric,
            var_contributions,
            es_contributions,
            n_positions: n,
            euler_residual,
        })
    }
}

// ---------------------------------------------------------------------------
// Historical simulation engine
// ---------------------------------------------------------------------------

/// Historical simulation position-level VaR decomposer.
///
/// Decomposes VaR and ES by attributing portfolio losses to individual
/// positions within tail scenarios. The Euler property holds approximately
/// (exact in the limit of infinite scenarios).
///
/// # Algorithm
///
/// 1. Compute portfolio P&L for each scenario: PnL_p(s) = sum_i PnL_i(s)
/// 2. Sort scenarios by portfolio P&L (ascending = worst first)
/// 3. Identify tail: scenarios where PnL_p <= -VaR_p
/// 4. Component ES: CES_i = mean(-PnL_i(s)) for s in tail
/// 5. Component VaR: CVaR_i = CES_i * (VaR_p / ES_p)  (Tasche scaling)
///
/// # References
///
/// - Hallerbach (2003): Decomposing portfolio Value-at-Risk.
///   `docs/REFERENCES.md#hallerbach-2003-decomposing-var`
#[derive(Debug, Clone, Default)]
pub struct HistoricalPositionDecomposer;

impl HistoricalPositionDecomposer {
    /// Decompose using pre-computed per-position scenario P&Ls.
    ///
    /// # Arguments
    ///
    /// * `position_pnls` - Matrix of per-position P&Ls, shape (n_scenarios, n_positions),
    ///   stored row-major. `position_pnls[s * n_positions + i]` is position `i`'s
    ///   P&L under scenario `s`.
    /// * `position_ids` - Position identifiers, length `n_positions`.
    /// * `n_scenarios` - Number of historical scenarios.
    /// * `config` - Decomposition parameters (only `confidence` is used;
    ///   `method` is ignored since this is always historical).
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are inconsistent, the number of
    /// scenarios is too small, or the confidence level is out of bounds.
    pub fn decompose_from_pnls(
        &self,
        position_pnls: &[f64],
        position_ids: &[PositionId],
        n_scenarios: usize,
        config: &DecompositionConfig,
    ) -> finstack_core::Result<PositionRiskDecomposition> {
        let n = position_ids.len();

        if position_pnls.len() != n_scenarios * n {
            return Err(finstack_core::Error::Validation(format!(
                "position_pnls length ({}) must equal n_scenarios ({}) * n_positions ({})",
                position_pnls.len(),
                n_scenarios,
                n
            )));
        }

        if config.confidence <= 0.0 || config.confidence >= 1.0 {
            return Err(finstack_core::Error::Validation(format!(
                "confidence must be in (0, 1), got {}",
                config.confidence
            )));
        }

        if n == 0 || n_scenarios == 0 {
            return Ok(PositionRiskDecomposition {
                portfolio_var: 0.0,
                portfolio_es: 0.0,
                confidence: config.confidence,
                method: DecompositionMethod::Historical,
                var_contributions: Vec::new(),
                es_contributions: Vec::new(),
                n_positions: n,
                euler_residual: None,
            });
        }

        // Reject configurations where the tail would contain less than one
        // scenario. (1 - confidence) * n_scenarios < 1 means the stated
        // confidence level cannot be resolved by the sample and any VaR/ES
        // estimate would be dominated by a single extreme observation.
        let expected_tail = (1.0 - config.confidence) * n_scenarios as f64;
        if expected_tail < 1.0 {
            return Err(finstack_core::Error::Validation(format!(
                "historical decomposition: (1 - confidence) * n_scenarios = {expected_tail} < 1; \
                 need at least {:.0} scenarios at confidence {} to resolve the tail",
                (1.0 / (1.0 - config.confidence)).ceil(),
                config.confidence
            )));
        }

        // Compute portfolio P&L for each scenario.
        let mut portfolio_pnls: Vec<(usize, f64)> = (0..n_scenarios)
            .map(|s| {
                let row_start = s * n;
                let pnl: f64 = position_pnls[row_start..row_start + n].iter().sum();
                (s, pnl)
            })
            .collect();

        // Sort ascending by portfolio P&L (worst first).
        portfolio_pnls.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Number of tail scenarios = floor((1 - confidence) * n_scenarios).
        // The C2 guard above ensures this is >= 1.
        let n_tail = ((1.0 - config.confidence) * n_scenarios as f64).floor() as usize;

        // Portfolio VaR: negative of the worst-case boundary scenario. The
        // tail spans sorted indices 0..n_tail (ascending P&L), so the VaR
        // threshold is the least-bad scenario of the tail, index n_tail-1.
        let var_idx = (n_tail - 1).min(n_scenarios - 1);
        let portfolio_var = -portfolio_pnls[var_idx].1;

        // Portfolio ES: average loss in tail scenarios.
        let portfolio_es: f64 = -portfolio_pnls[..n_tail]
            .iter()
            .map(|(_, pnl)| pnl)
            .sum::<f64>()
            / n_tail as f64;

        // Per-position Component ES: average of position-level losses in tail.
        // For large (n_tail * n) problems, reading the per-tail-scenario loss
        // rows in parallel is worthwhile; the subsequent accumulation runs
        // serially in input order so the floating-point sum is bit-identical
        // across runs. For small problems the allocation overhead of the
        // intermediate `tail_rows` Vec outweighs any gain, so we fall back to
        // an in-place serial accumulation that matches the original behavior.
        const PARALLEL_TAIL_THRESHOLD: usize = 100_000;
        let mut component_es_vec = vec![0.0; n];
        if n_tail.saturating_mul(n) >= PARALLEL_TAIL_THRESHOLD {
            use rayon::prelude::*;
            let tail_rows: Vec<Vec<f64>> = portfolio_pnls[..n_tail]
                .par_iter()
                .map(|&(s, _)| {
                    let row_start = s * n;
                    (0..n).map(|i| -position_pnls[row_start + i]).collect()
                })
                .collect();
            for row in &tail_rows {
                for i in 0..n {
                    component_es_vec[i] += row[i];
                }
            }
        } else {
            for &(s, _) in &portfolio_pnls[..n_tail] {
                let row_start = s * n;
                for i in 0..n {
                    component_es_vec[i] += -position_pnls[row_start + i];
                }
            }
        }
        for ces in component_es_vec.iter_mut() {
            *ces /= n_tail as f64;
        }

        // Component VaR via Tasche scaling: CVaR_i = CES_i * (VaR / ES).
        let var_es_ratio = if portfolio_es.abs() > VARIANCE_TOLERANCE {
            portfolio_var / portfolio_es
        } else {
            1.0
        };
        let component_var_vec: Vec<f64> = component_es_vec
            .iter()
            .map(|ces| ces * var_es_ratio)
            .collect();

        // Marginal VaR/ES are not analytically available from raw scenario
        // P&Ls: they require either position weights (to differentiate)
        // or a finite-difference repricing engine. Report None rather than
        // a misleading proxy value.
        let mut var_contributions = Vec::with_capacity(n);
        let mut es_contributions = Vec::with_capacity(n);

        for i in 0..n {
            let relative_var = if portfolio_var.abs() > VARIANCE_TOLERANCE {
                component_var_vec[i] / portfolio_var
            } else {
                0.0
            };

            let relative_es = if portfolio_es.abs() > VARIANCE_TOLERANCE {
                component_es_vec[i] / portfolio_es
            } else {
                0.0
            };

            var_contributions.push(PositionVarContribution {
                position_id: position_ids[i].clone(),
                component_var: component_var_vec[i],
                relative_var,
                marginal_var: None,
                incremental_var: None,
            });

            es_contributions.push(PositionEsContribution {
                position_id: position_ids[i].clone(),
                component_es: component_es_vec[i],
                relative_es,
                marginal_es: None,
            });
        }

        // Euler residual is algebraically zero in historical mode because
        // CVaR_i = CES_i * (VaR/ES) and sum(CES_i) = ES by construction.
        // Reporting it as None avoids implying a diagnostic that does not
        // exist here.
        Ok(PositionRiskDecomposition {
            portfolio_var,
            portfolio_es,
            confidence: config.confidence,
            method: DecompositionMethod::Historical,
            var_contributions,
            es_contributions,
            n_positions: n,
            euler_residual: None,
        })
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    type TestResult = finstack_core::Result<()>;

    // -----------------------------------------------------------------------
    // Parametric tests
    // -----------------------------------------------------------------------

    #[test]
    fn euler_exhaustion_two_position_portfolio() -> TestResult {
        // Two uncorrelated assets: sigma1 = 0.20, sigma2 = 0.30.
        // Weights: 0.6, 0.4.
        let weights = [0.6, 0.4];
        let covariance = [0.04, 0.0, 0.0, 0.09];
        let ids = [PositionId::new("A"), PositionId::new("B")];
        let config = DecompositionConfig::parametric_99();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        // sum(component_var) must equal portfolio_var.
        let sum_cvar: f64 = result
            .var_contributions
            .iter()
            .map(|c| c.component_var)
            .sum();
        assert!(
            (sum_cvar - result.portfolio_var).abs() < 1e-10,
            "Euler exhaustion failed: sum={sum_cvar}, total={}",
            result.portfolio_var
        );

        // sum(relative_var) must equal 1.0.
        let sum_rel: f64 = result
            .var_contributions
            .iter()
            .map(|c| c.relative_var)
            .sum();
        assert!(
            (sum_rel - 1.0).abs() < 1e-10,
            "relative VaR sum failed: {sum_rel}"
        );

        // Euler residual should be Some(~0) in parametric mode.
        let residual = result
            .euler_residual
            .expect("parametric decomposition must report euler_residual");
        assert!(residual.abs() < 1e-10, "euler_residual = {residual}");

        Ok(())
    }

    #[test]
    fn equal_weight_equal_vol_has_equal_component_var() -> TestResult {
        // Three assets, all identical vol, zero correlation.
        let vol = 0.15;
        let var = vol * vol;
        let n = 3;
        let w = 1.0 / n as f64;
        let weights = vec![w; n];
        let mut covariance = vec![0.0; n * n];
        for i in 0..n {
            covariance[i * n + i] = var;
        }
        let ids: Vec<PositionId> = (0..n).map(|i| PositionId::new(format!("P{i}"))).collect();
        let config = DecompositionConfig::parametric_95();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        // All component VaRs should be equal.
        let first_cvar = result.var_contributions[0].component_var;
        for c in &result.var_contributions {
            assert!(
                (c.component_var - first_cvar).abs() < 1e-12,
                "unequal component VaR: {} vs {first_cvar}",
                c.component_var
            );
        }

        // Each relative VaR should be 1/n.
        for c in &result.var_contributions {
            assert!(
                (c.relative_var - w).abs() < 1e-12,
                "relative VaR {} != expected {w}",
                c.relative_var
            );
        }

        Ok(())
    }

    #[test]
    fn single_position_portfolio() -> TestResult {
        let weights = [1.0];
        let covariance = [0.04]; // sigma = 0.20
        let ids = [PositionId::new("SOLO")];
        let config = DecompositionConfig::parametric_95().with_incremental();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        // Component VaR == portfolio VaR.
        assert!((result.var_contributions[0].component_var - result.portfolio_var).abs() < 1e-12);

        // Marginal VaR == portfolio VaR (single position, weight = 1).
        let mvar = result.var_contributions[0]
            .marginal_var
            .expect("parametric: marginal_var must be Some");
        assert!((mvar - result.portfolio_var).abs() < 1e-12);

        // Incremental VaR == portfolio VaR.
        let ivar = result.var_contributions[0]
            .incremental_var
            .unwrap_or(f64::NAN);
        assert!(
            (ivar - result.portfolio_var).abs() < 1e-12,
            "incremental VaR {ivar} != portfolio VaR {}",
            result.portfolio_var
        );

        // Relative VaR == 1.0.
        assert!((result.var_contributions[0].relative_var - 1.0).abs() < 1e-12);

        Ok(())
    }

    #[test]
    fn zero_weight_position_has_zero_contributions() -> TestResult {
        let weights = [1.0, 0.0];
        let covariance = [0.04, 0.01, 0.01, 0.09];
        let ids = [PositionId::new("A"), PositionId::new("ZERO")];
        let config = DecompositionConfig::parametric_95();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        let zero_pos = &result.var_contributions[1];
        assert!(
            zero_pos.component_var.abs() < 1e-12,
            "zero-weight component VaR = {}",
            zero_pos.component_var
        );
        assert!(!zero_pos.component_var.is_nan());
        let mvar = zero_pos
            .marginal_var
            .expect("parametric: marginal_var must be Some");
        assert!(!mvar.is_nan());
        assert!(!zero_pos.relative_var.is_nan());

        Ok(())
    }

    #[test]
    fn es_ge_var_for_all_positions() -> TestResult {
        // ES should always be >= VaR at the same confidence level.
        let weights = [0.4, 0.3, 0.3];
        let covariance = [0.04, 0.01, 0.005, 0.01, 0.09, 0.02, 0.005, 0.02, 0.0625];
        let ids = [
            PositionId::new("A"),
            PositionId::new("B"),
            PositionId::new("C"),
        ];
        let config = DecompositionConfig::parametric_99();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        assert!(
            result.portfolio_es >= result.portfolio_var,
            "portfolio ES ({}) < VaR ({})",
            result.portfolio_es,
            result.portfolio_var
        );

        for (vc, ec) in result
            .var_contributions
            .iter()
            .zip(result.es_contributions.iter())
        {
            // For positive component VaR, ES component should be >= VaR component.
            if vc.component_var > 0.0 {
                assert!(
                    ec.component_es >= vc.component_var - 1e-12,
                    "position {} ES ({}) < VaR ({})",
                    vc.position_id,
                    ec.component_es,
                    vc.component_var
                );
            }
        }

        Ok(())
    }

    #[test]
    fn negative_correlation_shows_diversification() -> TestResult {
        // Two positions with high negative correlation.
        let weights = [0.5, 0.5];
        // sigma1 = 0.2, sigma2 = 0.2, rho = -0.8
        // cov(1,2) = rho * sigma1 * sigma2 = -0.8 * 0.04 = -0.032
        let covariance = [0.04, -0.032, -0.032, 0.04];
        let ids = [PositionId::new("A"), PositionId::new("B")];
        let config = DecompositionConfig::parametric_95();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        // Portfolio VaR should be much less than sum of standalone VaRs.
        let z = normal_quantile(0.95);
        let standalone_var_a = 0.5 * 0.2 * z;
        let standalone_var_b = 0.5 * 0.2 * z;
        let sum_standalone = standalone_var_a + standalone_var_b;

        assert!(
            result.portfolio_var < sum_standalone,
            "portfolio VaR ({}) should be less than sum of standalone VaRs ({sum_standalone})",
            result.portfolio_var
        );

        // Both component VaRs should be positive (even with negative corr).
        for c in &result.var_contributions {
            assert!(
                c.component_var > 0.0,
                "component VaR for {} should be positive: {}",
                c.position_id,
                c.component_var
            );
        }

        // Euler still holds.
        let sum_cvar: f64 = result
            .var_contributions
            .iter()
            .map(|c| c.component_var)
            .sum();
        assert!((sum_cvar - result.portfolio_var).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn euler_exhaustion_five_positions() -> TestResult {
        // 5-position portfolio with a realistic covariance structure.
        let weights = [0.15, 0.25, 0.20, 0.25, 0.15];
        // Build a PSD covariance matrix from a Cholesky factor.
        let n = 5;
        // Lower triangular L (hand-crafted to ensure PSD).
        #[rustfmt::skip]
        let l = [
            0.20, 0.00, 0.00, 0.00, 0.00,
            0.05, 0.18, 0.00, 0.00, 0.00,
            0.03, 0.04, 0.22, 0.00, 0.00,
            0.02, 0.06, 0.03, 0.15, 0.00,
            0.01, 0.02, 0.05, 0.04, 0.19,
        ];
        // Sigma = L * L'.
        let mut covariance = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..n {
                    sum += l[i * n + k] * l[j * n + k];
                }
                covariance[i * n + j] = sum;
            }
        }

        let ids: Vec<PositionId> = (0..n).map(|i| PositionId::new(format!("P{i}"))).collect();
        let config = DecompositionConfig::parametric_99();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        let sum_cvar: f64 = result
            .var_contributions
            .iter()
            .map(|c| c.component_var)
            .sum();
        assert!(
            (sum_cvar - result.portfolio_var).abs() < 1e-10,
            "5-pos Euler exhaustion: sum={sum_cvar}, total={}",
            result.portfolio_var
        );

        let sum_ces: f64 = result.es_contributions.iter().map(|c| c.component_es).sum();
        assert!(
            (sum_ces - result.portfolio_es).abs() < 1e-10,
            "5-pos ES Euler exhaustion: sum={sum_ces}, total={}",
            result.portfolio_es
        );

        Ok(())
    }

    #[test]
    fn empty_portfolio_returns_zero() -> TestResult {
        let decomposer = ParametricPositionDecomposer;
        let result =
            decomposer.decompose_positions(&[], &[], &[], &DecompositionConfig::parametric_95())?;

        assert!(result.portfolio_var.abs() < 1e-12);
        assert!(result.portfolio_es.abs() < 1e-12);
        assert_eq!(result.n_positions, 0);
        assert!(result.var_contributions.is_empty());
        assert!(result.es_contributions.is_empty());

        Ok(())
    }

    #[test]
    fn rejects_mismatched_dimensions() {
        let decomposer = ParametricPositionDecomposer;

        // Weights longer than position_ids.
        let result = decomposer.decompose_positions(
            &[0.5, 0.5],
            &[0.04, 0.0, 0.0, 0.04],
            &[PositionId::new("A")],
            &DecompositionConfig::parametric_95(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_confidence() {
        let decomposer = ParametricPositionDecomposer;
        let mut config = DecompositionConfig::parametric_95();
        config.confidence = 1.5;

        let result =
            decomposer.decompose_positions(&[1.0], &[0.04], &[PositionId::new("A")], &config);
        assert!(result.is_err());
    }

    #[test]
    fn incremental_var_three_positions() -> TestResult {
        let weights = [0.4, 0.35, 0.25];
        let covariance = [0.04, 0.01, 0.005, 0.01, 0.09, 0.02, 0.005, 0.02, 0.0625];
        let ids = [
            PositionId::new("A"),
            PositionId::new("B"),
            PositionId::new("C"),
        ];
        let config = DecompositionConfig::parametric_99().with_incremental();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        // All incremental VaRs should be present.
        for c in &result.var_contributions {
            assert!(
                c.incremental_var.is_some(),
                "incremental VaR missing for {}",
                c.position_id
            );
        }

        // Incremental VaRs should be finite.
        for c in &result.var_contributions {
            let ivar = c.incremental_var.unwrap_or(f64::NAN);
            assert!(
                ivar.is_finite(),
                "incremental VaR for {} should be finite: {ivar}",
                c.position_id
            );
        }

        // Position B (highest standalone vol = 0.30) should have
        // the largest incremental VaR since removing it reduces risk most.
        let ivar_b = result.var_contributions[1].incremental_var.unwrap_or(0.0);
        let ivar_c = result.var_contributions[2].incremental_var.unwrap_or(0.0);
        assert!(
            ivar_b > ivar_c,
            "position B (higher vol) should have larger incremental VaR than C: B={ivar_b}, C={ivar_c}"
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Historical decomposition tests
    // -----------------------------------------------------------------------

    #[test]
    fn historical_decomposition_basic() -> TestResult {
        // 100 scenarios, 2 positions.
        // Position A: steady small losses around -0.01.
        // Position B: occasional large losses.
        let n = 2;
        let n_scenarios = 100;
        let mut pnls = Vec::with_capacity(n_scenarios * n);

        for s in 0..n_scenarios {
            let a_pnl = -0.01 + 0.001 * (s as f64 / 10.0).sin();
            let b_pnl = if s < 5 {
                -0.10 // Tail scenario for B.
            } else {
                0.005 + 0.002 * (s as f64 / 5.0).cos()
            };
            pnls.push(a_pnl);
            pnls.push(b_pnl);
        }

        let ids = [PositionId::new("A"), PositionId::new("B")];
        let config = DecompositionConfig::historical(0.95);

        let decomposer = HistoricalPositionDecomposer;
        let result = decomposer.decompose_from_pnls(&pnls, &ids, n_scenarios, &config)?;

        assert!(
            result.portfolio_var > 0.0,
            "portfolio VaR should be positive"
        );
        assert!(
            result.portfolio_es >= result.portfolio_var,
            "ES should >= VaR"
        );
        assert_eq!(result.n_positions, 2);
        assert_eq!(result.method, DecompositionMethod::Historical);

        Ok(())
    }

    #[test]
    fn historical_rejects_dimension_mismatch() {
        let decomposer = HistoricalPositionDecomposer;
        let result = decomposer.decompose_from_pnls(
            &[1.0, 2.0, 3.0], // 3 values, but 2 scenarios x 2 positions = 4.
            &[PositionId::new("A"), PositionId::new("B")],
            2,
            &DecompositionConfig::historical(0.95),
        );
        assert!(result.is_err());
    }

    #[test]
    fn historical_empty_returns_zero() -> TestResult {
        let decomposer = HistoricalPositionDecomposer;
        let result =
            decomposer.decompose_from_pnls(&[], &[], 0, &DecompositionConfig::historical(0.95))?;

        assert!(result.portfolio_var.abs() < 1e-12);
        assert_eq!(result.n_positions, 0);
        Ok(())
    }

    // C1 regression: VaR quantile index is the boundary of the tail, not
    // one-past-the-end. With 100 equally-spaced sorted P&Ls and 95%
    // confidence, the tail spans indices 0..5; VaR = -pnl[4] (index n_tail-1),
    // not -pnl[5].
    #[test]
    fn historical_var_uses_boundary_tail_index() -> TestResult {
        // 100 scenarios, single position with deterministic P&Ls:
        // pnl[s] = s as f64 / 100.0 - 0.5, so sorted ascending is
        // [-0.50, -0.49, ..., -0.46, -0.45, ...].
        let n_scenarios = 100;
        let n = 1;
        let mut pnls = Vec::with_capacity(n_scenarios * n);
        for s in 0..n_scenarios {
            pnls.push(s as f64 / 100.0 - 0.5);
        }

        let ids = [PositionId::new("X")];
        let config = DecompositionConfig::historical(0.95);

        let decomposer = HistoricalPositionDecomposer;
        let result = decomposer.decompose_from_pnls(&pnls, &ids, n_scenarios, &config)?;

        // n_tail = 5, var_idx = 4, sorted pnl[4] = 4/100 - 0.5 = -0.46.
        // Portfolio VaR = -(-0.46) = 0.46.
        assert!(
            (result.portfolio_var - 0.46).abs() < 1e-12,
            "portfolio_var = {}, expected 0.46 (boundary index 4)",
            result.portfolio_var
        );

        Ok(())
    }

    // C2 regression: reject configurations where the tail is too small
    // to resolve (e.g. 99% confidence with 50 scenarios: 0.01 * 50 = 0.5 < 1).
    #[test]
    fn historical_rejects_underspecified_tail() {
        let n_scenarios = 50;
        let n = 1;
        let pnls = vec![0.0; n_scenarios * n];
        let ids = [PositionId::new("X")];
        let config = DecompositionConfig::historical(0.99);

        let decomposer = HistoricalPositionDecomposer;
        let result = decomposer.decompose_from_pnls(&pnls, &ids, n_scenarios, &config);
        assert!(
            result.is_err(),
            "expected rejection when (1 - conf) * n_scenarios < 1"
        );
    }

    // C3/C4 regression: historical mode must report None for marginal
    // VaR, marginal ES, and euler_residual (none are meaningful in that
    // mode without additional inputs).
    #[test]
    fn historical_reports_none_for_marginals_and_residual() -> TestResult {
        let n = 2;
        let n_scenarios = 200;
        let mut pnls = Vec::with_capacity(n_scenarios * n);
        for s in 0..n_scenarios {
            pnls.push(-0.01 + 0.001 * (s as f64 / 10.0).sin());
            pnls.push(if s < 10 { -0.10 } else { 0.005 });
        }
        let ids = [PositionId::new("A"), PositionId::new("B")];
        let config = DecompositionConfig::historical(0.95);

        let decomposer = HistoricalPositionDecomposer;
        let result = decomposer.decompose_from_pnls(&pnls, &ids, n_scenarios, &config)?;

        assert!(
            result.euler_residual.is_none(),
            "historical euler_residual must be None"
        );
        for c in &result.var_contributions {
            assert!(
                c.marginal_var.is_none(),
                "historical marginal_var must be None for position {}",
                c.position_id
            );
        }
        for c in &result.es_contributions {
            assert!(
                c.marginal_es.is_none(),
                "historical marginal_es must be None for position {}",
                c.position_id
            );
        }

        Ok(())
    }

    // W1 regression: incremental VaR uses the textbook (non-renormalized)
    // definition, so for a long-only portfolio with positive-variance
    // positions the incremental VaR for each position must be non-negative
    // (removing a risky position cannot increase portfolio VaR).
    #[test]
    fn incremental_var_non_negative_for_long_only_portfolio() -> TestResult {
        let weights = [0.4, 0.35, 0.25];
        let covariance = [0.04, 0.01, 0.005, 0.01, 0.09, 0.02, 0.005, 0.02, 0.0625];
        let ids = [
            PositionId::new("A"),
            PositionId::new("B"),
            PositionId::new("C"),
        ];
        let config = DecompositionConfig::parametric_99().with_incremental();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        for c in &result.var_contributions {
            let ivar = c
                .incremental_var
                .expect("incremental VaR must be present when requested");
            assert!(
                ivar >= -1e-12,
                "long-only incremental VaR for {} must be non-negative, got {ivar}",
                c.position_id
            );
            // Textbook bound: incremental_k <= portfolio_var (the minimum
            // possible var_excl is zero, achieved only when the remaining
            // weights are perfectly hedged, which isn't true here).
            assert!(
                ivar <= result.portfolio_var + 1e-12,
                "incremental VaR for {} exceeds portfolio VaR: ivar={ivar}, pvar={}",
                c.position_id,
                result.portfolio_var
            );
        }

        Ok(())
    }

    // Parametric mode must report Some for marginals and residual.
    #[test]
    fn parametric_reports_some_for_marginals_and_residual() -> TestResult {
        let weights = [0.6, 0.4];
        let covariance = [0.04, 0.0, 0.0, 0.09];
        let ids = [PositionId::new("A"), PositionId::new("B")];
        let config = DecompositionConfig::parametric_95();

        let decomposer = ParametricPositionDecomposer;
        let result = decomposer.decompose_positions(&weights, &covariance, &ids, &config)?;

        assert!(
            result.euler_residual.is_some(),
            "parametric euler_residual must be Some"
        );
        for c in &result.var_contributions {
            assert!(
                c.marginal_var.is_some(),
                "parametric marginal_var must be Some for {}",
                c.position_id
            );
        }
        for c in &result.es_contributions {
            assert!(
                c.marginal_es.is_some(),
                "parametric marginal_es must be Some for {}",
                c.position_id
            );
        }

        Ok(())
    }
}
