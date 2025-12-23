//! Non-Deliverable Forward (NDF) instrument module.
//!
//! This module provides the [`Ndf`] instrument for modeling non-deliverable
//! forward contracts on restricted currency pairs.
//!
//! # Overview
//!
//! NDFs are cash-settled forward contracts used in markets where foreign
//! exchange restrictions prevent physical delivery of the non-convertible
//! currency. Settlement occurs in a freely convertible currency (typically USD)
//! based on the difference between the contracted rate and a fixing rate.
//!
//! # Common NDF Markets
//!
//! - **CNY (Chinese Yuan)**: CNHFIX - RMB fixing by CFETS
//! - **INR (Indian Rupee)**: RBI reference rate
//! - **BRL (Brazilian Real)**: PTAX rate
//! - **KRW (Korean Won)**: KFTC fixing
//! - **TWD (Taiwan Dollar)**: TAIFX fixing
//!
//! # Pricing
//!
//! ## Pre-Fixing (before fixing date)
//! ```text
//! F_market = S × DF_foreign(T) / DF_settlement(T)  [or fallback when restricted]
//! PV = notional × (F_market - contract_rate) × DF_settlement(T)
//! ```
//!
//! ## Post-Fixing (after fixing date, before settlement)
//! ```text
//! PV = notional × (fixing_rate - contract_rate) × DF_settlement(T_settlement)
//! ```
//!
//! # Fixing Conventions
//!
//! - Fixing typically occurs T-2 before settlement
//! - Fixing source determines the official rate used
//! - Once fixed, the NDF becomes a simple cash flow
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::instruments::ndf::Ndf;
//! use finstack_core::currency::Currency;
//!
//! // Create a USD/CNY NDF
//! let ndf = Ndf::example();
//! assert_eq!(ndf.base_currency, Currency::CNY);
//! assert_eq!(ndf.settlement_currency, Currency::USD);
//! ```
//!
//! # See Also
//!
//! - [`FxForward`](super::fx_forward::FxForward) for deliverable forwards
//! - [`FxSwap`](super::fx_swap::FxSwap) for FX swap instruments

/// Pricer for NDF instruments.
pub mod pricer;
mod types;

pub use pricer::NdfDiscountingPricer;
pub use types::Ndf;

/// Metrics submodule for NDF risk measures.
pub mod metrics;
