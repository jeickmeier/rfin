//! Analytical and semi-analytical pricing formulas for validation and production use.
//!
//! This module provides closed-form and semi-analytical pricing formulas
//! for various exotic options and derivatives. These formulas serve dual purposes:
//! 1. Validation of Monte Carlo implementations
//! 2. Production pricing when analytical methods are appropriate
//!
//! ## Methodology Sources
//!
//! - **Asian options**: Kemna & Vorst (1990) for geometric; Turnbull & Wakeman (1991) for arithmetic
//! - **Barrier options**: Reiner & Rubinstein (1991) "Breaking Down the Barriers"
//! - **Lookback options**: Conze & Viswanathan (1991); Haug (2007)
//! - **Quanto options**: Garman & Kohlhagen (1983); Brigo & Mercurio (2006)
//! - **Heston model**: Heston (1993); Carr & Madan (1999); Albrecher et al. (2007); Lord & Kahl (2010)

pub mod asian;
pub mod barrier;
pub mod greeks;
pub mod heston;
pub mod lookback;
pub mod quanto;

// Re-export commonly used functions
pub use asian::{
    arithmetic_asian_call_tw, arithmetic_asian_put_tw, geometric_asian_call, geometric_asian_put,
    AsianGreeks, AsianPriceResult,
};
pub use barrier::{
    barrier_call_continuous, barrier_put_continuous, down_in_call, down_out_call, up_in_call,
    up_out_call, BarrierType,
};
pub use greeks::{
    bs_call_delta, bs_call_greeks, bs_call_rho, bs_call_theta, bs_gamma, bs_put_delta,
    bs_put_greeks, bs_put_rho, bs_put_theta, bs_vega, CallGreeks, PutGreeks,
};
pub use heston::{heston_call_price_fourier, heston_put_price_fourier, HestonParams};
pub use lookback::{
    fixed_strike_lookback_call, fixed_strike_lookback_put, floating_strike_lookback_call,
    floating_strike_lookback_put,
};
pub use quanto::{
    quanto_call, quanto_call_simple, quanto_drift_adjustment, quanto_put, quanto_put_simple,
};

