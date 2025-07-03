#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Zero-std financial primitives & date utilities for the **RustFin** ecosystem.
//!
//! This crate exposes lightweight, composable building-blocks that are
//! commonly required in pricing engines and risk systems:
//!
//! * [`Currency`] – ISO-4217 codes with numeric identifiers and metadata
//! * [`Money`] – type-safe monetary amounts that refuse to mix currencies
//! * [`time`] – date/time scaffolding (business calendars, day-count, schedules)
//!
//! The implementation is `#![no_std]` by default and adds conveniences such as
//! `std::error::Error` & `Display` impls when compiled with the **`std`** feature.
//!
//! # Quick start
//! ```
//! use rfin_core::{Currency, Money};
//!
//! // Parse ISO-4217 codes (case-insensitive)
//! let eur = "eur".parse::<Currency>().unwrap();
//!
//! // Perform arithmetic that refuses to mix currencies
//! let subtotal = Money::eur(49.50);
//! let tax      = Money::eur( 9.90);
//! let total    = (subtotal + tax).unwrap();
//! assert_eq!(format!("{}", total), "59.4 EUR");
//! ```
//!
//! # Cargo features
//! | Feature       | Purpose                                            |
//! |-------------- |----------------------------------------------------|
//! | `std`         | Enables `std` trait impls (`Error`, `Display`, ...) |
//! | `serde`       | `Serialize`/`Deserialize` for public types         |
//! | `decimal128`  | `MoneyDecimal` using `rust_decimal::Decimal`       |
//!
//! # Minimum Supported Rust Version (MSRV)
//! This crate targets **Rust 1.75**.  It is tested in CI and follows the
//! standard *cargo-semver* guideline: MSRV may only bump in a **minor** release.
//!
//! ---
//! _Released under the MIT license.  Contributions welcome!_

#[cfg(feature = "std")]
extern crate std;

// Core modules
pub mod currency;
pub mod error;
pub mod money;

/// Date & calendar helpers (facade over the `time` crate)
pub mod dates;

// Re-export main error type for convenience
pub use error::Error;
/// Convenient alias carrying the crate's unified [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

// Top-level re-exports of commonly used primitives for easier discovery
pub use crate::currency::Currency;
pub use crate::money::Money;

// Top-level re-exports for ergonomic access – keeps `use` sites terse.
pub use crate::dates::DayCount;
pub use crate::dates::{Date, OffsetDateTime, PrimitiveDateTime};
pub use crate::dates::{DateExt, OffsetDateTimeExt};

// Schedule frequency re-export
pub use crate::dates::Frequency;
