//! Commonly used types and functions.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_portfolio::prelude::*;
//! ```

// Re-export everything from the crate root (all pub use items from lib.rs)
pub use crate::*;

// Explicit re-export to disambiguate from finstack_core::prelude::{Error, Result}
pub use crate::error::{Error, Result};

// Re-export the full core prelude for a unified foundation
pub use finstack_core::prelude::*;
