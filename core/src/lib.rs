#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Core financial primitives and date handling for the RustFin library.
//!
//! This crate provides fundamental building blocks for financial computations,
//! including date/time handling and other financial primitives,
//! designed to work in no_std environments by default.

#[cfg(feature = "std")]
extern crate std;

// Internal macros
mod macros;

// Core modules
pub mod dates;
pub mod error;
pub mod primitives;

// Re-export main error type for convenience
pub use error::Error;
