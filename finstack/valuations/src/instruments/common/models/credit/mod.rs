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

pub mod dynamic_recovery;
pub mod endogenous_hazard;
pub mod merton;
pub mod toggle_exercise;

pub use dynamic_recovery::DynamicRecoverySpec;
pub use endogenous_hazard::EndogenousHazardSpec;
pub use merton::{AssetDynamics, BarrierType, MertonModel};
pub use toggle_exercise::{CreditState, CreditStateVariable, ToggleExerciseModel};
