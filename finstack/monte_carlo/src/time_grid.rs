//! Time grid types from [`finstack_core::math::time_grid`].
//!
//! This module re-exports the core implementation so Monte Carlo callers can depend on
//! `finstack-monte_carlo` alone. For non-MC code, import [`TimeGrid`](finstack_core::math::time_grid::TimeGrid)
//! directly from `finstack-core`.

pub use finstack_core::math::time_grid::*;
