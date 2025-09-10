//! Comprehensive calibration framework for term structures and surfaces.
//!
//! Provides market-standard calibration methodologies for:
//! - Interest rate curves (discount/forward)
//! - Credit curves (survival/hazard)
//! - Inflation curves
//! - Volatility surfaces
//! - Base correlation curves
//!
//! Supports both sequential bootstrapping and global optimization approaches.

pub mod bootstrap;
pub mod primitives;
pub mod simple_calibration;

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Result, F};
use std::collections::HashMap;

// Re-export the simple calibration API for convenience
pub use simple_calibration::SimpleCalibration;

/// Core trait for calibrating market data structures from instrument quotes.
///
/// This trait provides a unified interface for all calibration processes,
/// whether they involve sequential bootstrapping or global optimization.
pub trait Calibrator<Input, Output> {
    /// Calibrate the target structure to match market quotes.
    ///
    /// # Arguments
    /// * `instruments` - Market instruments providing calibration constraints
    /// * `quotes` - Market quotes for the instruments  
    /// * `base_context` - Base market data (e.g., discount curves for credit calibration)
    ///
    /// # Returns
    /// Calibrated output structure and diagnostic report
    fn calibrate(
        &self,
        quotes: &[Input],
        base_context: &MarketContext,
    ) -> Result<(Output, CalibrationReport)>;
}

/// Market quote with bid/ask spread and metadata.
#[derive(Clone, Debug)]
pub struct MarketQuote {
    /// Instrument identifier
    pub instrument_id: String,
    /// Quote value (rate, spread, volatility, etc.)
    pub value: F,
    /// Bid-ask spread (optional)
    pub bid_ask_spread: Option<F>,
    /// Quote timestamp
    pub as_of: finstack_core::dates::Date,
    /// Market convention/source
    pub source: String,
    /// Quality indicator (0-100, 100 = best)
    pub quality: Option<u8>,
}

impl MarketQuote {
    /// Create a new market quote.
    pub fn new(
        instrument_id: impl Into<String>,
        value: F,
        as_of: finstack_core::dates::Date,
        source: impl Into<String>,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            value,
            bid_ask_spread: None,
            as_of,
            source: source.into(),
            quality: None,
        }
    }

    /// Set bid-ask spread.
    pub fn with_bid_ask_spread(mut self, spread: F) -> Self {
        self.bid_ask_spread = Some(spread);
        self
    }

    /// Set quality indicator.
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = Some(quality);
        self
    }
}

/// Calibration diagnostic report.
#[derive(Clone, Debug)]
pub struct CalibrationReport {
    /// Calibration success flag
    pub success: bool,
    /// Final residuals by instrument
    pub residuals: HashMap<String, F>,
    /// Number of iterations taken
    pub iterations: usize,
    /// Final objective function value
    pub objective_value: F,
    /// Maximum absolute residual
    pub max_residual: F,
    /// Root mean square error
    pub rmse: F,
    /// Convergence reason
    pub convergence_reason: String,
    /// Calibration metadata
    pub metadata: HashMap<String, String>,
}

impl CalibrationReport {
    /// Create a new calibration report.
    pub fn new() -> Self {
        Self {
            success: false,
            residuals: HashMap::new(),
            iterations: 0,
            objective_value: F::INFINITY,
            max_residual: F::INFINITY,
            rmse: F::INFINITY,
            convergence_reason: "Not started".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Mark calibration as successful.
    pub fn success(mut self) -> Self {
        self.success = true;
        self
    }

    /// Set residuals.
    pub fn with_residuals(mut self, residuals: HashMap<String, F>) -> Self {
        self.max_residual = residuals.values().map(|r| r.abs()).fold(0.0, f64::max);
        let sum_sq: F = residuals.values().map(|r| r * r).sum();
        self.rmse = if residuals.is_empty() {
            0.0
        } else {
            (sum_sq / residuals.len() as F).sqrt()
        };
        self.residuals = residuals;
        self
    }

    /// Set iteration count.
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Set convergence reason.
    pub fn with_convergence_reason(mut self, reason: impl Into<String>) -> Self {
        self.convergence_reason = reason.into();
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Create a successful calibration report with common fields set.
    pub fn success_with(
        residuals: HashMap<String, F>,
        iterations: usize,
        reason: impl Into<String>,
    ) -> Self {
        Self::new()
            .success()
            .with_residuals(residuals)
            .with_iterations(iterations)
            .with_convergence_reason(reason)
    }

    /// Add a single residual to the report.
    pub fn push_residual(&mut self, key: impl Into<String>, value: F) {
        self.residuals.insert(key.into(), value);
        // Update derived metrics
        self.max_residual = self.residuals.values().map(|r| r.abs()).fold(0.0, f64::max);
        let sum_sq: F = self.residuals.values().map(|r| r * r).sum();
        self.rmse = if self.residuals.is_empty() {
            0.0
        } else {
            (sum_sq / self.residuals.len() as F).sqrt()
        };
    }
}

impl Default for CalibrationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Solver type selection for calibration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SolverKind {
    /// Newton-Raphson solver with automatic derivative estimation
    Newton,
    /// Brent's method solver (robust, bracketing required)
    Brent,
    /// Hybrid solver that tries Newton first, falls back to Brent
    Hybrid,
}

impl Default for SolverKind {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Configuration for calibration processes.
#[derive(Clone, Debug)]
pub struct CalibrationConfig {
    /// Solver tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Use parallel processing when available
    pub use_parallel: bool,
    /// Random seed for reproducible results
    pub random_seed: Option<u64>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Solver type selection
    pub solver_kind: SolverKind,
    /// Entity-specific seniority mappings for credit calibration
    pub entity_seniority:
        HashMap<String, finstack_core::market_data::term_structures::hazard_curve::Seniority>,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            use_parallel: false, // Deterministic by default
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::default(),
            entity_seniority: HashMap::new(),
        }
    }
}

/// Macro to apply calibration with the appropriate solver type.
/// This avoids the need for a SolverInstance enum while working around
/// the object-safety limitations of the Solver trait.
#[macro_export]
macro_rules! with_solver {
    ($config:expr, |$solver:ident| $body:expr) => {
        match $config.solver_kind {
            $crate::calibration::SolverKind::Newton => {
                let $solver = finstack_core::math::NewtonSolver::new()
                    .with_tolerance($config.tolerance)
                    .with_max_iterations($config.max_iterations);
                $body
            }
            $crate::calibration::SolverKind::Brent => {
                let $solver = finstack_core::math::BrentSolver::new()
                    .with_tolerance($config.tolerance);
                $body
            }
            $crate::calibration::SolverKind::Hybrid => {
                let $solver = finstack_core::math::HybridSolver::new();
                $body
            }
        }
    };
}

/// Calibration error types.
#[derive(Debug)]
pub enum CalibrationError {
    /// Convergence failure
    ConvergenceFailure { iterations: usize, final_error: F },
    /// Insufficient market data
    InsufficientData { message: String },
    /// Invalid market quotes
    InvalidQuotes { message: String },
    /// Numerical instability
    NumericalInstability { message: String },
    /// No-arbitrage violation
    ArbitrageViolation { message: String },
}

impl std::fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalibrationError::ConvergenceFailure {
                iterations,
                final_error,
            } => {
                write!(
                    f,
                    "Failed to converge after {} iterations, final error: {}",
                    iterations, final_error
                )
            }
            CalibrationError::InsufficientData { message } => {
                write!(f, "Insufficient market data: {}", message)
            }
            CalibrationError::InvalidQuotes { message } => {
                write!(f, "Invalid market quotes: {}", message)
            }
            CalibrationError::NumericalInstability { message } => {
                write!(f, "Numerical instability: {}", message)
            }
            CalibrationError::ArbitrageViolation { message } => {
                write!(f, "No-arbitrage violation detected: {}", message)
            }
        }
    }
}

