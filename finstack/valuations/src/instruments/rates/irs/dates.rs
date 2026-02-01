//! Date and business day utilities for interest rate swaps.
//!
//! This module re-exports the shared payment delay function from common pricing utilities.

// Re-export the shared strict implementation
pub(crate) use crate::instruments::common_impl::pricing::swap_legs::add_payment_delay;
