//! Hull-White trinomial tree for Bermudan swaption pricing.
//!
//! Implements the industry-standard Hull-White 1-factor short rate model on a
//! recombining trinomial lattice. The tree is calibrated to the initial yield
//! curve via forward induction on the drift parameter α(t).
//!
//! # Model Dynamics
//!
//! The Hull-White model specifies the following short rate dynamics:
//!
//! ```text
//! dr(t) = [θ(t) - κr(t)]dt + σdW(t)
//! ```
//!
//! where:
//! - κ = mean reversion speed
//! - σ = short rate volatility
//! - θ(t) = time-dependent drift calibrated to fit initial yield curve
//!
//! # Tree Construction
//!
//! The tree uses a two-phase approach:
//! 1. Build tree in auxiliary x-space where x(t) = r(t) - α(t)
//! 2. Calibrate α(t) via forward induction to match discount curve
//!
//! The x-variable follows:
//! ```text
//! dx(t) = -κx(t)dt + σdW(t)
//! ```
//!
//! which has constant transition probabilities at each step.
//!
//! # References
//!
//! - Hull, J. & White, A. (1994). "Numerical Procedures for Implementing
//!   Term Structure Models I: Single-Factor Models", *Journal of Derivatives*
//! - Hull, J. (2018). *Options, Futures, and Other Derivatives*, 10th ed.
//!   Chapter 31: Interest Rate Derivatives: Models of the Short Rate

use crate::instruments::common_impl::validation;
use finstack_core::market_data::traits::Discounting;
use finstack_core::{Error, Result};

// ============================================================================
// Configuration
// ============================================================================

/// Hull-White 1-factor trinomial tree configuration.
///
/// # Parameter Guidelines
///
/// | Parameter | Typical Range | Description |
/// |-----------|---------------|-------------|
/// | kappa | 0.01-0.10 | Mean reversion (higher = faster reversion) |
/// | sigma | 0.005-0.015 | Normal volatility (50-150 bps) |
/// | steps | 50-200 | Tree steps (more = accuracy, O(n²) cost) |
#[derive(Clone, Debug)]
pub struct HullWhiteTreeConfig {
    /// Mean reversion speed (κ), annualized.
    ///
    /// Higher values cause rates to revert faster to the mean.
    /// Typical range: 0.01-0.10 (1-10% per year)
    pub kappa: f64,

    /// Short rate volatility (σ), annualized.
    ///
    /// This is normal/absolute volatility in rate units.
    /// Typical range: 0.005-0.015 (50-150 bps per year)
    pub sigma: f64,

    /// Number of time steps in the tree.
    ///
    /// More steps improve accuracy but increase computation time O(n²).
    /// Typical values: 50 (fast), 100 (standard), 200+ (high precision)
    pub steps: usize,

    /// Maximum number of nodes per step (limits tree width).
    ///
    /// For mean-reverting processes, the tree doesn't grow indefinitely.
    /// Default: 2 * steps + 1 (sufficient for most cases)
    pub max_nodes: Option<usize>,
}

impl Default for HullWhiteTreeConfig {
    fn default() -> Self {
        Self {
            kappa: 0.03, // 3% mean reversion
            sigma: 0.01, // 100 bps volatility
            steps: 100,
            max_nodes: None,
        }
    }
}

impl HullWhiteTreeConfig {
    /// Create a new configuration with specified parameters.
    pub fn new(kappa: f64, sigma: f64, steps: usize) -> Self {
        Self {
            kappa,
            sigma,
            steps,
            max_nodes: None,
        }
    }

    /// Set maximum nodes per step.
    pub fn with_max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = Some(max_nodes);
        self
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        validation::require_with(self.kappa > 0.0, || "kappa must be positive".into())?;
        validation::require_with(self.sigma > 0.0, || "sigma must be positive".into())?;
        validation::require_with(self.steps >= 2, || "steps must be at least 2".into())?;
        Ok(())
    }
}

