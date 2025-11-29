//! Stochastic risk metrics for structured credit.
//!
//! This module provides comprehensive risk metrics for stochastic
//! structured credit analysis including:
//!
//! - **Expected Loss (EL)**: Probability-weighted average loss
//! - **Unexpected Loss (UL)**: Loss standard deviation (Credit VaR proxy)
//! - **Expected Shortfall (ES)**: Tail risk metric (CVaR)
//! - **Correlation01**: Sensitivity to 1% asset correlation bump
//! - **RecoveryCorrelation01**: Sensitivity to recovery-default correlation
//! - **PrepaymentVol01**: Sensitivity to prepayment volatility
//!
//! # Risk Metric Definitions
//!
//! ## Expected Loss (EL)
//! ```text
//! EL = E[Loss] = Σᵢ pᵢ × Lossᵢ
//! ```
//!
//! ## Unexpected Loss (UL)
//! ```text
//! UL = σ(Loss) = √(E[Loss²] - E[Loss]²)
//! ```
//!
//! ## Expected Shortfall (ES) at confidence α
//! ```text
//! ES_α = E[Loss | Loss > VaR_α]
//! ```
//!
//! ## Correlation01
//! ```text
//! Corr01 = (NPV(ρ + 0.01) - NPV(ρ)) / 0.01
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let metrics = StochasticMetrics::compute(&tree, &config)?;
//!
//! println!("Expected Loss: ${:.0}", metrics.expected_loss);
//! println!("Unexpected Loss: ${:.0}", metrics.unexpected_loss);
//! println!("99% ES: ${:.0}", metrics.expected_shortfall_99);
//! println!("Corr01: ${:.2}/1%", metrics.correlation_01);
//! ```

mod calculator;
mod sensitivities;

pub use calculator::{StochasticMetrics, StochasticMetricsCalculator};
pub use sensitivities::{CorrelationSensitivities, SensitivityConfig};
