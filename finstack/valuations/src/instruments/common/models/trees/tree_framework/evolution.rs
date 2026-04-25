use finstack_core::HashMap;

use super::node_state::StateVariables;

/// Tree branching type for evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeBranching {
    /// Two-way branching (up/down)
    Binomial,
    /// Three-way branching (up/middle/down)
    Trinomial,
}

/// Generic tree parameters for state variable evolution
#[derive(Debug, Clone)]
pub struct TreeParameters {
    /// Number of time steps
    pub steps: usize,
    /// Time step size
    pub dt: f64,
    /// Tree branching type
    pub branching: TreeBranching,
    /// Evolution parameters for each state variable
    pub evolution_params: HashMap<&'static str, EvolutionParams>,
}

/// Parameters controlling how a state variable evolves in the tree
#[derive(Debug, Clone)]
pub struct EvolutionParams {
    /// Volatility for this factor
    pub volatility: f64,
    /// Drift rate (e.g., r-q for equity)
    pub drift: f64,
    /// Up factor
    pub up_factor: f64,
    /// Down factor
    pub down_factor: f64,
    /// Middle factor (for trinomial)
    pub middle_factor: Option<f64>,
    /// Probability of up move
    pub prob_up: f64,
    /// Probability of down move
    pub prob_down: f64,
    /// Probability of middle move (for trinomial)
    pub prob_middle: Option<f64>,
}

impl EvolutionParams {
    /// Create evolution parameters for a single equity factor (CRR model).
    ///
    /// # Errors
    ///
    /// Returns [`finstack_core::Error::Validation`] when the implied risk-neutral
    /// probability falls outside `[0, 1]` or any factor is non-positive (which
    /// happens for extreme combinations of `vol`, `drift`, `dt`). Release builds
    /// must enforce this — silent arbitrage in lattice probabilities is a
    /// production hazard.
    pub fn equity_crr(
        volatility: f64,
        risk_free_rate: f64,
        dividend_yield: f64,
        dt: f64,
    ) -> finstack_core::Result<Self> {
        let u = (volatility * dt.sqrt()).exp();
        let d = 1.0 / u;
        let drift = risk_free_rate - dividend_yield;
        let p = ((drift * dt).exp() - d) / (u - d);

        if !(0.0..=1.0).contains(&p) {
            return Err(finstack_core::Error::Validation(format!(
                "CRR probability p={} out of bounds [0,1] for vol={}, r={}, q={}, dt={}",
                p, volatility, risk_free_rate, dividend_yield, dt
            )));
        }
        if !(u > 0.0 && d > 0.0) {
            return Err(finstack_core::Error::Validation(format!(
                "CRR up/down factors must be positive: u={}, d={}",
                u, d
            )));
        }

        Ok(Self {
            volatility,
            drift,
            up_factor: u,
            down_factor: d,
            middle_factor: None,
            prob_up: p,
            prob_down: 1.0 - p,
            prob_middle: None,
        })
    }

    /// Create evolution parameters for trinomial tree.
    ///
    /// # Errors
    ///
    /// Returns [`finstack_core::Error::Validation`] when any of the three
    /// risk-neutral probabilities is negative or the probabilities fail to sum
    /// to one within `1e-10`. This catches arbitrage-violating parameter
    /// combinations (extreme drift/vol/dt) in release builds; debug-only
    /// `debug_assert!` was insufficient.
    pub fn equity_trinomial(
        volatility: f64,
        risk_free_rate: f64,
        dividend_yield: f64,
        dt: f64,
    ) -> finstack_core::Result<Self> {
        let u = (volatility * (2.0 * dt).sqrt()).exp();
        let d = 1.0 / u;
        let m = 1.0;

        let drift = risk_free_rate - dividend_yield;
        let sqrt_dt_half = (dt / 2.0).sqrt();
        let exp_drift_half = (drift * dt / 2.0).exp();

        let denominator = (volatility * sqrt_dt_half).exp() - (-volatility * sqrt_dt_half).exp();
        let p_u = ((exp_drift_half - (-volatility * sqrt_dt_half).exp()) / denominator).powi(2);
        let p_d = (((volatility * sqrt_dt_half).exp() - exp_drift_half) / denominator).powi(2);
        let p_m = 1.0 - p_u - p_d;

        if !(p_u >= 0.0 && p_d >= 0.0 && p_m >= 0.0) {
            return Err(finstack_core::Error::Validation(format!(
                "Trinomial probabilities must be non-negative: p_u={}, p_d={}, p_m={} \
                 (vol={}, r={}, q={}, dt={})",
                p_u, p_d, p_m, volatility, risk_free_rate, dividend_yield, dt
            )));
        }
        if (p_u + p_d + p_m - 1.0).abs() >= 1e-10 {
            return Err(finstack_core::Error::Validation(format!(
                "Trinomial probabilities must sum to 1: p_u + p_d + p_m = {}",
                p_u + p_d + p_m
            )));
        }

        Ok(Self {
            volatility,
            drift,
            up_factor: u,
            down_factor: d,
            middle_factor: Some(m),
            prob_up: p_u,
            prob_down: p_d,
            prob_middle: Some(p_m),
        })
    }
}

/// Barrier option configuration for discrete monitoring.
#[derive(Debug, Clone)]
pub enum BarrierStyle {
    /// Knock-out barrier: option becomes void upon breach (rebate may apply)
    KnockOut,
    /// Knock-in barrier: engine tracks barrier hit state for path-dependent pricing
    KnockIn,
}

/// Barrier specification for discrete barrier monitoring in tree pricing.
///
/// Defines barrier levels, rebate, and style for incorporating barrier
/// conditions into recombining tree valuation.
///
/// # Barrier Touch Convention
///
/// This implementation uses **non-strict inequality** for barrier observation:
/// - Up barrier: triggered when `spot >= up_level`
/// - Down barrier: triggered when `spot <= down_level`
///
/// This differs from QuantLib's default (strict inequality: `>` and `<`).
/// The non-strict convention is more conservative for knock-out options
/// (barrier is triggered at the exact level) and matches Bloomberg's behavior.
#[derive(Debug, Clone)]
pub struct BarrierSpec {
    /// Up barrier level (S >= up triggers a touch; non-strict inequality)
    pub up_level: Option<f64>,
    /// Down barrier level (S <= down triggers a touch; non-strict inequality)
    pub down_level: Option<f64>,
    /// Rebate amount paid on knock-out (or at expiry if knock-in never triggers)
    pub rebate: f64,
    /// Barrier style (engine only enforces KnockOut directly)
    pub style: BarrierStyle,
}

/// Custom state generator function type for flexible tree evolution.
///
/// Given a step index and node index, returns the state variable value at that node.
/// This allows for pre-calibrated trees (e.g., short-rate trees) to inject
/// custom state values instead of using multiplicative factors.
///
/// # Arguments
/// * `step` - Time step index (0 to N)
/// * `node` - Node index at this step
///
/// # Returns
/// * State variable value (e.g., interest rate, spot price)
pub type StateGenerator = Box<dyn Fn(usize, usize) -> f64>;