// ============================================================================
// Hull-White Trinomial Tree
// ============================================================================

/// Calibrated Hull-White trinomial tree.
///
/// The tree is built and calibrated via [`HullWhiteTree::calibrate`]. After
/// calibration, it can compute bond prices, forward swap rates, and annuities
/// at any node.
///
/// # Node Indexing
///
/// At step `i`, there are `2*j_max + 1` nodes where `j_max = min(i, max_j)`.
/// Nodes are indexed from `0` to `2*j_max`, with the central node at index `j_max`.
///
/// The x-value at node (i, j) is: `x[i][j] = (j - j_max) * dx`
///
/// The short rate at node (i, j) is: `r[i][j] = x[i][j] + alpha[i]`
#[derive(Clone, Debug)]
pub struct HullWhiteTree {
    /// Configuration parameters
    config: HullWhiteTreeConfig,
    /// Time grid (year fractions from t=0)
    time_grid: Vec<f64>,
    /// Time step size
    dt: f64,
    /// x-space step size (dx = σ√(3dt))
    dx: f64,
    /// Maximum j index for x-nodes
    j_max: usize,
    /// α(t) drift parameter at each time step (calibrated to yield curve)
    alpha: Vec<f64>,
    /// Transition probabilities: (p_up, p_mid, p_down) for each step
    /// Indexed by step, then by node j
    probs: Vec<Vec<(f64, f64, f64)>>,
    /// Arrow-Debreu state prices Q(t,j) for verification
    state_prices: Vec<Vec<f64>>,
}