impl std::error::Error for CalibrationError {}

impl From<CalibrationError> for finstack_core::Error {
    fn from(err: CalibrationError) -> Self {
        let (message, category) = match &err {
            CalibrationError::ConvergenceFailure {
                iterations,
                final_error,
            } => (
                format!(
                    "Failed to converge after {} iterations, final error: {}",
                    iterations, final_error
                ),
                "convergence".to_string(),
            ),
            CalibrationError::InsufficientData { message } => (
                format!("Insufficient market data: {}", message),
                "data".to_string(),
            ),
            CalibrationError::InvalidQuotes { message } => (
                format!("Invalid market quotes: {}", message),
                "quotes".to_string(),
            ),
            CalibrationError::NumericalInstability { message } => (
                format!("Numerical instability: {}", message),
                "numerical".to_string(),
            ),
            CalibrationError::ArbitrageViolation { message } => (
                format!("No-arbitrage violation detected: {}", message),
                "arbitrage".to_string(),
            ),
        };
        finstack_core::Error::Calibration { message, category }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::math::Solver;

    #[test]
    fn test_solver_selection() {
        // Test that different solver kinds can be created and work
        let mut config = CalibrationConfig::default();

        // Test default (Hybrid) - verify it can solve
        let f = |x: F| x * x - 4.0; // Root at x = 2
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);

        // Test Newton
        config.solver_kind = SolverKind::Newton;
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);

        // Test Brent
        config.solver_kind = SolverKind::Brent;
        let root = crate::with_solver!(&config, |solver| solver.solve(f, 1.5).unwrap());
        assert!((root - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_calibration_config_defaults() {
        let config = CalibrationConfig::default();
        assert_eq!(config.tolerance, 1e-10);
        assert_eq!(config.max_iterations, 100);
        assert!(!config.use_parallel);
        assert_eq!(config.random_seed, Some(42));
        assert!(!config.verbose);
        assert_eq!(config.solver_kind, SolverKind::Hybrid);
        assert!(config.entity_seniority.is_empty());
    }

    #[test]
    fn test_solver_kind_default() {
        assert_eq!(SolverKind::default(), SolverKind::Hybrid);
    }

    #[test]
    fn test_calibration_report_success_with() {
        let mut residuals = HashMap::new();
        residuals.insert("test_instrument".to_string(), 1e-6);
        residuals.insert("another_instrument".to_string(), 2e-6);

        let report = CalibrationReport::success_with(residuals, 10, "Test calibration completed");

        assert!(report.success);
        assert_eq!(report.iterations, 10);
        assert_eq!(report.convergence_reason, "Test calibration completed");
        assert_eq!(report.residuals.len(), 2);
        assert!(report.max_residual > 0.0);
        assert!(report.rmse > 0.0);
    }

    #[test]
    fn test_calibration_report_push_residual() {
        let mut report = CalibrationReport::new().success();

        report.push_residual("instrument1", 1e-6);
        report.push_residual("instrument2", 2e-6);

        assert_eq!(report.residuals.len(), 2);
        assert!((report.max_residual - 2e-6).abs() < 1e-12);

        // Test that metrics are updated correctly
        let expected_rmse = ((1e-12_f64 + 4e-12_f64) / 2.0_f64).sqrt();
        assert!((report.rmse - expected_rmse).abs() < 1e-15);
    }
}
