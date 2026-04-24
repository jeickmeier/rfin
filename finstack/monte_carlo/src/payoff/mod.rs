//! Payoff definitions for Monte Carlo pricing.
//!
//! Start with [`vanilla`] for European call / put, digital, and forward-style
//! payoffs. This module also includes path-dependent payoffs such as Asian,
//! barrier, basket, and lookback contracts.
//!
//! All payoffs return [`finstack_core::money::Money`] for currency safety and
//! are evaluated on a mutable [`crate::traits::PathState`], which lets them
//! inspect named state variables and record path-level cashflows.

pub mod asian;
pub mod barrier;
pub mod basket;
pub mod lookback;
pub mod vanilla;

pub use basket::{margrabe_exchange_option, BasketCall, BasketPut, BasketType, ExchangeOption};
pub use vanilla::{Digital, EuropeanCall, EuropeanPut, Forward};
