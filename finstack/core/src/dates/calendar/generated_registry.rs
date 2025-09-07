//! Generated calendar registry and calendar type declarations.
//!
//! This module includes the build-generated registry and calendar types and
//! re-exports them at the parent `dates::calendar` level via `pub use` in
//! `mod.rs`.

// Bring Rule into scope for generated constants like `const XX_RULES: &[Rule] = ...`.
use crate::dates::calendar::Rule;

// The generated file relies on crate-exported macros like `declare_calendar!`.
include!(concat!(env!("OUT_DIR"), "/generated_calendars.rs"));
