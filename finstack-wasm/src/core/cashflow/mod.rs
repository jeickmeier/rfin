pub mod primitives;

pub use primitives::{JsCFKind, JsCashFlow};

// Note: Performance functions (npv, irr_periodic, xirr) are exported from
// `valuations::performance` and re-exported at the top level as:
// - calculateNpv / npv
// - irrPeriodic
// - xirr
// The TypeScript layer can create module aliases for parity with Python's
// `finstack.core.cashflow.performance` module layout.
