//! Core types for VaR backtesting: breach indicators, test results,
//! traffic-light zones, and configuration.

/// Whether a realized P&L breached the VaR forecast on a given day.
///
/// A breach (hit) occurs when the realized P&L is more negative than
/// the VaR forecast: `realized < var_forecast` (both are negative numbers
/// representing losses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Breach {
    Hit,
    Miss,
}

/// Identifies which VaR estimation method produced a forecast series.
///
/// Used by the multi-model comparison orchestrator to label results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VarMethod {
    Historical,
    Parametric,
    CornishFisher,
}

/// Basel Committee traffic-light classification for VaR model adequacy.
///
/// Based on the number of exceptions (breaches) observed in a 250-day
/// window at 99% confidence. The zone determines the capital multiplier
/// applied to the bank's market risk charge.
///
/// # References
///
/// - Basel Committee on Banking Supervision (1996):
///   see docs/REFERENCES.md#bcbs1996MarketRisk
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrafficLightZone {
    /// 0-4 exceptions. Model is adequate.
    Green,
    /// 5-9 exceptions. Model may have issues; supervisory review required.
    Yellow,
    /// 10+ exceptions. Model is inadequate; mandatory remediation.
    Red,
}

impl TrafficLightZone {
    /// Capital multiplier for the market risk charge.
    ///
    /// Green = 3.0, Yellow = 3.4-3.85 (linearly interpolated by exception
    /// count), Red = 4.0.
    #[must_use]
    pub fn capital_multiplier(&self, exceptions: usize) -> f64 {
        match self {
            Self::Green => 3.0,
            Self::Yellow => {
                // 5 -> 3.4, 6 -> 3.5, 7 -> 3.65, 8 -> 3.75, 9 -> 3.85
                let yellow_multipliers = [3.4, 3.5, 3.65, 3.75, 3.85];
                let idx = exceptions.saturating_sub(5).min(4);
                yellow_multipliers[idx]
            }
            Self::Red => 4.0,
        }
    }
}

/// Result of the Kupiec Proportion of Failures (POF) test.
///
/// Tests unconditional coverage: H0: p = alpha, where p is the true
/// breach probability and alpha is the VaR confidence tail probability.
///
/// # References
///
/// - Kupiec (1995): see docs/REFERENCES.md#kupiec1995VaRBacktest
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KupiecResult {
    /// Likelihood-ratio test statistic LR_uc. Asymptotically chi-squared(1).
    pub lr_statistic: f64,
    /// p-value from chi-squared(1) distribution. Reject H0 if p < significance.
    pub p_value: f64,
    /// Number of observed VaR breaches.
    pub breach_count: usize,
    /// Expected number of breaches under H0: alpha * T.
    pub expected_count: f64,
    /// Total number of observations.
    pub total_observations: usize,
    /// Observed breach rate: breach_count / total_observations.
    pub observed_rate: f64,
    /// Whether H0 is rejected at the 5% significance level.
    pub reject_h0_5pct: bool,
}

/// Result of the Christoffersen conditional coverage test.
///
/// Decomposes into:
/// 1. Unconditional coverage (same as Kupiec LR_uc)
/// 2. Independence: H0 = breaches are serially independent (no clustering)
/// 3. Joint (conditional coverage): LR_cc = LR_uc + LR_ind, chi-squared(2)
///
/// # References
///
/// - Christoffersen (1998): see docs/REFERENCES.md#christoffersen1998VaRBacktest
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChristoffersenResult {
    /// Unconditional coverage component (identical to Kupiec LR_uc).
    pub lr_uc: f64,
    /// Independence component LR_ind. chi-squared(1).
    pub lr_ind: f64,
    /// Joint conditional coverage statistic: LR_cc = LR_uc + LR_ind. chi-squared(2).
    pub lr_cc: f64,
    /// p-value for unconditional coverage test.
    pub p_value_uc: f64,
    /// p-value for independence test.
    pub p_value_ind: f64,
    /// p-value for joint conditional coverage test.
    pub p_value_cc: f64,
    /// Transition matrix counts: [n00, n01, n10, n11] where n_ij counts
    /// transitions from state i to state j (0 = miss, 1 = hit).
    pub transition_counts: [usize; 4],
    /// Whether H0 (joint) is rejected at the 5% significance level.
    pub reject_h0_5pct: bool,
}

