//! Volatility conventions and conversion utilities.
//!
//! Provides volatility quoting conventions, pricing kernels, and ATM (strike = forward)
//! conversion utilities. Non-ATM conversions should rely on a surface that handles
//! strike/delta logic explicitly.

mod conventions;
mod convert;
mod pricing;

pub use conventions::VolatilityConvention;
pub use convert::convert_atm_volatility;
pub use pricing::{bachelier_price, black_price, black_shifted_price};
