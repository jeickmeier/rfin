#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Cashflow schedule generation, aggregation, and accrual infrastructure.
//!
//! This crate provides the schedule-building and cashflow-analysis layer used
//! across the Finstack workspace for bonds, loans, swaps, and structured
//! products. The public API centers on [`cashflow::builder::CashFlowSchedule`]
//! and the finance-facing specification types that drive coupon, fee,
//! amortization, default, prepayment, and recovery behavior.
//!
//! # Entry Points
//!
//! - [`cashflow::builder`] for schedule construction and schedule-level PV
//!   helpers
//! - [`cashflow::aggregation`] for currency-preserving dated-flow aggregation
//! - [`cashflow::accrual`] for schedule-driven accrued-interest calculations
//! - [`cashflow::traits`] for [`cashflow::traits::CashflowProvider`] and
//!   [`cashflow::traits::schedule_from_dated_flows`]
//! - [`cashflow::primitives`] for `CashFlow`, `CFKind`, and other core cashflow
//!   primitives re-exported from `finstack-core`
//!
//! # Important Conventions
//!
//! - Amounts are represented with currency-tagged `Money`.
//! - Coupon rates are typically decimals, while spreads and fee quotes are often
//!   basis points.
//! - Day-count behavior is explicit and should be supplied by callers rather
//!   than inferred from examples.
//! - Payment and reset timing depend on business-day conventions, calendars, and
//!   lag settings carried on the schedule specs.
//!
//! See [`cashflow`] for the guided module overview and the main public
//! re-exports.

pub mod cashflow;
mod serde_defaults;

pub use cashflow::*;
