//! Shared rates pricing utilities.

/// Bermudan call provision shared across callable exotic rate products.
pub mod bermudan_call;
/// Cumulative coupon tracker for path-dependent products (TARN, Snowball).
pub mod cumulative_coupon;
/// Forward swap rate and annuity helpers shared by CMS instruments.
pub mod forward_swap_rate;
