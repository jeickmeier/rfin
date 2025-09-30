//! Procedural macros for the finstack-valuations crate.
//!
//! This crate provides derive and attribute macros to reduce boilerplate
//! and improve type safety in the valuations module:
//!
//! - `FinancialBuilder`: Generates type-safe builder patterns for instruments
//! - `register_pricer`: Auto-registers pricers with the global registry

use proc_macro::TokenStream;

mod financial_builder;
mod register_pricer;

/// Derives a builder pattern for financial instrument structs.
///
/// See the `financial_builder` module for detailed documentation.
#[proc_macro_derive(FinancialBuilder, attributes(builder))]
pub fn derive_financial_builder(input: TokenStream) -> TokenStream {
    financial_builder::derive_financial_builder_impl(input)
}

/// Auto-register a Pricer implementation with the global pricer registry.
///
/// See the `register_pricer` module for detailed documentation.
#[proc_macro_attribute]
pub fn register_pricer(attr: TokenStream, item: TokenStream) -> TokenStream {
    register_pricer::register_pricer_impl(attr, item)
}