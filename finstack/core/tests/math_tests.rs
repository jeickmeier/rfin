#[path = "math/common.rs"]
mod common;

// Unified interpolator tests (replaces individual interp_*.rs files)
#[path = "math/interp_unified.rs"]
mod interp_unified;

#[path = "math/interp_coverage.rs"]
mod interp_coverage;

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
