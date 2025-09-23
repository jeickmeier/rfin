//! Valuation and pricing functionality for the Python bindings.
//!
//! This module contains:
//! - Cashflow generation and analysis
//! - Financial instruments
//! - Pricing and risk metrics
//! - Attributes and tagging
//! - Covenant evaluation and management
//! - Workout and recovery management
//! - Policy implementations

pub mod attributes;
pub mod cashflow;
pub mod covenants;
pub mod instruments;
pub mod results;
// policy and workout bindings removed to simplify valuations surface

// Re-export commonly used types at the valuations module level

// Display implementations for types used across multiple modules
use crate::core::dates::PyDate;
use crate::core::money::PyMoney;
use std::fmt;

impl fmt::Display for PyDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner())
    }
}

impl fmt::Debug for PyDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PyDate({})", self.inner())
    }
}

impl fmt::Display for PyMoney {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner())
    }
}

impl fmt::Debug for PyMoney {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PyMoney({})", self.inner())
    }
}
