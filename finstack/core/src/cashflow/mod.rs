//! Cashflow primitives and present value calculations.
//!
//! This module provides foundational types and functions for cashflow analysis,
//! including present value (NPV), internal rate of return (IRR/XIRR), and
//! discounting operations commonly used in fixed income and derivatives pricing.
//!
//! # Components
//!
//! - **Primitives** ([`primitives`]): Core types ([`CashFlow`], [`Notional`], [`CFKind`])
//! - **Discounting** ([`discounting`]): Present value calculation with discount curves
//! - **XIRR** ([`xirr`]): IRR/XIRR metrics for investment analysis

//!
//! # Financial Concepts
//!
//! ## Net Present Value (NPV)
//!
//! The present value of a stream of future cashflows, discounted at an
//! appropriate rate:
//! ```text
//! NPV = Σ CF_i / (1 + r)^t_i
//! ```
//!
//! ## Internal Rate of Return (IRR)
//!
//! The discount rate that makes NPV = 0, representing the effective yield
//! of an investment:
//! ```text
//! 0 = Σ CF_i / (1 + IRR)^t_i
//! ```
//!
//! # Examples
//!
//! ## NPV Calculation
//!
//! ```rust
//! use finstack_core::cashflow::discounting::npv_constant;
//! use finstack_core::dates::{Date, DayCount};
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let cf1 = (Date::from_calendar_date(2025, Month::July, 1).unwrap(), 1000.0);
//! let cf2 = (Date::from_calendar_date(2026, Month::January, 1).unwrap(), 1000.0);
//!
//! let present_value = npv_constant(&[cf1, cf2], 0.05, base, DayCount::Act365F)?;
//! assert!(present_value > 0.0);
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! ## IRR Calculation
//!
//! ```rust
//! use finstack_core::cashflow::xirr::InternalRateOfReturn;
//!
//! // Initial investment followed by 4 quarterly returns (20% total return)
//! let cash_flows = vec![-10000.0, 3000.0, 3000.0, 3000.0, 3000.0];
//! let irr = cash_flows.irr(None)?;
//! assert!(irr > 0.0); // Positive return
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - **Textbooks**:
//!   - Brealey, R. A., Myers, S. C., & Allen, F. (2020). *Principles of Corporate Finance*
//!     (13th ed.). McGraw-Hill. Chapters 5-6 (NPV and IRR).
//!   - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!     Pearson. Chapter 4 (Interest Rates and Present Value).
//!
//! - **IRR Algorithm**:
//!   - Lin, S. A. (1976). "The Modified Internal Rate of Return and Investment Criterion."
//!     *The Engineering Economist*, 21(4), 237-247.

pub mod discounting;
pub mod primitives;
pub mod xirr;

pub use discounting::{npv, npv_constant, Discountable};
pub use primitives::{CFKind, CashFlow};
pub use xirr::InternalRateOfReturn;
