//! Centralized numerical tolerances used across the optimization stack.
//!
//! The optimizer mixes several different scales (raw quantities, notionals,
//! base-currency PVs, fractional weights, constraint slacks) in the same LP.
//! Picking a single tolerance for all of them is wrong — a 1e-9 weight is
//! genuinely negligible, while a 1e-9 base-currency-PV may not be. The
//! constants below name each tolerance by what it gates, so future changes
//! are easier to reason about and harder to miss.
//!
//! # Conventions
//!
//! - **`PV_PER_UNIT_TOL`**: smallest `|pv_per_unit|` we will divide by when
//!   reconstructing implied quantities under `ValueWeight`. Below this we
//!   either reject the candidate (decision-space) or set the implied
//!   quantity to zero (LP solver) — see the call sites for which.
//! - **`MIN_WEIGHT_TOL`**: smallest `|min_weight|` that we treat as
//!   "non-zero" when classifying candidates as long-only or
//!   long-short-eligible.
//! - **`GROSS_BASE_TOL`**: smallest gross base-currency / notional value
//!   below which weight normalization collapses to zero (avoids divide-by-
//!   zero on hedged-flat books).
//! - **`WEIGHT_TOL`**: smallest absolute weight we treat as a real position
//!   when filtering the trade list output.
//! - **`SLACK_TOL`**: smallest absolute constraint slack at which a
//!   constraint is reported as binding in the result envelope.
//!
//! These are deliberately *not* a single global tolerance. The PV scale
//! (`PV_PER_UNIT_TOL = 1e-12`) and the weight scale (`WEIGHT_TOL = 1e-9`)
//! and the constraint-slack scale (`SLACK_TOL = 1e-6`) all live at
//! different orders of magnitude because the quantities they gate live at
//! different orders of magnitude.

/// Smallest absolute `pv_per_unit` we will divide by when reconstructing
/// implied quantities. Used by both decision-space candidate filtering
/// and LP-solver quantity reconstruction.
pub const PV_PER_UNIT_TOL: f64 = 1e-12;

/// Smallest `|min_weight|` we treat as non-zero when classifying
/// candidates as "long-only-eligible" vs "shortable".
pub const MIN_WEIGHT_TOL: f64 = 1e-12;

/// Smallest absolute gross base-currency PV / notional below which the
/// weight-normalization denominator is treated as effectively zero.
/// Hedged-flat portfolios with cancelling longs and shorts can land here
/// even when individual positions are far from zero.
pub const GROSS_BASE_TOL: f64 = 1e-6;

/// Smallest absolute weight we report in the post-solve trade list.
/// Below this, the position is treated as not held / not traded.
pub const WEIGHT_TOL: f64 = 1e-9;

/// Smallest absolute constraint slack at which a constraint is reported
/// as binding in the result envelope.
pub const SLACK_TOL: f64 = 1e-6;
