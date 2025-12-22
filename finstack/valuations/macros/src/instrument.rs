use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, GenericArgument, PathArguments, Type};

/// Derive macro to implement the `Instrument` trait with consistent boilerplate.
///
/// Usage:
/// ```text
/// #[derive(Instrument)]
/// #[instrument(key = "EquityOption", price_fn = "npv")]
/// pub struct EquityOption {
///     // ... fields required by the generated Instrument impl (id, attributes, etc.) ...
/// }
/// ```
pub fn derive_instrument_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let spot_id_impl = spot_id_impl(&input);
    let vol_surface_impl = vol_surface_impl(&input);

    let mut key_ident: Option<syn::Ident> = None;
    let mut price_fn_ident: syn::Ident = format_ident!("npv");

    for attr in &input.attrs {
        if !attr.path().is_ident("instrument") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                let lit: syn::LitStr = meta.value()?.parse()?;
                key_ident = Some(format_ident!("{}", lit.value()));
            } else if meta.path.is_ident("price_fn") {
                let lit: syn::LitStr = meta.value()?.parse()?;
                price_fn_ident = format_ident!("{}", lit.value());
            }
            Ok(())
        });
    }

    let key = key_ident.unwrap_or_else(|| {
        panic!("#[derive(Instrument)] requires #[instrument(key = \"InstrumentTypeVariant\")]")
    });

    let (_, ty_generics_no_where, _) = input.generics.split_for_impl();
    let self_type: syn::Type = syn::parse_quote!(#ident #ty_generics_no_where);

    let mut generics = input.generics.clone();
    {
        let where_clause = generics.make_where_clause();
        where_clause
            .predicates
            .push(syn::parse_quote!(#self_type: Clone + Send + Sync));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics crate::instruments::common::traits::Instrument for #ident #ty_generics #where_clause {
            fn id(&self) -> &str {
                self.id.as_str()
            }

            fn key(&self) -> crate::pricer::InstrumentType {
                crate::pricer::InstrumentType::#key
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
                &self.attributes
            }

            fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
                &mut self.attributes
            }

            fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
                Box::new(self.clone())
            }

            fn value(
                &self,
                market: &finstack_core::market_data::context::MarketContext,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<finstack_core::money::Money> {
                self.#price_fn_ident(market, as_of)
            }

            fn price_with_metrics(
                &self,
                market: &finstack_core::market_data::context::MarketContext,
                as_of: finstack_core::dates::Date,
                metrics: &[crate::metrics::MetricId],
            ) -> finstack_core::Result<crate::results::ValuationResult> {
                let base_value = self.value(market, as_of)?;
                crate::instruments::common::helpers::build_with_metrics_dyn(
                    std::sync::Arc::new(self.clone()),
                    std::sync::Arc::new(market.clone()),
                    as_of,
                    base_value,
                    metrics,
                )
            }

            #spot_id_impl
            #vol_surface_impl
        }
    };

    TokenStream::from(expanded)
}

fn spot_id_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let Some(field_type) = find_field_type(input, "spot_id") else {
        return quote! {};
    };

    let body = if is_option_string(&field_type) {
        quote! { self.spot_id.as_deref() }
    } else if is_string(&field_type) {
        quote! { Some(self.spot_id.as_str()) }
    } else {
        return quote! {};
    };

    quote! {
        fn spot_id(&self) -> Option<&str> {
            #body
        }
    }
}

fn vol_surface_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let Some(field_type) = find_field_type(input, "vol_surface_id") else {
        return quote! {};
    };

    let body = if is_option_curve_id(&field_type) {
        quote! { self.vol_surface_id.clone() }
    } else if is_curve_id(&field_type) {
        quote! { Some(self.vol_surface_id.clone()) }
    } else {
        return quote! {};
    };

    quote! {
        fn vol_surface_id(&self) -> Option<finstack_core::types::CurveId> {
            #body
        }
    }
}

fn find_field_type(input: &DeriveInput, field_name: &str) -> Option<Type> {
    match &input.data {
        Data::Struct(data_struct) => data_struct
            .fields
            .iter()
            .find(|field| {
                field
                    .ident
                    .as_ref()
                    .map(|id| id == field_name)
                    .unwrap_or(false)
            })
            .map(|field| field.ty.clone()),
        _ => None,
    }
}

fn is_option_curve_id(ty: &Type) -> bool {
    is_option_of(ty, "CurveId")
}

fn is_curve_id(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        return type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident == "CurveId")
            .unwrap_or(false);
    }
    false
}

fn is_option_string(ty: &Type) -> bool {
    is_option_of(ty, "String")
}

fn is_string(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        return type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident == "String")
            .unwrap_or(false);
    }
    false
}

fn is_option_of(ty: &Type, target: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(Type::Path(inner_path))) = args.args.first() {
                        if let Some(last) = inner_path.path.segments.last() {
                            return last.ident == target;
                        }
                    }
                }
            }
        }
    }
    false
}
