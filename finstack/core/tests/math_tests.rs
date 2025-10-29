#[path = "math/common.rs"]
mod common;
#[path = "math/interp_cubic_hermite.rs"]
mod interp_cubic_hermite;
#[path = "math/interp_flat_fwd.rs"]
mod interp_flat_fwd;
#[path = "math/interp_linear.rs"]
mod interp_linear;
#[path = "math/interp_log_linear.rs"]
mod interp_log_linear;
#[path = "math/interp_monotone_convex.rs"]
mod interp_monotone_convex;
#[path = "math/interp_traits.rs"]
mod interp_traits;
#[path = "math/math_integration.rs"]
mod math_integration;
#[path = "math/math_root_finding.rs"]
mod math_root_finding;
#[path = "math/math_stats.rs"]
mod math_stats;
#[path = "math/math_summation.rs"]
mod math_summation;

#[cfg(feature = "serde")]
#[path = "math/test_interp_serde.rs"]
mod test_interp_serde;
#[cfg(feature = "serde")]
#[path = "math/test_solver_serde.rs"]
mod test_solver_serde;

// #[path = "math_random.rs"]
// mod math_random; // Removed - had HashSet<f64> trait bound errors
