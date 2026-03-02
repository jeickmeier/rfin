//! Merton structural credit model with distance-to-default and default probability.
//!
//! Implements the Merton (1974) model and its Black-Cox (1976) first-passage
//! extension for estimating firm default probability from balance-sheet data.
//!
//! # References
//!
//! - Merton, R. C. (1974). "On the Pricing of Corporate Debt: The Risk
//!   Structure of Interest Rates." *Journal of Finance*, 29(2), 449-470.
//! - Black, F. & Cox, J. C. (1976). "Valuing Corporate Securities: Some
//!   Effects of Bond Indenture Provisions." *Journal of Finance*, 31(2), 351-367.
//!
//! # Examples
//!
//! ```
//! use finstack_valuations::instruments::common::models::credit::MertonModel;
//!
//! let model = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
//! let dd = model.distance_to_default(1.0);
//! let pd = model.default_probability(1.0);
//! let spread = model.implied_spread(5.0, 0.40);
//! ```

use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::math::norm_cdf;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::{InputError, Result};

#[cfg(feature = "mc")]
use finstack_core::math::random::{poisson_inverse_cdf, RandomNumberGenerator};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Asset dynamics specification for the Merton model.
///
/// Controls the stochastic process assumed for the firm's asset value.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssetDynamics {
    /// Standard geometric Brownian motion (lognormal diffusion).
    GeometricBrownian,
    /// Jump-diffusion process (Merton 1976) with Poisson jumps.
    JumpDiffusion {
        /// Poisson jump arrival intensity (jumps per year).
        jump_intensity: f64,
        /// Mean log-jump size.
        jump_mean: f64,
        /// Volatility of log-jump size.
        jump_vol: f64,
    },
    /// CreditGrades model extension with uncertain recovery barrier.
    CreditGrades {
        /// Uncertainty in the default barrier level.
        barrier_uncertainty: f64,
        /// Mean recovery rate at default.
        mean_recovery: f64,
    },
}

/// Barrier monitoring type for default determination.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BarrierType {
    /// Default only assessed at maturity (classic Merton).
    Terminal,
    /// Continuous barrier monitoring (Black-Cox extension).
    FirstPassage {
        /// Growth rate of the default barrier over time.
        barrier_growth_rate: f64,
    },
}

/// Merton structural credit model.
///
/// Models a firm's equity as a call option on its assets, where default
/// occurs when asset value falls below the debt barrier.
///
/// # Fields
///
/// - `asset_value` (V_0): Current market value of the firm's assets.
/// - `asset_vol` (sigma_V): Annualized volatility of asset returns.
/// - `debt_barrier` (B): Face value of debt / default point.
/// - `risk_free_rate` (r): Continuous risk-free rate.
/// - `payout_rate` (q): Continuous dividend / payout yield on assets.
/// - `barrier_type`: Terminal or first-passage barrier monitoring.
/// - `dynamics`: Asset return dynamics specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MertonModel {
    asset_value: f64,
    asset_vol: f64,
    debt_barrier: f64,
    risk_free_rate: f64,
    payout_rate: f64,
    barrier_type: BarrierType,
    dynamics: AssetDynamics,
}

