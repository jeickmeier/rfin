//! Tolerance constants for numerical comparisons in risk tests.
//!
//! Using named constants instead of magic numbers improves readability
//! and ensures consistency across tests.

/// Tight tolerance for bit-exact or analytical comparisons.
/// Use when comparing identical calculations or analytical formulas.
pub const TIGHT: f64 = 1e-10;

/// Standard tolerance for finite-difference vs analytical comparisons.
/// Use when comparing FD approximations to analytical values.
pub const STANDARD: f64 = 1e-6;

/// Loose tolerance for Monte Carlo or cross-methodology comparisons.
/// Use when comparing MC results or different calculation methods.
pub const LOOSE: f64 = 1e-3;

/// 0.01% relative tolerance.
/// Use for high-precision relative comparisons.
pub const PERCENT_001: f64 = 0.0001;

/// 0.1% relative tolerance.
/// Use for standard relative comparisons.
pub const PERCENT_01: f64 = 0.001;

/// 1% relative tolerance.
/// Use for loose relative comparisons (e.g., FD approximations).
pub const PERCENT_1: f64 = 0.01;

/// 5% relative tolerance.
/// Use for very loose comparisons (e.g., second-order greeks via FD).
pub const PERCENT_5: f64 = 0.05;

/// Small absolute threshold for near-zero checks.
/// Use when determining if a value is effectively zero.
pub const NEAR_ZERO: f64 = 1e-8;
