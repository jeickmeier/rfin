//! Market data build tests.
//!
//! Tests for building instruments from market quotes.
//!
//! ## Test Organization
//!
//! - `credit` - Credit instrument building from CDS quotes
//! - `rates` - Rate instrument building from deposit/swap/FRA quotes
//! - `fx` - FX instrument building from FX forward quotes
//! - `bond` - Bond instrument building from clean-price quotes
//! - `xccy` - Cross-currency swap instrument building from basis swap quotes

pub mod bond;
pub mod credit;
pub mod fx;
pub mod rates;
pub mod xccy;
