//! CDS instrument tests module.
//!
//! Comprehensive test suite organized by functionality:
//! - `construction`: CDS creation, builders, and conventions
//! - `pricing`: Core pricing logic (legs, NPV, schedules)
//! - `metrics`: All metrics calculations (CS01, DV01, etc.)
//! - `integration_methods`: Numerical integration approaches
//! - `market_validation`: ISDA standards and market benchmarks
//! - `edge_cases`: Boundary conditions and error handling
//! - `golden_vectors`: ISDA Standard Model reference validation (tight tolerances)

mod test_cds_construction;
mod test_cds_edge_cases;
mod test_cds_integration_methods;
mod test_cds_market_validation;
mod test_cds_metrics;
mod test_cds_pricing;
mod test_par_spread_roundtrip;

// Golden vector tests with ISDA Standard Model reference values
mod golden_vectors;

// QuantLib parity tests (legacy - being replaced by golden_vectors)
mod quantlib_parity;

mod test_cds_pricing_2;
mod test_upfront;
