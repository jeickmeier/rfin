//! Interest rate swap instruments and pricing.
//!
//! Interest rate swaps (IRS) are OTC derivatives where two parties exchange
//! fixed and floating interest rate cashflows on a notional amount. This module
//! provides plain vanilla swaps, basis swaps, and par rate calculations.
//!
//! # Swap Structure
//!
//! A standard "payer" swap:
//! - **Fixed leg**: Pay fixed rate on notional
//! - **Float leg**: Receive floating rate (e.g., SOFR, EURIBOR)
//!
//! A "receiver" swap is the opposite (receive fixed, pay floating).
//!
//! # Pricing
//!
//! Under the risk-neutral measure, swap value is:
//!
//! ```text
//! PV_swap = PV_fixed_leg - PV_float_leg
//! ```
//!
//! For a payer swap:
//! ```text
//! PV = PV_float - PV_fixed
//!    = N · Σ τᵢ · Fwd(t_i) · DF(t_i) - N · K · Σ τᵢ · DF(t_i)
//! ```
//!
//! where:
//! - N = notional
//! - K = fixed rate
//! - Fwd(t_i) = forward rate for period i
//! - DF(t_i) = discount factor to payment date i
//! - τᵢ = accrual period (day count fraction)
//!
//! # Par Swap Rate
//!
//! The par rate is the fixed rate that makes PV_swap = 0:
//!
//! ```text
//! Par Rate = Σ τᵢ · Fwd(t_i) · DF(t_i) / Σ τᵢ · DF(t_i)
//!          = (DF(start) - DF(end)) / Annuity
//! ```
//!
//! # Market Conventions
//!
//! Standard conventions by currency:
//!
//! - **USD**: ACT/360 (float), 30/360 or ACT/360 (fixed), SOFR index
//! - **EUR**: ACT/360 (float), 30/360 (fixed), EURIBOR index
//! - **GBP**: ACT/365 (float), ACT/365 (fixed), SONIA index
//! - **JPY**: ACT/365 (float), ACT/365 (fixed), TONA index
//!
//! # Key Metrics
//!
//! - **Par Rate**: Market swap rate for zero initial value
//! - **DV01**: Dollar value of 1bp parallel shift in curve
//! - **Bucketed DV01**: Sensitivity to individual curve points
//! - **Annuity**: Present value of 1 unit paid each period
//!
//! # Examples
//!
//! See [`InterestRateSwap`] for construction and usage examples.
//!
//! # See Also
//!
//! - [`InterestRateSwap`] for the main swap struct
//! - [`FixedLegSpec`] for fixed leg specification
//! - [`FloatLegSpec`] for floating leg specification
//! - [`PayReceive`] for swap direction
//! - [`metrics`] for swap-specific risk metrics

pub mod metrics;
/// Interest rate swap pricer implementation
pub mod pricer;
mod types;

pub use types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, ParRateMethod, PayReceive};