impl HullWhiteTree {
    /// Build and calibrate a Hull-White tree to match the discount curve.
    ///
    /// # Arguments
    ///
    /// * `config` - Tree configuration (κ, σ, steps)
    /// * `discount_curve` - Initial yield curve for calibration
    /// * `time_to_maturity` - Total tree horizon in years
    ///
    /// # Returns
    ///
    /// Calibrated tree ready for pricing
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::common::models::trees::HullWhiteTree;
    /// use finstack_valuations::instruments::common::models::trees::HullWhiteTreeConfig;
    ///
    /// let config = HullWhiteTreeConfig::default();
    /// # let discount_curve: &dyn finstack_core::market_data::traits::Discounting = todo!();
    /// let tree = HullWhiteTree::calibrate(config, discount_curve, 10.0)?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn calibrate(
        config: HullWhiteTreeConfig,
        discount_curve: &dyn Discounting,
        time_to_maturity: f64,
    ) -> Result<Self> {
        config.validate()?;

        let dt = time_to_maturity / config.steps as f64;
        // Standard trinomial spacing: dx = σ√(3dt) ensures variance matching
        let dx = config.sigma * (3.0 * dt).sqrt();

        // Maximum j value (tree width limit based on mean reversion)
        //
        // For mean-reverting processes, the tree width is bounded to prevent
        // probabilities from becoming negative or unstable at extreme nodes.
        //
        // The theoretical limit j_max is derived from the requirement that the
        // standard trinomial probabilities remain positive. For Hull-White:
        //
        //   p_up = 1/6 + (j²M² + jM)/2
        //   p_mid = 2/3 - j²M²
        //   p_down = 1/6 + (j²M² - jM)/2
        //
        // where M = κ·dt. For p_mid > 0, we need: j²M² < 2/3
        // Solving: |j| < √(2/3) / M = 0.816 / (κ·dt)
        //
        // The constant 0.184 ≈ 1 - 0.816 provides a conservative margin
        // to ensure probabilities remain well-behaved.
        //
        // Reference: Hull & White (1994), "Numerical Procedures for Implementing
        // Term Structure Models I: Single-Factor Models"
        let j_max_theoretical = (0.184 / (config.kappa * dt)).ceil() as usize;
        let j_max = config
            .max_nodes
            .map(|m| (m - 1) / 2)
            .unwrap_or(j_max_theoretical)
            .max(1);

        // Build time grid
        let time_grid: Vec<f64> = (0..=config.steps).map(|i| i as f64 * dt).collect();

        // Initialize alpha and state prices
        let mut alpha = vec![0.0; config.steps + 1];
        let mut state_prices = Vec::with_capacity(config.steps + 1);
        let mut probs = Vec::with_capacity(config.steps);

        // Initial state: single node at t=0 with Q(0,0) = 1
        state_prices.push(vec![1.0]);

        // Get initial short rate from discount curve
        let r0 = if dt > 0.0 {
            -discount_curve.df(dt).ln() / dt
        } else {
            0.03
        };
        alpha[0] = r0;

        // Forward induction: calibrate α(t) at each step
        for step in 0..config.steps {
            let _t = time_grid[step];
            let t_next = time_grid[step + 1];

            // Number of nodes at current and next step
            let curr_j_max = step.min(j_max);
            let next_j_max = (step + 1).min(j_max);

            // Compute transition probabilities for this step
            let mut step_probs = Vec::with_capacity(2 * curr_j_max + 1);
            for j in 0..=(2 * curr_j_max) {
                let j_signed = j as i32 - curr_j_max as i32;
                let p = Self::compute_probabilities(config.kappa, dt, dx, j_signed, curr_j_max)?;
                step_probs.push(p);
            }
            probs.push(step_probs);

            // Calibrate α(step+1) to match discount factor at t_next
            let target_df = discount_curve.df(t_next);
            alpha[step + 1] = Self::calibrate_alpha(
                &state_prices[step],
                &probs[step],
                dx,
                dt,
                curr_j_max,
                next_j_max,
                target_df,
            )?;

            // Compute state prices for next step
            let next_state_prices = Self::forward_state_prices(
                &state_prices[step],
                &probs[step],
                &alpha,
                step,
                dx,
                dt,
                curr_j_max,
                next_j_max,
            );
            state_prices.push(next_state_prices);
        }

        Ok(Self {
            config,
            time_grid,
            dt,
            dx,
            j_max,
            alpha,
            probs,
            state_prices,
        })
    }

    /// Compute trinomial transition probabilities for node j.
    ///
    /// For the Hull-White model with mean reversion κ:
    /// - p_up = 1/6 + (j²M² + jM)/2
    /// - p_mid = 2/3 - j²M²
    /// - p_down = 1/6 + (j²M² - jM)/2
    ///
    /// where M = κ·dt
    ///
    /// At boundaries (|j| >= j_max), we use drift-adjusted branching that:
    /// 1. Prevents the tree from growing beyond j_max
    /// 2. Accounts for mean reversion to maintain martingale property
    fn compute_probabilities(
        kappa: f64,
        dt: f64,
        dx: f64,
        j: i32,
        j_max: usize,
    ) -> finstack_core::Result<(f64, f64, f64)> {
        if !kappa.is_finite() || !dt.is_finite() || !dx.is_finite() || dt <= 0.0 || dx <= 0.0 {
            return Err(finstack_core::Error::Validation(
                "Hull-White probabilities require finite, positive inputs".to_string(),
            ));
        }

        let m = kappa * dt;
        let jf = j as f64;

        // Standard interior node probabilities (Hull-White trinomial)
        let mut p_up = 1.0 / 6.0 + (jf * jf * m * m + jf * m) / 2.0;
        let mut p_mid = 2.0 / 3.0 - jf * jf * m * m;
        let mut p_down = 1.0 / 6.0 + (jf * jf * m * m - jf * m) / 2.0;

        // At boundaries, use drift-adjusted probabilities that maintain martingale property
        // The mean reversion drift is: -κ * x = -κ * j * dx
        // We need probabilities that match the first two moments of the process
        let j_abs = j.unsigned_abs() as usize;
        if j_abs >= j_max && j_max > 0 {
            // x-value at this node
            let x_j = jf * dx;

            // Mean reversion drift: E[dx] = -κ * x * dt
            // We use modified branching at boundaries to keep within bounds
            // while respecting the drift
            let drift = -kappa * x_j * dt;

            if j > 0 {
                // Upper boundary: can only go down or stay (no up move)
                // Match first moment: p_mid * 0 + p_down * (-dx) = drift
                // Match second moment: p_mid * 0 + p_down * dx² = σ²dt
                // With constraint: p_mid + p_down = 1, p_up = 0

                // Simplified: p_down = -drift/dx + variance_term
                // where variance_term ensures second moment is matched
                let variance_term = 1.0 / 6.0; // Approximation for variance matching

                // Drift-adjusted probability (mean reversion pulls toward center)
                let p_down_adj = (-drift / dx + variance_term).clamp(0.0, 1.0);
                p_up = 0.0;
                p_down = p_down_adj.min(1.0);
                p_mid = 1.0 - p_down;
            } else if j < 0 {
                // Lower boundary: can only go up or stay (no down move)
                // Match first moment: p_up * dx + p_mid * 0 = drift
                // Note: drift is positive here (pulling up toward center)

                let variance_term = 1.0 / 6.0;

                // Drift-adjusted probability (mean reversion pulls toward center)
                let p_up_adj = (drift / dx + variance_term).clamp(0.0, 1.0);
                p_down = 0.0;
                p_up = p_up_adj.min(1.0);
                p_mid = 1.0 - p_up;
            }
        }

        // Ensure probabilities are valid (handle numerical edge cases)
        if p_up < 0.0
            || p_mid < 0.0
            || p_down < 0.0
            || !p_up.is_finite()
            || !p_mid.is_finite()
            || !p_down.is_finite()
        {
            return Err(finstack_core::Error::Validation(format!(
                "Hull-White probabilities invalid at j={j} (p_up={p_up}, p_mid={p_mid}, p_down={p_down})"
            )));
        }

        // Normalize to ensure sum = 1
        let sum = p_up + p_mid + p_down;
        if sum > 0.0 && sum.is_finite() {
            p_up /= sum;
            p_mid /= sum;
            p_down /= sum;
        } else {
            return Err(finstack_core::Error::Validation(
                "Hull-White probabilities did not sum to a finite value".to_string(),
            ));
        }

        // Final clamp for safety
        p_up = p_up.clamp(0.0, 1.0);
        p_mid = p_mid.clamp(0.0, 1.0);
        p_down = p_down.clamp(0.0, 1.0);

        // Debug assertion for development
        debug_assert!(
            (p_up + p_mid + p_down - 1.0).abs() < 1e-10,
            "Probabilities must sum to 1: p_up={}, p_mid={}, p_down={}",
            p_up,
            p_mid,
            p_down
        );

        Ok((p_up, p_mid, p_down))
    }

    /// Calibrate α at next step to match target discount factor.
    fn calibrate_alpha(
        curr_state_prices: &[f64],
        curr_probs: &[(f64, f64, f64)],
        dx: f64,
        dt: f64,
        curr_j_max: usize,
        _next_j_max: usize,
        target_df: f64,
    ) -> Result<f64> {
        // The sum Σ Q(t,j) * exp(-r(t,j)*dt) must equal P(0,t+dt)
        // where r(t,j) = x(t,j) + α(t)
        //
        // This gives: exp(-α*dt) * Σ Q(t,j) * exp(-x(t,j)*dt) * transition = P(0,t+dt)

        // Compute the weighted sum of discounted state prices
        let mut weighted_sum = 0.0;

        for j in 0..curr_state_prices.len() {
            let j_signed = j as i32 - curr_j_max as i32;
            let x_j = j_signed as f64 * dx;
            let (p_up, p_mid, p_down) = curr_probs[j];

            // Contribution from this node (before α adjustment)
            let base_discount = (-x_j * dt).exp();
            let contribution = curr_state_prices[j] * base_discount * (p_up + p_mid + p_down);
            weighted_sum += contribution;
        }

        if weighted_sum <= 0.0 {
            return Err(Error::Validation(
                "Invalid state prices in tree calibration".into(),
            ));
        }

        // Solve for α: exp(-α*dt) * weighted_sum = target_df
        let alpha = -(target_df / weighted_sum).ln() / dt;

        Ok(alpha)
    }

    /// Compute state prices for next time step.
    #[allow(clippy::too_many_arguments)]
    fn forward_state_prices(
        curr_state_prices: &[f64],
        curr_probs: &[(f64, f64, f64)],
        alpha: &[f64],
        step: usize,
        dx: f64,
        dt: f64,
        curr_j_max: usize,
        next_j_max: usize,
    ) -> Vec<f64> {
        let num_next_nodes = 2 * next_j_max + 1;
        let mut next_prices = vec![0.0; num_next_nodes];

        for j in 0..curr_state_prices.len() {
            let j_signed = j as i32 - curr_j_max as i32;
            let x_j = j_signed as f64 * dx;
            let r_j = x_j + alpha[step];

            let (p_up, p_mid, p_down) = curr_probs[j];
            let discount = (-r_j * dt).exp();
            let q_contribution = curr_state_prices[j] * discount;

            // Map to next step indices
            let next_mid = (j_signed + next_j_max as i32) as usize;

            // Up transition (j -> j+1)
            if next_mid + 1 < num_next_nodes {
                next_prices[next_mid + 1] += q_contribution * p_up;
            }
            // Mid transition (j -> j)
            if next_mid < num_next_nodes {
                next_prices[next_mid] += q_contribution * p_mid;
            }
            // Down transition (j -> j-1)
            if next_mid > 0 {
                next_prices[next_mid - 1] += q_contribution * p_down;
            }
        }

        next_prices
    }

    // ========================================================================
    // Accessor Methods
    // ========================================================================

    /// Get configuration.
    pub fn config(&self) -> &HullWhiteTreeConfig {
        &self.config
    }

    /// Get number of time steps.
    pub fn num_steps(&self) -> usize {
        self.config.steps
    }

    /// Get time at step i.
    pub fn time_at_step(&self, step: usize) -> f64 {
        self.time_grid.get(step).copied().unwrap_or(0.0)
    }

    /// Get time step size.
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// Get number of nodes at a given step.
    pub fn num_nodes(&self, step: usize) -> usize {
        let j_max = step.min(self.j_max);
        2 * j_max + 1
    }

    /// Get short rate r(t,j) at node (step, node_idx).
    pub fn rate_at_node(&self, step: usize, node_idx: usize) -> f64 {
        let j_max = step.min(self.j_max);
        let j_signed = node_idx as i32 - j_max as i32;
        let x_j = j_signed as f64 * self.dx;
        x_j + self.alpha.get(step).copied().unwrap_or(0.0)
    }

    /// Get transition probabilities at node (step, node_idx).
    ///
    /// Returns (p_up, p_mid, p_down).
    pub fn probabilities(&self, step: usize, node_idx: usize) -> (f64, f64, f64) {
        self.probs
            .get(step)
            .and_then(|p| p.get(node_idx))
            .copied()
            .unwrap_or((1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0))
    }

    /// Get state price Q(t,j) at node (step, node_idx).
    pub fn state_price(&self, step: usize, node_idx: usize) -> f64 {
        self.state_prices
            .get(step)
            .and_then(|p| p.get(node_idx))
            .copied()
            .unwrap_or(0.0)
    }

    // ========================================================================
    // Bond Price Calculations
    // ========================================================================

    /// Compute zero-coupon bond price P(t, T) at node (step, node_idx).
    ///
    /// Uses the Hull-White analytical formula:
    /// ```text
    /// P(t, T) = A(t, T) * exp(-B(t, T) * r(t))
    /// ```
    ///
    /// where:
    /// - B(t, T) = (1 - exp(-κ(T-t))) / κ
    /// - A(t, T) = P(0,T)/P(0,t) * exp(B(t,T)*f(0,t) - σ²(1-e^(-2κt))B²/(4κ))
    ///
    /// # Arguments
    ///
    /// * `step` - Current time step
    /// * `node_idx` - Node index at current step
    /// * `maturity_time` - Bond maturity time T (year fraction from t=0)
    /// * `discount_curve` - Initial yield curve for A(t,T) calculation
    pub fn bond_price(
        &self,
        step: usize,
        node_idx: usize,
        maturity_time: f64,
        discount_curve: &dyn Discounting,
    ) -> f64 {
        let t = self.time_at_step(step);
        let tau = maturity_time - t;

        if tau <= 0.0 {
            return 1.0;
        }

        let r = self.rate_at_node(step, node_idx);
        let kappa = self.config.kappa;
        let sigma = self.config.sigma;

        // B(t, T) factor
        let b = if kappa.abs() < 1e-10 {
            tau // Limit as κ → 0
        } else {
            (1.0 - (-kappa * tau).exp()) / kappa
        };

        // A(t, T) factor using market discount factors
        let p_0_t = discount_curve.df(t);
        let p_0_tt = discount_curve.df(maturity_time);

        if p_0_t <= 0.0 {
            return 0.0;
        }

        // Forward rate at t=0 for maturity t
        let f_0_t = if t > 0.0 {
            discount_curve
                .instantaneous_forward(t)
                .unwrap_or_else(|_| -p_0_t.ln() / t)
        } else {
            self.alpha[0]
        };

        // Variance term
        let var_term = if kappa.abs() < 1e-10 {
            sigma * sigma * t * b * b / 2.0
        } else {
            sigma * sigma * (1.0 - (-2.0 * kappa * t).exp()) * b * b / (4.0 * kappa)
        };

        let ln_a = (p_0_tt / p_0_t).ln() + b * f_0_t - var_term;
        let a = ln_a.exp();

        a * (-b * r).exp()
    }

    /// Compute forward swap rate S(t) at node (step, node_idx).
    ///
    /// The forward swap rate is computed as:
    /// ```text
    /// S(t) = [P(t, T_start) - P(t, T_end)] / A(t)
    /// ```
    ///
    /// where A(t) = Σᵢ τᵢ P(t, Tᵢ) is the annuity.
    ///
    /// # Arguments
    ///
    /// * `step` - Current time step
    /// * `node_idx` - Node index
    /// * `swap_start_time` - Swap start time (year fraction)
    /// * `swap_end_time` - Swap end time (year fraction)
    /// * `payment_times` - Payment date times (year fractions)
    /// * `accrual_fractions` - Accrual fractions for each period
    /// * `discount_curve` - Initial yield curve
    #[allow(clippy::too_many_arguments)]
    pub fn forward_swap_rate(
        &self,
        step: usize,
        node_idx: usize,
        swap_start_time: f64,
        swap_end_time: f64,
        payment_times: &[f64],
        accrual_fractions: &[f64],
        discount_curve: &dyn Discounting,
    ) -> f64 {
        let t = self.time_at_step(step);

        // Filter to remaining payments
        let remaining: Vec<_> = payment_times
            .iter()
            .zip(accrual_fractions.iter())
            .filter(|(&pay_t, _)| pay_t > t)
            .collect();

        if remaining.is_empty() {
            return 0.0;
        }

        // Start discount factor (or 1.0 if already started)
        let p_start = if swap_start_time > t {
            self.bond_price(step, node_idx, swap_start_time, discount_curve)
        } else {
            1.0
        };

        // End discount factor
        let p_end = self.bond_price(step, node_idx, swap_end_time, discount_curve);

        // Annuity
        let annuity = self.annuity(
            step,
            node_idx,
            payment_times,
            accrual_fractions,
            discount_curve,
        );

        if annuity.abs() < 1e-12 {
            return 0.0;
        }

        (p_start - p_end) / annuity
    }

    /// Compute annuity (PV01) at node (step, node_idx).
    ///
    /// ```text
    /// A(t) = Σᵢ τᵢ P(t, Tᵢ)
    /// ```
    ///
    /// Only includes payments occurring after time t.
    pub fn annuity(
        &self,
        step: usize,
        node_idx: usize,
        payment_times: &[f64],
        accrual_fractions: &[f64],
        discount_curve: &dyn Discounting,
    ) -> f64 {
        let t = self.time_at_step(step);

        payment_times
            .iter()
            .zip(accrual_fractions.iter())
            .filter(|(&pay_t, _)| pay_t > t)
            .map(|(&pay_t, &tau)| tau * self.bond_price(step, node_idx, pay_t, discount_curve))
            .sum()
    }

    // ========================================================================
    // Backward Induction
    // ========================================================================

    /// Price an instrument using backward induction.
    ///
    /// # Arguments
    ///
    /// * `terminal_values` - Payoff values at maturity for each node
    /// * `intermediate_value_fn` - Called at each step to adjust for early exercise/coupons
    ///
    /// The `intermediate_value_fn` takes (step, node_idx, continuation_value) and returns
    /// the adjusted value (e.g., max(continuation, exercise_value) for Bermudan options).
    pub fn backward_induction<F>(&self, terminal_values: &[f64], intermediate_value_fn: F) -> f64
    where
        F: Fn(usize, usize, f64) -> f64,
    {
        let n = self.config.steps;

        // Start with terminal values
        let mut values = terminal_values.to_vec();

        // Backward induction
        for step in (0..n).rev() {
            let num_nodes = self.num_nodes(step);
            let num_next_nodes = self.num_nodes(step + 1);
            let j_max_curr = step.min(self.j_max);
            let j_max_next = (step + 1).min(self.j_max);

            let mut new_values = vec![0.0; num_nodes];

            for (j, new_value) in new_values.iter_mut().enumerate() {
                let r_j = self.rate_at_node(step, j);
                let (p_up, p_mid, p_down) = self.probabilities(step, j);

                // Discounted expected value from child nodes
                let j_signed = j as i32 - j_max_curr as i32;
                let next_mid = (j_signed + j_max_next as i32) as usize;

                let v_up = if next_mid + 1 < num_next_nodes {
                    values[next_mid + 1]
                } else {
                    values[num_next_nodes - 1]
                };

                let v_mid = if next_mid < num_next_nodes {
                    values[next_mid]
                } else {
                    values[num_next_nodes - 1]
                };

                let v_down = if next_mid > 0 {
                    values[next_mid - 1]
                } else {
                    values[0]
                };

                let expected_value = p_up * v_up + p_mid * v_mid + p_down * v_down;
                let discounted = expected_value * (-r_j * self.dt).exp();

                // Apply intermediate value function (exercise decision, coupons, etc.)
                *new_value = intermediate_value_fn(step, j, discounted);
            }

            values = new_values;
        }

        // Return value at root node
        values.first().copied().unwrap_or(0.0)
    }

    /// Map a time (year fraction) to the nearest tree step.
    pub fn time_to_step(&self, time: f64) -> usize {
        if time <= 0.0 {
            return 0;
        }
        let step = (time / self.dt).round() as usize;
        step.min(self.config.steps)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1)
                    .expect("Valid date"),
            )
            .knots([
                (0.0, 1.0),
                (0.5, 0.985),
                (1.0, 0.97),
                (2.0, 0.94),
                (5.0, 0.85),
                (10.0, 0.70),
            ])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Valid curve")
    }

    #[test]
    fn test_tree_calibration() {
        // Use 200 steps for production-quality < 1 bp calibration
        let config = HullWhiteTreeConfig::new(0.03, 0.01, 200);
        let curve = test_discount_curve();

        let tree =
            HullWhiteTree::calibrate(config, &curve, 5.0).expect("Calibration should succeed");

        // Tree should have correct number of steps
        assert_eq!(tree.num_steps(), 200);

        // State prices should sum to discount factors
        // Production standard: calibration error < 1 basis point (0.0001)
        for step in [20, 50, 100, 150, 200] {
            let t = tree.time_at_step(step);
            let target_df = curve.df(t);
            let sum_q: f64 = (0..tree.num_nodes(step))
                .map(|j| tree.state_price(step, j))
                .sum();

            let error = (sum_q - target_df).abs();
            let error_bps = (error / target_df) * 10000.0;

            // Production tolerance: < 1 basis point
            assert!(
                error_bps < 1.0,
                "State price calibration error {:.6} ({:.4} bps) at step {} (t={:.2})",
                error,
                error_bps,
                step,
                t
            );
        }
    }

    #[test]
    fn test_bond_price_at_maturity() {
        // Use 200 steps for production-quality < 1 bp accuracy
        let config = HullWhiteTreeConfig::new(0.03, 0.01, 200);
        let curve = test_discount_curve();

        let tree =
            HullWhiteTree::calibrate(config, &curve, 2.0).expect("Calibration should succeed");
        let final_step = tree.num_steps();
        let mid_node = tree.num_nodes(final_step) / 2;

        // Bond price at maturity should be exactly 1.0
        // Production standard: < 1 bp error
        let bp = tree.bond_price(final_step, mid_node, 2.0, &curve);
        let error_bps = (bp - 1.0).abs() * 10000.0;
        assert!(
            error_bps < 1.0,
            "Bond price at maturity should be 1.0, got {:.8} (error: {:.4} bps)",
            bp,
            error_bps
        );
    }

    #[test]
    fn probabilities_fail_fast_when_invalid() {
        let err =
            HullWhiteTree::compute_probabilities(0.03, 0.25, 0.0, 1, 1).expect_err("should fail");
        match err {
            finstack_core::Error::Validation(msg) => {
                assert!(msg.contains("finite"), "message={msg}");
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn test_backward_induction_zero_payoff() {
        let config = HullWhiteTreeConfig::new(0.03, 0.01, 10);
        let curve = test_discount_curve();

        let tree =
            HullWhiteTree::calibrate(config, &curve, 1.0).expect("Calibration should succeed");

        // Zero payoff should give zero value
        let terminal = vec![0.0; tree.num_nodes(10)];
        let value = tree.backward_induction(&terminal, |_, _, cont| cont);

        assert!(value.abs() < 1e-10, "Zero payoff should give zero value");
    }

    #[test]
    fn test_backward_induction_unit_payoff() {
        // Use 200 steps for production-quality < 1 bp backward induction
        let config = HullWhiteTreeConfig::new(0.03, 0.01, 200);
        let curve = test_discount_curve();

        let tree =
            HullWhiteTree::calibrate(config, &curve, 1.0).expect("Calibration should succeed");
        let final_step = tree.num_steps();

        // Unit payoff at all nodes should give approximately the discount factor
        let terminal = vec![1.0; tree.num_nodes(final_step)];
        let value = tree.backward_induction(&terminal, |_, _, cont| cont);

        let target_df = curve.df(1.0);
        let error = (value - target_df).abs();
        let error_bps = (error / target_df) * 10000.0;

        // Production standard: pricing error < 1 basis point
        assert!(
            error_bps < 1.0,
            "Unit payoff value {:.8} should match df {:.8} (error: {:.4} bps)",
            value,
            target_df,
            error_bps
        );
    }
}
