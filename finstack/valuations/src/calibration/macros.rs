//! Calibration helper macro – maintained for backward compatibility.
//!
//! New code should prefer the tiny factory `make_solver(&cfg)` which returns a
//! concrete `SolverInstance`. This macro simply expands to that call and binds
//! a local `$solver` for the provided body.

#[macro_export]
macro_rules! with_solver {
    ($config:expr, |$solver:ident| $body:expr) => {{
        let $solver = $crate::calibration::make_solver($config);
        $body
    }};
}
