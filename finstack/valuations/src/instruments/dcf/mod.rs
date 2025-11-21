//! Discounted Cash Flow (DCF) instruments for corporate valuation.
//!
//! DCF is the standard methodology for valuing companies based on projected
//! free cash flows. This module provides a first-class instrument for DCF
//! analysis, supporting both Gordon Growth and Exit Multiple approaches for
//! terminal value calculation.
//!
//! # DCF Structure
//!
//! - **Explicit Period**: Projected free cash flows (typically 3-10 years)
//! - **Terminal Value**: Perpetuity value using Gordon Growth or Exit Multiple
//! - **Enterprise Value**: PV(explicit flows) + PV(terminal value)
//! - **Equity Value**: Enterprise Value - Net Debt
//!
//! # Valuation Formula
//!
//! ```text
//! EV = Σ FCF_t / (1 + WACC)^t + TV / (1 + WACC)^n
//! Equity Value = EV - Net Debt
//! ```
//!
//! Where:
//! - FCF_t = Free Cash Flow in year t
//! - WACC = Weighted Average Cost of Capital
//! - TV = Terminal Value (Gordon Growth or Exit Multiple)
//! - n = Number of explicit forecast years
//!
//! # Terminal Value Methods
//!
//! ## Gordon Growth Model
//! ```text
//! TV = FCF_terminal × (1 + g) / (WACC - g)
//! ```
//!
//! ## Exit Multiple
//! ```text
//! TV = Terminal_Metric × Multiple
//! (e.g., EBITDA × 10x)
//! ```
//!
//! # Use Cases
//!
//! - M&A valuation and deal pricing
//! - LBO analysis and sponsor returns
//! - Fairness opinions
//! - Corporate strategy and capital allocation
//! - Public market comps (implied multiples)
//!
//! # Example
//!
//! ```rust
//! use finstack_valuations::instruments::dcf::{DiscountedCashFlow, TerminalValueSpec};
//! use finstack_core::types::InstrumentId;
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Project free cash flows
//! let flows = vec![
//!     (Date::from_calendar_date(2026, Month::January, 1)?, 100_000.0),
//!     (Date::from_calendar_date(2027, Month::January, 1)?, 110_000.0),
//!     (Date::from_calendar_date(2028, Month::January, 1)?, 120_000.0),
//! ];
//!
//! let dcf = DiscountedCashFlow::builder()
//!     .id(InstrumentId::new("ACME-DCF"))
//!     .currency(Currency::USD)
//!     .flows(flows)
//!     .wacc(0.10) // 10% WACC
//!     .terminal_value(TerminalValueSpec::GordonGrowth { growth_rate: 0.02 })
//!     .net_debt(500_000.0)
//!     .valuation_date(Date::from_calendar_date(2025, Month::January, 1)?)
//!     .attributes(Default::default())
//!     .build()?;
//!
//! // Equity value is returned by the value() method
//! # Ok(())
//! # }
//! ```

pub mod metrics;
pub mod pricer;
mod types;

pub use types::{DiscountedCashFlow, TerminalValueSpec};

