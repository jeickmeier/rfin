//! VaR backtesting: coverage tests, independence tests, and regulatory
//! traffic-light classification.
//!
//! Evaluates whether VaR forecasts (from [`crate::risk_metrics::value_at_risk`],
//! [`crate::risk_metrics::parametric_var`], or [`crate::risk_metrics::cornish_fisher_var`])
//! are statistically accurate by comparing predicted loss thresholds against
//! realized P&L.
//!
//! # Quick start
//!
//! ```rust
//! use finstack_analytics::backtesting::{run_backtest, VarBacktestConfig};
//!
//! let var_forecasts: Vec<f64> = vec![-0.02; 250];
//! let mut realized: Vec<f64> = vec![-0.01; 250];
//! realized[50] = -0.03;
//! realized[120] = -0.025;
//!
//! let result = run_backtest(
//!     &var_forecasts, &realized,
//!     &VarBacktestConfig::new().with_confidence(0.99),
//! );
//! ```

mod orchestrator;
mod tests;
mod types;

pub use orchestrator::{compare_var_backtests, rolling_var_forecasts, run_backtest};
pub use tests::{
    christoffersen_test, classify_breaches, kupiec_test, pnl_explanation, traffic_light,
};
pub use types::{
    BacktestResult, Breach, ChristoffersenResult, KupiecResult, MultiModelComparison,
    PnlExplanation, TrafficLightResult, TrafficLightZone, VarBacktestConfig, VarMethod,
};
