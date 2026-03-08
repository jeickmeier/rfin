//! Cashflow primitives and present value calculations.
//!
//! This module provides foundational types and functions for cashflow analysis,
//! including present value (NPV), internal rate of return (IRR/XIRR), and
//! discounting operations commonly used in fixed income and derivatives pricing.
//!
//! # Components
//!
//! - **Primitives** (`primitives`): Core types (`CashFlow`, `Notional`, `CFKind`)
//! - **Discounting** (`discounting`): Present value calculation with discount curves
//! - **XIRR** (`xirr`): IRR/XIRR metrics for investment analysis

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
//! For NPV with Money-denominated cashflows, use `npv()` with a discount curve:
//!
//! ```rust
//! use finstack_core::cashflow::npv;
//! use finstack_core::dates::DayCount;
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use finstack_core::market_data::term_structures::FlatCurve;
//! use time::macros::date;
//!
//! let base = date!(2025 - 01 - 01);
//! let cf1 = (date!(2025 - 07 - 01), Money::new(1000.0, Currency::USD));
//! let cf2 = (date!(2026 - 01 - 01), Money::new(1000.0, Currency::USD));
//!
//! // Create a flat discount curve at 5% annual rate
//! let rate: f64 = 0.05;
//! let continuous_rate = (1.0 + rate).ln();
//! let curve = FlatCurve::new(continuous_rate, base, DayCount::Act365F, "EXAMPLE");
//!
//! let present_value = npv(&curve, base, Some(DayCount::Act365F), &[cf1, cf2])?;
//! assert!(present_value.amount() > 0.0);
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! For scalar cashflows, use `npv_amounts()`:
//!
//! ```rust
//! use finstack_core::cashflow::npv_amounts;
//! use finstack_core::dates::DayCount;
//! use time::macros::date;
//!
//! let base = date!(2025 - 01 - 01);
//! let flows = vec![
//!     (date!(2025 - 07 - 01), 1000.0),
//!     (date!(2026 - 01 - 01), 1000.0),
//! ];
//!
//! let present_value = npv_amounts(&flows, 0.05, Some(base), Some(DayCount::Act365F))?;
//! assert!(present_value > 0.0);
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! ## IRR Calculation
//!
//! ```rust
//! use finstack_core::cashflow::InternalRateOfReturn;
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

mod discounting;
mod primitives;
mod xirr;

pub use discounting::{npv, npv_amounts, npv_amounts_with_ctx, npv_with_ctx, Discountable};
pub use primitives::{CFKind, CashFlow};
pub use xirr::{xirr_with_daycount_ctx, InternalRateOfReturn};
