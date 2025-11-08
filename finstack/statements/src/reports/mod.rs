//! Convenience reporting for financial statements.
//!
//! This module provides human-friendly formatted reports for common statement
//! analysis tasks.
//!
//! # Features
//!
//! - **Table Formatting** - ASCII and Markdown table builders
//! - **Debt Reports** - Structure, service schedules, and maturity profiles
//! - **P&L Reports** - Summary views and credit assessments
//!
//! # Examples
//!
//! ```rust,ignore
//! use finstack_statements::reports::{Report, PLSummaryReport};
//!
//! let report = PLSummaryReport::new(&results, line_items, periods);
//! println!("{}", report.to_string());
//! ```

pub mod debt;
pub mod summary;
pub mod tables;

pub use debt::{print_debt_summary, DebtSummaryReport};
pub use summary::{CreditAssessmentReport, PLSummaryReport};
pub use tables::{Alignment, TableBuilder};

/// Core reporting trait.
///
/// Implement this trait to provide multiple output formats for a report.
pub trait Report {
    /// Convert report to string format.
    fn to_string(&self) -> String;

    /// Print report to stdout.
    fn print(&self) {
        println!("{}", self.to_string());
    }

    /// Convert report to Markdown format.
    fn to_markdown(&self) -> String {
        self.to_string() // Default implementation - subclasses can override
    }
}