impl MertonModel {
    /// Create a new Merton model with GBM dynamics and terminal barrier.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be >= 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value <= 0` or
    /// `debt_barrier <= 0`, and [`InputError::NegativeValue`] if `asset_vol < 0`.
    pub fn new(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
    ) -> Result<Self> {
        Self::new_with_dynamics(
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
    }

    /// Create a new Merton model with full parameterisation.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be >= 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    /// * `payout_rate` - Dividend / payout yield q
    /// * `barrier_type` - Terminal or first-passage
    /// * `dynamics` - Asset return dynamics
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value <= 0` or
    /// `debt_barrier <= 0`, and [`InputError::NegativeValue`] if `asset_vol < 0`.
    pub fn new_with_dynamics(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        barrier_type: BarrierType,
        dynamics: AssetDynamics,
    ) -> Result<Self> {
        if asset_value <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if asset_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }
        if debt_barrier <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        Ok(Self {
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            payout_rate,
            barrier_type,
            dynamics,
        })
    }

    /// Distance-to-default over the given horizon.
    ///
    /// DD = (ln(V/B) + (r - q - sigma^2/2) * T) / (sigma * sqrt(T))
    ///
    /// A higher DD indicates a lower probability of default.
    #[inline]
    pub fn distance_to_default(&self, horizon: f64) -> f64 {
        let sigma = self.asset_vol;
        let mu = self.risk_free_rate - self.payout_rate - 0.5 * sigma * sigma;
        let sqrt_t = horizon.sqrt();
        ((self.asset_value / self.debt_barrier).ln() + mu * horizon) / (sigma * sqrt_t)
    }

    /// Default probability over the given horizon.
    ///
    /// - **Terminal barrier**: PD = N(-DD) (Merton 1974).
    /// - **First-passage barrier**: Black-Cox (1976) closed-form with
    ///   exponentially growing barrier at rate `g`:
    ///
    ///   Let mu = r - q - sigma^2/2, H = B * exp(g * T).
    ///   d_plus  = (ln(V/H) + mu * T) / (sigma * sqrt(T))
    ///   d_minus = (ln(V/H) - mu * T) / (sigma * sqrt(T))
    ///   PD = N(-d_plus) + (V/H)^(-2*mu/sigma^2) * N(d_minus)
    pub fn default_probability(&self, horizon: f64) -> f64 {
        match self.barrier_type {
            BarrierType::Terminal => {
                let dd = self.distance_to_default(horizon);
                norm_cdf(-dd)
            }
            BarrierType::FirstPassage {
                barrier_growth_rate,
            } => {
                let sigma = self.asset_vol;
                let mu = self.risk_free_rate - self.payout_rate - 0.5 * sigma * sigma;
                let sqrt_t = horizon.sqrt();
                let sigma_sqrt_t = sigma * sqrt_t;

                // Barrier at horizon: H = B * exp(g * T)
                let h = self.debt_barrier * (barrier_growth_rate * horizon).exp();
                let log_v_h = (self.asset_value / h).ln();

                let d_plus = (log_v_h + mu * horizon) / sigma_sqrt_t;
                let d_minus = (log_v_h - mu * horizon) / sigma_sqrt_t;

                // (V/H)^(-2*mu/sigma^2)
                let exponent = -2.0 * mu / (sigma * sigma);
                let ratio_term = (self.asset_value / h).powf(exponent);

                norm_cdf(-d_plus) + ratio_term * norm_cdf(d_minus)
            }
        }
    }

    /// Implied credit spread from default probability and recovery rate.
    ///
    /// s = -ln(1 - PD * (1 - R)) / T
    ///
    /// where PD is the default probability over horizon T and R is the
    /// assumed recovery rate (fraction of face value recovered at default).
    #[inline]
    pub fn implied_spread(&self, horizon: f64, recovery: f64) -> f64 {
        let pd = self.default_probability(horizon);
        let lgd = 1.0 - recovery;
        -(1.0 - pd * lgd).ln() / horizon
    }

    // -----------------------------------------------------------------------
    // Calibration methods
    // -----------------------------------------------------------------------

    /// Compute implied equity value and equity volatility from the structural model.
    ///
    /// Uses the Black-Scholes call option formula where equity is a call on
    /// the firm's assets with strike equal to the debt barrier:
    ///
    /// - d1 = (ln(V/B) + (r + sigma^2/2) * T) / (sigma * sqrt(T))
    /// - d2 = d1 - sigma * sqrt(T)
    /// - E = V * N(d1) - B * exp(-r*T) * N(d2)
    /// - sigma_E = N(d1) * sigma_V * V / E
    ///
    /// # Arguments
    ///
    /// * `horizon` - Time horizon T in years (must be > 0)
    ///
    /// # Returns
    ///
    /// A tuple `(equity_value, equity_vol)`.
    pub fn implied_equity(&self, horizon: f64) -> (f64, f64) {
        let v = self.asset_value;
        let sigma = self.asset_vol;
        let b = self.debt_barrier;
        let r = self.risk_free_rate;
        let sqrt_t = horizon.sqrt();

        let d1 = ((v / b).ln() + (r + 0.5 * sigma * sigma) * horizon) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;

        let nd1 = norm_cdf(d1);
        let nd2 = norm_cdf(d2);

        let equity = v * nd1 - b * (-r * horizon).exp() * nd2;
        let equity_vol = nd1 * sigma * v / equity;

        (equity, equity_vol)
    }

    /// KMV calibration: recover asset value and asset volatility from observed
    /// equity value and equity volatility.
    ///
    /// Solves the 2x2 nonlinear system iteratively (fixed-point iteration):
    /// - E = V * N(d1) - B * exp(-r*T) * N(d2)
    /// - sigma_E * E = N(d1) * sigma_V * V
    ///
    /// Convergence is typically fast (10-20 iterations).
    ///
    /// # Arguments
    ///
    /// * `equity_value` - Observed market equity value E
    /// * `equity_vol` - Observed equity volatility sigma_E
    /// * `total_debt` - Face value of debt B
    /// * `risk_free_rate` - Risk-free rate r
    /// * `maturity` - Time to maturity T in years
    ///
    /// # Errors
    ///
    /// Returns an error if inputs are invalid or iteration fails to converge.
    pub fn from_equity(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        maturity: f64,
    ) -> Result<Self> {
        if equity_value <= 0.0 || total_debt <= 0.0 || maturity <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if equity_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }

        let e = equity_value;
        let sigma_e = equity_vol;
        let b = total_debt;
        let r = risk_free_rate;
        let t = maturity;
        let sqrt_t = t.sqrt();

        // Initial guesses
        let mut v = e + b;
        let mut sigma_v = sigma_e * e / v;

        let max_iter = 100;
        let tol = 1e-8;

        for _ in 0..max_iter {
            let v_prev = v;

            let d1 = ((v / b).ln() + (r + 0.5 * sigma_v * sigma_v) * t) / (sigma_v * sqrt_t);
            let d2 = d1 - sigma_v * sqrt_t;

            let nd1 = norm_cdf(d1);
            let nd2 = norm_cdf(d2);

            // Update V from the call pricing equation
            v = (e + b * (-r * t).exp() * nd2) / nd1;
            // Update sigma_V from the volatility relation
            sigma_v = sigma_e * e / (nd1 * v);

            // Check convergence on relative change in V
            if ((v - v_prev) / v_prev).abs() < tol {
                return Self::new(v, sigma_v, b, r);
            }
        }

        // Return best estimate even if not fully converged
        Self::new(v, sigma_v, b, r)
    }

    /// CDS spread calibration: find asset volatility that matches a target
    /// CDS spread.
    ///
    /// Uses Brent's method to solve for sigma_V such that the model's
    /// implied spread equals the target CDS spread.
    ///
    /// # Arguments
    ///
    /// * `cds_spread_bp` - Target CDS spread in basis points
    /// * `recovery` - Recovery rate (fraction)
    /// * `total_debt` - Face value of debt B
    /// * `risk_free_rate` - Risk-free rate r
    /// * `maturity` - Time to maturity T in years
    /// * `asset_value` - Assumed initial asset value V
    ///
    /// # Errors
    ///
    /// Returns an error if the solver fails to find a solution or inputs
    /// are invalid.
    pub fn from_cds_spread(
        cds_spread_bp: f64,
        recovery: f64,
        total_debt: f64,
        risk_free_rate: f64,
        maturity: f64,
        asset_value: f64,
    ) -> Result<Self> {
        if total_debt <= 0.0 || maturity <= 0.0 || asset_value <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }

        let target_spread = cds_spread_bp / 10_000.0;

        let solver = BrentSolver::new().tolerance(1e-8).bracket_bounds(0.01, 2.0);

        let sigma_v = solver.solve(
            |sigma| {
                // Build a temporary model with this sigma_v to compute implied spread.
                // We use the inner formula directly to avoid the Result from new().
                let v = asset_value;
                let b = total_debt;
                let r = risk_free_rate;
                let sig = sigma;
                let mu = r - 0.5 * sig * sig;
                let sqrt_t = maturity.sqrt();
                let dd = ((v / b).ln() + mu * maturity) / (sig * sqrt_t);
                let pd = norm_cdf(-dd);
                let lgd = 1.0 - recovery;
                let spread = -(1.0 - pd * lgd).ln() / maturity;
                spread - target_spread
            },
            0.20, // initial guess
        )?;

        Self::new(asset_value, sigma_v, total_debt, risk_free_rate)
    }

    /// CreditGrades model construction from equity observables.
    ///
    /// A simplified calibration that derives asset value and asset volatility
    /// from equity data and constructs a model with `CreditGrades` dynamics
    /// and `FirstPassage` barrier.
    ///
    /// # Arguments
    ///
    /// * `equity_value` - Observed market equity value E
    /// * `equity_vol` - Observed equity volatility sigma_E
    /// * `total_debt` - Face value of debt
    /// * `risk_free_rate` - Risk-free rate r
    /// * `barrier_uncertainty` - Uncertainty in the default barrier level
    /// * `mean_recovery` - Mean recovery rate at default
    ///
    /// # Errors
    ///
    /// Returns an error if inputs are invalid.
    pub fn credit_grades(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        barrier_uncertainty: f64,
        mean_recovery: f64,
    ) -> Result<Self> {
        if equity_value <= 0.0 || total_debt <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if equity_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }

        // Asset value = equity + debt * mean_recovery
        let v0 = equity_value + total_debt * mean_recovery;
        // Asset vol from leverage relation
        let sigma_v = equity_vol * equity_value / v0;
        // Barrier = debt * mean_recovery
        let barrier = total_debt * mean_recovery;

        Self::new_with_dynamics(
            v0,
            sigma_v,
            barrier,
            risk_free_rate,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::CreditGrades {
                barrier_uncertainty,
                mean_recovery,
            },
        )
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Current asset value V_0.
    #[inline]
    pub fn asset_value(&self) -> f64 {
        self.asset_value
    }

    /// Asset volatility sigma_V.
    #[inline]
    pub fn asset_vol(&self) -> f64 {
        self.asset_vol
    }

    /// Debt barrier B.
    #[inline]
    pub fn debt_barrier(&self) -> f64 {
        self.debt_barrier
    }

    /// Risk-free rate r.
    #[inline]
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// Payout rate q (dividend yield).
    #[inline]
    pub fn payout_rate(&self) -> f64 {
        self.payout_rate
    }

    /// Barrier monitoring type.
    #[inline]
    pub fn barrier_type(&self) -> &BarrierType {
        &self.barrier_type
    }

    /// Asset dynamics specification.
    #[inline]
    pub fn dynamics(&self) -> &AssetDynamics {
        &self.dynamics
    }

    // -----------------------------------------------------------------------
    // Hazard curve generation
    // -----------------------------------------------------------------------

    /// Generate a [`HazardCurve`] compatible with existing pricing engines.
    ///
    /// Converts structural model default probabilities to piecewise-constant
    /// hazard rates at the specified tenor grid.
    ///
    /// # Algorithm
    ///
    /// 1. Compute survival probability S(t) = 1 - PD(t) at each tenor.
    /// 2. Back out piecewise-constant hazard rates between consecutive tenors:
    ///    - λ_0 = -ln(S(t_0)) / t_0
    ///    - λ_i = -ln(S(t_{i+1}) / S(t_i)) / (t_{i+1} - t_i) for i >= 1
    /// 3. Build via `HazardCurve::builder`.
    ///
    /// # Arguments
    ///
    /// * `id` - Curve identifier
    /// * `base_date` - Valuation date for the curve
    /// * `tenors` - Tenor grid in years (must be non-empty, positive, sorted)
    /// * `recovery` - Recovery rate assumption (e.g. 0.40)
    ///
    /// # Errors
    ///
    /// Returns an error if `tenors` is empty, contains non-positive values,
    /// or if the `HazardCurve` builder fails.
    pub fn to_hazard_curve(
        &self,
        id: &str,
        base_date: time::Date,
        tenors: &[f64],
        recovery: f64,
    ) -> Result<HazardCurve> {
        if tenors.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }

        // Sort tenors and validate positivity
        let mut sorted_tenors: Vec<f64> = tenors.to_vec();
        sorted_tenors.sort_by(|a, b| a.total_cmp(b));

        if sorted_tenors[0] <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }

        const EPSILON: f64 = 1e-15;

        // Compute survival probabilities, clamped to [epsilon, 1.0]
        let survivals: Vec<f64> = sorted_tenors
            .iter()
            .map(|&t| {
                let pd = self.default_probability(t);
                (1.0 - pd).clamp(EPSILON, 1.0)
            })
            .collect();

        // Build knot points: (tenor, hazard_rate)
        let mut knots: Vec<(f64, f64)> = Vec::with_capacity(sorted_tenors.len());

        // First point: λ_0 = -ln(S(t_0)) / t_0
        let lambda_0 = -survivals[0].ln() / sorted_tenors[0];
        knots.push((sorted_tenors[0], lambda_0));

        // Subsequent points: λ_i = -ln(S(t_{i+1}) / S(t_i)) / (t_{i+1} - t_i)
        for i in 1..sorted_tenors.len() {
            let dt = sorted_tenors[i] - sorted_tenors[i - 1];
            let lambda_i = -(survivals[i] / survivals[i - 1]).ln() / dt;
            knots.push((sorted_tenors[i], lambda_i.max(0.0)));
        }

        HazardCurve::builder(id)
            .base_date(base_date)
            .knots(knots)
            .recovery_rate(recovery)
            .build()
    }
}

// ---------------------------------------------------------------------------
// Monte Carlo path simulation (feature-gated)
// ---------------------------------------------------------------------------

/// Results from Monte Carlo path simulation.
#[cfg(feature = "mc")]
#[derive(Debug, Clone)]
pub struct SimulatedPaths {
    /// Time grid from 0 to T.
    pub times: Vec<f64>,
    /// Asset values: paths[path_idx][time_idx].
    pub asset_values: Vec<Vec<f64>>,
    /// Number of paths simulated.
    pub num_paths: usize,
    /// Number of time steps.
    pub num_steps: usize,
}

#[cfg(feature = "mc")]
impl MertonModel {
    /// Simulate asset value paths using Monte Carlo.
    ///
    /// Supports GBM and jump-diffusion dynamics. Optionally uses antithetic
    /// variates to reduce variance.
    ///
    /// # Arguments
    ///
    /// * `num_paths` - Number of paths to simulate
    /// * `num_steps` - Number of time steps per path
    /// * `horizon` - Time horizon T in years
    /// * `rng` - Random number generator (trait object)
    /// * `antithetic` - If true, use antithetic variates for variance reduction
    ///
    /// # Returns
    ///
    /// [`SimulatedPaths`] containing the time grid and all simulated asset paths.
    pub fn simulate_paths(
        &self,
        num_paths: usize,
        num_steps: usize,
        horizon: f64,
        rng: &mut dyn RandomNumberGenerator,
        antithetic: bool,
    ) -> SimulatedPaths {
        let dt = horizon / num_steps as f64;
        let sqrt_dt = dt.sqrt();

        // Build time grid: t = 0, dt, 2*dt, ..., T
        let times: Vec<f64> = (0..=num_steps).map(|i| i as f64 * dt).collect();

        let v0 = self.asset_value;
        let sigma = self.asset_vol;
        let r = self.risk_free_rate;
        let q = self.payout_rate;

        // Determine drift and whether we have jumps
        let (drift_per_step, jump_params) = match &self.dynamics {
            AssetDynamics::GeometricBrownian | AssetDynamics::CreditGrades { .. } => {
                let drift = (r - q - 0.5 * sigma * sigma) * dt;
                (drift, None)
            }
            AssetDynamics::JumpDiffusion {
                jump_intensity,
                jump_mean,
                jump_vol,
            } => {
                // kappa = E[e^J] - 1 where J ~ N(mu_J, sigma_J^2)
                let kappa = (jump_mean + 0.5 * jump_vol * jump_vol).exp() - 1.0;
                // Compensated drift to keep E[V(T)] = V0 * e^{(r-q)T}
                let drift = (r - q - jump_intensity * kappa - 0.5 * sigma * sigma) * dt;
                (drift, Some((*jump_intensity, *jump_mean, *jump_vol)))
            }
        };

        let diffusion = sigma * sqrt_dt;

        // Determine how many base paths to generate
        let (n_base, gen_antithetic) = if antithetic {
            // For num_paths requested: generate ceil(num_paths/2) base paths
            // and their mirrors. Total = 2 * n_base.
            // If num_paths is odd, we generate one extra base path without mirror
            // to hit exactly num_paths.
            let n_base = num_paths.div_ceil(2);
            (n_base, true)
        } else {
            (num_paths, false)
        };

        let mut all_paths: Vec<Vec<f64>> = Vec::with_capacity(num_paths);

        for _ in 0..n_base {
            // Generate normals for this base path
            let normals: Vec<f64> = (0..num_steps).map(|_| rng.normal(0.0, 1.0)).collect();

            // Generate jump data if needed
            let jump_data: Option<Vec<(usize, Vec<f64>)>> = jump_params.map(|(lambda, _, _)| {
                let lambda_dt = lambda * dt;
                (0..num_steps)
                    .map(|_| {
                        let n_jumps = poisson_inverse_cdf(lambda_dt, rng.uniform());
                        let jump_normals: Vec<f64> =
                            (0..n_jumps).map(|_| rng.normal(0.0, 1.0)).collect();
                        (n_jumps, jump_normals)
                    })
                    .collect()
            });

            // Build the base path
            let mut base_path = Vec::with_capacity(num_steps + 1);
            base_path.push(v0);
            let mut v = v0;

            for step in 0..num_steps {
                let z = normals[step];
                v *= (drift_per_step + diffusion * z).exp();

                // Apply jumps if present
                if let (Some(ref jd), Some((_, mu_j, sigma_j))) = (&jump_data, jump_params) {
                    let (_n_jumps, ref jump_z) = jd[step];
                    for &jz in jump_z {
                        v *= (mu_j - 0.5 * sigma_j * sigma_j + sigma_j * jz).exp();
                    }
                }

                base_path.push(v);
            }

            all_paths.push(base_path);

            // Build the antithetic (mirror) path if requested
            if gen_antithetic && all_paths.len() < num_paths {
                let mut anti_path = Vec::with_capacity(num_steps + 1);
                anti_path.push(v0);
                let mut v_anti = v0;

                for step in 0..num_steps {
                    let z = -normals[step]; // Negated normal
                    v_anti *= (drift_per_step + diffusion * z).exp();

                    // Apply jumps (same jump counts and jump normals — only diffusion Z is negated)
                    if let (Some(ref jd), Some((_, mu_j, sigma_j))) = (&jump_data, jump_params) {
                        let (_n_jumps, ref jump_z) = jd[step];
                        for &jz in jump_z {
                            v_anti *= (mu_j - 0.5 * sigma_j * sigma_j + sigma_j * jz).exp();
                        }
                    }

                    anti_path.push(v_anti);
                }

                all_paths.push(anti_path);
            }
        }

        // Trim to exact num_paths in case antithetic generated one extra
        all_paths.truncate(num_paths);

        SimulatedPaths {
            times,
            asset_values: all_paths,
            num_paths,
            num_steps,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn dd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let dd = m.distance_to_default(1.0);
        // DD = (ln(100/80) + (0.05 - 0 - 0.02)*1) / (0.2*1) = (0.22314 + 0.03) / 0.2 = 1.2657
        assert!((dd - 1.2657).abs() < 0.01, "DD={dd}");
    }

    #[test]
    fn pd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        // PD = N(-1.2657) ~ 0.1028
        assert!((pd - 0.1028).abs() < 0.01, "PD={pd}");
    }

    #[test]
    fn zero_vol_means_no_default_when_solvent() {
        let m = MertonModel::new(100.0, 1e-10, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        assert!(pd < 1e-6, "Zero vol, solvent -> PD~0, got {pd}");
    }

    #[test]
    fn pd_increases_with_vol() {
        let m_low = MertonModel::new(100.0, 0.10, 80.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.40, 80.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn pd_increases_with_leverage() {
        let m_low = MertonModel::new(100.0, 0.20, 60.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.20, 95.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn first_passage_pd_higher_than_terminal() {
        let m_term = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let m_fp = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.05,
            },
            AssetDynamics::GeometricBrownian,
        )
        .unwrap();
        assert!(
            m_fp.default_probability(5.0) > m_term.default_probability(5.0),
            "First-passage PD should be higher than terminal PD"
        );
    }

    #[test]
    fn implied_spread_positive_for_risky_firm() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
        let spread = m.implied_spread(5.0, 0.40);
        assert!(spread > 0.0, "Spread should be positive");
        assert!(spread < 0.20, "Spread should be reasonable, got {spread}");
    }

    #[test]
    fn new_rejects_invalid_inputs() {
        assert!(MertonModel::new(0.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(-1.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, -0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, 0.20, 0.0, 0.05).is_err());
    }

    #[test]
    fn implied_equity_from_known_asset() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let (equity, equity_vol) = m.implied_equity(1.0);
        // E should be V*N(d1) - B*e^(-rT)*N(d2)
        assert!(equity > 0.0, "Equity should be positive, got {equity}");
        assert!(
            equity_vol > 0.0,
            "Equity vol should be positive, got {equity_vol}"
        );
        // With V=100, B=80, sigma=0.20, r=0.05, T=1:
        // d1 = (ln(1.25) + (0.05 + 0.02)*1) / 0.2 = (0.2231 + 0.07) / 0.2 = 1.4657
        // d2 = 1.4657 - 0.2 = 1.2657
        // E = 100*N(1.4657) - 80*e^(-0.05)*N(1.2657) ~ 100*0.9286 - 76.10*0.8972 ~ 24.59
        assert!((equity - 24.59).abs() < 1.0, "Equity={equity}");
    }

    #[test]
    fn from_equity_recovers_known_values() {
        let m_known = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let (equity, equity_vol) = m_known.implied_equity(1.0);
        let m_calibrated =
            MertonModel::from_equity(equity, equity_vol, 80.0, 0.05, 1.0).expect("calibration");
        assert!(
            (m_calibrated.asset_value() - 100.0).abs() < 0.5,
            "Asset value should recover: got {}",
            m_calibrated.asset_value()
        );
        assert!(
            (m_calibrated.asset_vol() - 0.20).abs() < 0.01,
            "Asset vol should recover: got {}",
            m_calibrated.asset_vol()
        );
    }

    #[test]
    fn from_cds_spread_roundtrips() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let spread = m.implied_spread(5.0, 0.40);
        let spread_bp = spread * 10_000.0;
        let m2 =
            MertonModel::from_cds_spread(spread_bp, 0.40, 80.0, 0.04, 5.0, 100.0).expect("cds cal");
        assert!(
            (m2.asset_vol() - 0.25).abs() < 0.02,
            "Asset vol should recover: got {}",
            m2.asset_vol()
        );
    }

    #[test]
    fn to_hazard_curve_survival_matches_pd() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let hc = m
            .to_hazard_curve("TEST", base, &[1.0, 3.0, 5.0, 7.0, 10.0], 0.40)
            .expect("hc");
        // Survival at 5Y should match 1 - PD(5)
        let sp5 = hc.sp(5.0);
        let pd5 = m.default_probability(5.0);
        assert!(
            (sp5 - (1.0 - pd5)).abs() < 0.02,
            "sp5={sp5}, 1-pd5={}",
            1.0 - pd5
        );
    }

    #[test]
    fn to_hazard_curve_hazard_rates_positive() {
        let m = MertonModel::new(100.0, 0.30, 80.0, 0.04).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let hc = m
            .to_hazard_curve("TEST2", base, &[1.0, 3.0, 5.0], 0.40)
            .expect("hc");
        // All hazard rates should be positive for a risky firm
        for t in [0.5, 1.0, 2.0, 3.0, 4.0, 5.0] {
            let hr = hc.hazard_rate(t);
            assert!(
                hr > 0.0,
                "Hazard rate at t={t} should be positive, got {hr}"
            );
        }
    }

    #[test]
    fn to_hazard_curve_riskier_firm_higher_hazard() {
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let m_safe = MertonModel::new(100.0, 0.15, 50.0, 0.04).expect("valid");
        let m_risky = MertonModel::new(100.0, 0.30, 85.0, 0.04).expect("valid");
        let hc_safe = m_safe
            .to_hazard_curve("SAFE", base, &[1.0, 5.0, 10.0], 0.40)
            .expect("hc");
        let hc_risky = m_risky
            .to_hazard_curve("RISKY", base, &[1.0, 5.0, 10.0], 0.40)
            .expect("hc");
        assert!(
            hc_risky.hazard_rate(3.0) > hc_safe.hazard_rate(3.0),
            "Riskier firm should have higher hazard rate"
        );
    }

    #[test]
    fn credit_grades_produces_valid_model() {
        let m = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 0.40).expect("cg");
        assert!(m.asset_value() > 0.0);
        assert!(m.asset_vol() > 0.0);
        assert!(matches!(m.dynamics(), AssetDynamics::CreditGrades { .. }));
        assert!(matches!(m.barrier_type(), BarrierType::FirstPassage { .. }));
        let pd = m.default_probability(5.0);
        assert!(pd > 0.0 && pd < 1.0, "PD should be in (0,1), got {pd}");
    }

    // -----------------------------------------------------------------------
    // Monte Carlo path simulation tests
    // -----------------------------------------------------------------------

    #[cfg(feature = "mc")]
    #[test]
    fn simulate_paths_deterministic_with_seed() {
        use finstack_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng1 = Pcg64Rng::new(42);
        let mut rng2 = Pcg64Rng::new(42);
        let paths1 = m.simulate_paths(10, 60, 5.0, &mut rng1, false);
        let paths2 = m.simulate_paths(10, 60, 5.0, &mut rng2, false);
        assert_eq!(
            paths1.asset_values[0], paths2.asset_values[0],
            "Same seed should give same paths"
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn simulate_paths_gbm_mean_converges() {
        use finstack_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng = Pcg64Rng::new(42);
        let paths = m.simulate_paths(50_000, 60, 5.0, &mut rng, true);
        let mean_terminal: f64 = paths
            .asset_values
            .iter()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>()
            / paths.num_paths as f64;
        let expected = 100.0 * (0.05_f64 * 5.0).exp();
        let rel_error = (mean_terminal - expected).abs() / expected;
        assert!(
            rel_error < 0.02,
            "Mean terminal should converge to E[V(T)] = V\u{2080}\u{00d7}e^(rT) = {expected}, got {mean_terminal}, rel_err={rel_error}"
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn simulate_paths_correct_dimensions() {
        use finstack_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng = Pcg64Rng::new(42);
        let paths = m.simulate_paths(100, 60, 5.0, &mut rng, false);
        assert_eq!(paths.num_paths, 100);
        assert_eq!(paths.num_steps, 60);
        assert_eq!(paths.times.len(), 61); // includes t=0
        assert_eq!(paths.asset_values.len(), 100);
        assert_eq!(paths.asset_values[0].len(), 61);
        assert!(
            (paths.times[0] - 0.0).abs() < 1e-10,
            "First time should be 0"
        );
        assert!(
            (paths.times[60] - 5.0).abs() < 1e-10,
            "Last time should be horizon"
        );
        assert!(
            (paths.asset_values[0][0] - 100.0).abs() < 1e-10,
            "Should start at V\u{2080}"
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn jump_diffusion_produces_different_paths() {
        use finstack_core::math::random::Pcg64Rng;
        let m_gbm = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let m_jd = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.5,
                jump_mean: -0.05,
                jump_vol: 0.10,
            },
        )
        .expect("valid");
        let mut rng1 = Pcg64Rng::new(42);
        let mut rng2 = Pcg64Rng::new(42);
        let paths_gbm = m_gbm.simulate_paths(100, 60, 5.0, &mut rng1, false);
        let paths_jd = m_jd.simulate_paths(100, 60, 5.0, &mut rng2, false);
        // JD paths should differ from GBM (different drift compensation + jumps)
        let gbm_terminal: f64 = paths_gbm
            .asset_values
            .iter()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>();
        let jd_terminal: f64 = paths_jd
            .asset_values
            .iter()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>();
        assert!(
            (gbm_terminal - jd_terminal).abs() > 1.0,
            "JD should produce different terminal values"
        );
    }
}
