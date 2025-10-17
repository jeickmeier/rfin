//! CDS instrument tests module.
//!
//! Comprehensive test suite organized by functionality:
//! - `construction`: CDS creation, builders, and conventions
//! - `pricing`: Core pricing logic (legs, NPV, schedules)
//! - `metrics`: All metrics calculations (CS01, DV01, etc.)
//! - `integration_methods`: Numerical integration approaches
//! - `market_validation`: ISDA standards and market benchmarks
//! - `edge_cases`: Boundary conditions and error handling

mod test_cds_construction;
mod test_cds_edge_cases;
mod test_cds_integration_methods;
mod test_cds_market_validation;
mod test_cds_metrics;
mod test_cds_pricing;

// QuantLib parity tests
mod quantlib_parity;

// CDS index tests (commented out - needs API updates)
// mod test_cds_index_imm_factor_cs01;
