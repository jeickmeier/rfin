//! Core primitives and financial types for the Python bindings.
//!
//! This module contains the fundamental building blocks including:
//! - Currency types
//! - Money (currency-safe amounts)
//! - Date and time handling
//! - Market data structures

pub mod currency;
pub mod dates;
pub mod market_data;
pub mod money;

// Re-export commonly used types at the core module level
