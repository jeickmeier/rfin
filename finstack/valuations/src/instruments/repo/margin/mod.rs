//! Repo margin and collateral management.
//!
//! This module provides GMRA 2011 compliant margin specification and
//! cashflow generation for repurchase agreements.
//!
//! # Overview
//!
//! Repo margining ensures that the cash provider remains adequately
//! collateralized throughout the life of the transaction. Key features:
//!
//! - Mark-to-market margin maintenance
//! - Haircut-based initial margin
//! - Margin call generation
//! - Collateral substitution support
//! - Margin interest accrual
//!
//! # GMRA 2011 Standards
//!
//! The Global Master Repurchase Agreement (GMRA 2011) provides the standard
//! documentation framework for repo transactions. This module implements:
//!
//! - Margin maintenance mechanics (Paragraph 4)
//! - Margin call deadlines and settlement
//! - Collateral substitution rules
//! - Close-out and netting provisions

mod cashflows;
mod spec;

pub use cashflows::*;
pub use spec::*;

