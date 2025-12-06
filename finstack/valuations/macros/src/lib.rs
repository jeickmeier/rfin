//! Procedural macros for the finstack-valuations crate.
//!
//! This crate provides derive and attribute macros to reduce boilerplate
//! and improve type safety in the valuations module:
//!
//! - `FinancialBuilder`: Generates type-safe builder patterns for instruments
//! - `Instrument`: Generates boilerplate `Instrument` trait implementations

use proc_macro::TokenStream;

mod financial_builder;
mod instrument;

/// Derives a builder pattern for financial instrument structs.
///
/// See the `financial_builder` module for detailed documentation.
#[proc_macro_derive(FinancialBuilder, attributes(builder))]
pub fn derive_financial_builder(input: TokenStream) -> TokenStream {
    financial_builder::derive_financial_builder_impl(input)
}

/// Derives the `Instrument` trait implementation.
#[proc_macro_derive(Instrument, attributes(instrument))]
pub fn derive_instrument(input: TokenStream) -> TokenStream {
    instrument::derive_instrument_impl(input)
}
