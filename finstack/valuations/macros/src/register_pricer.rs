//! Register pricer attribute macro for auto-registration with the pricer registry.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

/// Implementation of the register_pricer attribute macro.
///
/// This attribute macro should be placed on an `impl Pricer for T` block.
/// It will automatically register an instance of the pricer using the `inventory` crate.
///
/// The macro generates an `inventory::submit!` block that registers a constructor
/// function for the pricer. At runtime, all registered pricers are collected and
/// added to the standard pricer registry.
///
/// # Example
///
/// ```ignore
/// #[register_pricer]
/// impl Pricer for SimpleBondOasPricer {
///     fn key(&self) -> PricerKey {
///         PricerKey::new(InstrumentType::Bond, ModelKey::Oas)
///     }
///
///     fn price(&self, instrument: &dyn Instrument, market: &Market) -> Result<Money> {
///         // ... implementation
///     }
/// }
/// ```
///
/// This expands to:
///
/// ```ignore
/// impl Pricer for SimpleBondOasPricer {
///     // ... original implementation
/// }
///
/// inventory::submit! {
///     crate::pricer::PricerRegistration {
///         ctor: || Box::new(SimpleBondOasPricer::new()),
///     }
/// }
/// ```
pub fn register_pricer_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    // Extract the type being implemented
    let self_ty = &input.self_ty;

    // Generate the inventory submission using crate:: for internal use
    let expanded = quote! {
        #input

        inventory::submit! {
            crate::pricer::PricerRegistration {
                ctor: || Box::new(<#self_ty>::new()),
            }
        }
    };

    TokenStream::from(expanded)
}
