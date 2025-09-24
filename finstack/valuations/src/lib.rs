//! Finstack Valuations module.
//!
//! Provides pricing, risk metrics, and cashflow generation for financial instruments.
//! Built on a metrics framework that separates pricing logic from measure computation.
//!
//! # Quick Start
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::results::ValuationResult;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention};
//! use finstack_core::dates::StubKind;
//! use time::Month;
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//! // Note: Bond constructor would be used here
//! ```

pub mod calibration;
pub mod cashflow;
pub mod pricer;
pub mod results;

// Export macros before instruments module
#[macro_use]
pub mod instruments;
pub mod covenants;
pub mod metrics;
pub mod performance;

pub use finstack_core::prelude::*;

// Minimal solver factory to replace macro-based construction
pub mod solver_factory {
    use crate::calibration::{CalibrationConfig, SolverKind};
    use finstack_core::math::{BrentSolver, HybridSolver, NewtonSolver, Solver};
    use finstack_core::F;

    #[derive(Clone, Debug)]
    pub enum Solver1D {
        Newton(NewtonSolver),
        Brent(BrentSolver),
        Hybrid(HybridSolver),
    }

    impl Solver for Solver1D {
        fn solve<Func>(&self, f: Func, initial_guess: F) -> finstack_core::Result<F>
        where
            Func: Fn(F) -> F,
        {
            match self {
                Solver1D::Newton(s) => s.solve(f, initial_guess),
                Solver1D::Brent(s) => s.solve(f, initial_guess),
                Solver1D::Hybrid(s) => s.solve(f, initial_guess),
            }
        }
    }

    pub fn make_solver(cfg: &CalibrationConfig) -> Solver1D {
        match cfg.solver_kind {
            SolverKind::Newton => {
                let s = NewtonSolver::new()
                    .with_tolerance(cfg.tolerance)
                    .with_max_iterations(cfg.max_iterations);
                Solver1D::Newton(s)
            }
            SolverKind::Brent => {
                let s = BrentSolver::new().with_tolerance(cfg.tolerance);
                Solver1D::Brent(s)
            }
            SolverKind::Hybrid => {
                let s = HybridSolver::new()
                    .with_tolerance(cfg.tolerance)
                    .with_max_iterations(cfg.max_iterations);
                Solver1D::Hybrid(s)
            }
            // Fallback to Hybrid for unsupported 1D kinds in this factory
            SolverKind::LevenbergMarquardt | SolverKind::DifferentialEvolution => {
                let s = HybridSolver::new()
                    .with_tolerance(cfg.tolerance)
                    .with_max_iterations(cfg.max_iterations);
                Solver1D::Hybrid(s)
            }
        }
    }
}
