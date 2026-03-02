//! Structural credit models for default probability estimation.
//!
//! This module provides implementations of structural credit models,
//! starting with the classic Merton (1974) framework and its extensions
//! (Black-Cox first-passage, jump-diffusion, CreditGrades).
//!
//! # Module Organization
//!
//! - [`merton`]: Merton structural model with distance-to-default, default
//!   probability (terminal and first-passage), and implied credit spread.

pub mod merton;

pub use merton::{AssetDynamics, BarrierType, MertonModel};
