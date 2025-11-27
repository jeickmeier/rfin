//! Monte Carlo integration tests.
//!
//! This module includes tests for:
//! - Path capture functionality
//! - Analytical vs MC model selection
//! - Process parameter serialization

#![cfg(feature = "mc")]

#[path = "mc/mc_path_capture_tests.rs"]
mod mc_path_capture_tests;

#[path = "mc/analytical_vs_mc_choice_test.rs"]
mod analytical_vs_mc_choice_test;

#[path = "mc/mc_process_params_serialization.rs"]
mod mc_process_params_serialization;
