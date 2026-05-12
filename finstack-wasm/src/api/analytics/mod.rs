//! WASM bindings for `finstack-analytics`.
//!
//! The only entry point exposed to JS is the [`Performance`] class. Every
//! analytic — returns/risk metrics, periodic returns, benchmark comparisons,
//! basic factor models — is reachable as a `Performance` method. The
//! supporting `CagrBasis` and `BenchmarkAlignmentPolicy` classes are exposed
//! as value-object inputs.

mod performance;
mod support;
mod types;

pub use performance::WasmPerformance;
pub use types::{WasmBenchmarkAlignmentPolicy, WasmCagrBasis};
