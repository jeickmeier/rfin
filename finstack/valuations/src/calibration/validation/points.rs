//! Shared validation grids (static points) to avoid repeated allocations.

// Discount curve validation points
pub(crate) const DF_MONO_POINTS: &[f64] = &[
    0.0, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
];
pub(crate) const DF_BOUNDS_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0];

// Forward curve validation points
pub(crate) const FWD_ARBI_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
pub(crate) const FWD_BOUNDS_POINTS: &[f64] = &[0.25, 0.5, 1.0, 2.0, 5.0, 10.0];

// Hazard curve validation points
pub(crate) const HAZARD_ARBI_POINTS: &[f64] = &[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
pub(crate) const HAZARD_MONO_POINTS: &[f64] = &[0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
pub(crate) const HAZARD_BOUNDS_POINTS: &[f64] = &[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];

// Inflation curve validation points
pub(crate) const INFL_ARBI_POINTS: &[f64] = &[0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 20.0];
pub(crate) const INFL_MONO_POINTS: &[f64] = &[1.0, 2.0, 3.0, 5.0, 10.0];
pub(crate) const INFL_BOUNDS_POINTS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 20.0, 30.0];