/// Basel traffic-light assessment result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrafficLightResult {
    /// Assigned zone (Green, Yellow, Red).
    pub zone: TrafficLightZone,
    /// Number of exceptions in the evaluation window.
    pub exceptions: usize,
    /// Capital multiplier for the market risk charge.
    pub capital_multiplier: f64,
    /// Window size used (typically 250 trading days).
    pub window_size: usize,
    /// VaR confidence level used (typically 0.99).
    pub confidence: f64,
}

/// P&L explanation metrics for VaR model validation.
///
/// Compares different P&L concepts to assess how well the risk model
/// captures actual portfolio behavior. The key metric is the P&L
/// explanation ratio, which regulators use to assess model adequacy.
///
/// # P&L Concepts
///
/// - **Actual P&L**: realized daily P&L from the trading book
/// - **Hypothetical P&L**: P&L assuming positions are held constant
///   from the prior day (removes new trades and intraday activity)
/// - **Risk-theoretical P&L**: P&L predicted by the VaR model's risk
///   factors (what the model thinks should have happened)
///
/// # References
///
/// - Basel Committee FRTB (2019): see docs/REFERENCES.md#bcbs2019FRTB
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnlExplanation {
    /// Mean of (hypothetical_pnl - risk_theoretical_pnl) / VaR across days.
    pub explanation_ratio: f64,
    /// Mean absolute deviation of unexplained P&L:
    /// mean(|hypothetical_pnl - risk_theoretical_pnl|).
    pub mean_abs_unexplained: f64,
    /// Standard deviation of unexplained P&L.
    pub std_unexplained: f64,
    /// Number of observations used.
    pub n: usize,
}

/// Full backtest result aggregating all statistical tests and diagnostics.
///
/// Produced by the orchestrator's `run_backtest()` function.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BacktestResult {
    /// Kupiec unconditional coverage test.
    pub kupiec: KupiecResult,
    /// Christoffersen conditional coverage test.
    pub christoffersen: ChristoffersenResult,
    /// Basel traffic-light classification.
    pub traffic_light: TrafficLightResult,
    /// Time series of breach indicators aligned with the input series.
    pub breaches: Vec<Breach>,
    /// VaR confidence level used for the backtest.
    pub confidence: f64,
}

/// Multi-model backtest comparison.
///
/// Runs the same realized P&L through multiple VaR forecast series
/// and collects results side-by-side.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiModelComparison {
    pub results: Vec<(VarMethod, BacktestResult)>,
}

/// Configuration for the VaR backtest orchestrator.
///
/// Builder pattern allows customizing the evaluation window, confidence
/// level, and optional P&L explanation inputs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarBacktestConfig {
    /// VaR confidence level (e.g. 0.99 for 99% VaR). Default: 0.99.
    pub confidence: f64,
    /// Basel traffic-light window size. Default: 250 (trading days).
    pub window_size: usize,
    /// Significance level for hypothesis tests. Default: 0.05.
    pub significance: f64,
}

impl Default for VarBacktestConfig {
    fn default() -> Self {
        Self {
            confidence: 0.99,
            window_size: 250,
            significance: 0.05,
        }
    }
}

impl VarBacktestConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    #[must_use]
    pub fn with_window_size(mut self, window_size: usize) -> Self {
        self.window_size = window_size;
        self
    }

    #[must_use]
    pub fn with_significance(mut self, significance: f64) -> Self {
        self.significance = significance;
        self
    }
}
