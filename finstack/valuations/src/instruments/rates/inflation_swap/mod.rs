//! Zero-coupon inflation swap instruments for inflation exposure.
//!
//! Zero-coupon inflation swaps exchange a fixed rate (breakeven inflation)
//! for the realized cumulative inflation over the swap's life. They are
//! fundamental instruments for calibrating real (inflation-adjusted) curves.
//!
//! # Structure
//!
//! - **Fixed leg**: Pay or receive fixed breakeven rate
//! - **Inflation leg**: Pay or receive cumulative CPI growth
//! - **No interim payments**: Single payment at maturity
//!
//! # Payoff at Maturity
//!
//! ```text
//! Inflation leg = Notional × [CPI(T)/CPI(0) - 1]
//! Fixed leg = Notional × Breakeven_rate × T
//! ```
//!
//! Net payment = Inflation leg - Fixed leg (for receiver)
//!
//! # Breakeven Inflation Rate
//!
//! The fixed rate that makes PV = 0:
//!
//! ```text
//! Breakeven = [DF_nominal(T) / DF_real(T) - 1] / T
//! ```
//!
//! where DF_nominal and DF_real are nominal and real discount factors.
//!
//! # Inflation Indexation
//!
//! Standard conventions:
//! - **Index**: CPI-U (US), HICP ex-tobacco (EU), RPI or CPI (UK)
//! - **Lag**: Typically 3 months
//! - **Base CPI**: Inflation index at swap start (with lag)
//!
//! # Market Conventions
//!
//! - **USD**: CPI-U, 3-month lag, ACT/ACT
//! - **EUR**: HICP ex-tobacco, 3-month lag, ACT/ACT  
//! - **GBP**: RPI or CPI, 3-month lag, ACT/ACT
//!
//! # Calibration Role
//!
//! Zero-coupon inflation swaps calibrate real discount curves:
//! - 1Y to 30Y maturities typically quoted
//! - Real curve = Nominal curve / Inflation curve
//! - Used to price inflation-linked bonds
//!
//! # See Also
//!
//! - [`InflationSwap`] for instrument struct
//! - [`inflation_linked_bond`](super::inflation_linked_bond) for linkers
//! - [`calibration::methods::inflation_curve`](crate::calibration::methods::inflation_curve)

pub mod metrics;
/// Inflation swap pricer implementation
pub mod pricer;
mod types;

pub use pricer::SimpleInflationSwapDiscountingPricer;
pub use types::{
    InflationSwap, InflationSwapBuilder, PayReceiveInflation, YoYInflationSwap,
    YoYInflationSwapBuilder,
};
