//! Forward Rate Agreement (FRA) instruments for forward interest rate contracts.
//!
//! FRAs are OTC contracts that lock in an interest rate for a future period.
//! They are fundamental instruments for hedging interest rate risk and
//! calibrating forward curves in the multi-curve framework.
//!
//! # FRA Structure
//!
//! - **Trade date**: When contract is entered
//! - **Fixing date**: When reference rate is observed
//! - **Start date**: Beginning of interest period
//! - **End date**: End of interest period
//! - **Fixed rate**: Agreed FRA rate
//! - **Notional**: Contract size
//!
//! # Pricing
//!
//! FRA payoff at settlement (discounted to start date):
//!
//! ```text
//! Payoff = N × τ × (Rate_realized - Rate_fixed) / (1 + Rate_realized × τ)
//! ```
//!
//! Present value from valuation date:
//!
//! ```text
//! PV = N × τ × (F - K) × DF(start)
//! ```
//!
//! where:
//! - N = notional
//! - τ = accrual fraction (day count)
//! - F = forward rate from curves
//! - K = FRA rate (fixed rate)
//! - DF(start) = discount factor to settlement
//!
//! # Market Conventions
//!
//! FRA naming: "3x6" means 3-month forward starting in 6 months
//!
//! Standard conventions by currency:
//! - **USD**: ACT/360, SOFR-based, T+2 settlement
//! - **EUR**: ACT/360, EURIBOR-based, T+2 settlement
//! - **GBP**: ACT/365, SONIA-based, T+0 settlement
//!
//! # Calibration Role
//!
//! FRAs are used to calibrate forward curves:
//! - Short forward rates (3M-2Y typically)
//! - Bridge between deposits and swaps
//! - Essential for multi-curve construction
//!
//! # See Also
//!
//! - [`ForwardRateAgreement`] for instrument struct
//! - [`calibration::methods::forward_curve`](crate::calibration::methods::forward_curve) for curve construction

pub mod metrics;
pub mod pricer;
mod types;

pub use types::ForwardRateAgreement;
