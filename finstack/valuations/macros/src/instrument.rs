use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, GenericArgument, PathArguments, Type};

/// Derive macro to implement the `Instrument` trait with consistent boilerplate.
///
/// This macro is an internal codegen helper for the `finstack-valuations` crate.
/// It expands to crate-relative paths such as `crate::instruments::...`, so it is
/// not a hygienic public macro for use outside this crate hierarchy.
///
/// Usage:
/// ```text
/// #[derive(Instrument)]
/// #[instrument(key = "EquityOption", price_fn = "npv")]
/// pub struct EquityOption {
///     // ... fields required by the generated Instrument impl (id, attributes, etc.) ...
/// }
/// ```
pub(crate) fn derive_instrument_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let market_deps_impl = market_dependencies_impl(&input);

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

    let Some(key) = key_ident else {
        return syn::Error::new_spanned(
            &ident,
            "#[derive(Instrument)] requires #[instrument(key = \"InstrumentTypeVariant\")]",
        )
        .to_compile_error()
        .into();
    };

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
        impl #impl_generics crate::instruments::common_impl::traits::Instrument for #ident #ty_generics #where_clause {
            fn id(&self) -> &str {
                self.id.as_str()
            }

            fn key(&self) -> crate::pricer::InstrumentType {
                crate::pricer::InstrumentType::#key
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
                &self.attributes
            }

            fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
                &mut self.attributes
            }

            fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
                Box::new(self.clone())
            }

            fn value(
                &self,
                market: &finstack_core::market_data::context::MarketContext,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<finstack_core::money::Money> {
                self.#price_fn_ident(market, as_of)
            }

            #market_deps_impl
        }
    };

    TokenStream::from(expanded)
}

fn market_dependencies_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let spot_expr = spot_id_option_expr(input);
    let vol_expr = vol_surface_option_expr(input);
    let fx_pair_impl = fx_pair_impl(input);

    quote! {
        fn market_dependencies(
            &self,
        ) -> crate::instruments::common_impl::dependencies::MarketDependencies {
            let mut deps =
                crate::instruments::common_impl::dependencies::MarketDependencies::from_curve_dependencies(self);

            if let Some(spot_id) = #spot_expr {
                deps.add_spot_id(spot_id);
            }
            if let Some(vol_surface_id) = #vol_expr {
                deps.add_vol_surface_id(vol_surface_id.as_str());
            }

            #fx_pair_impl

            deps
        }
    }
}

fn spot_id_option_expr(input: &DeriveInput) -> proc_macro2::TokenStream {
    let Some(field_type) = find_field_type(input, "spot_id") else {
        return quote! { Option::<String>::None };
    };

    if is_option_string(&field_type) {
        quote! { self.spot_id.clone() }
    } else if is_string(&field_type) {
        quote! { Some(self.spot_id.clone()) }
    } else {
        quote! { Option::<String>::None }
    }
}

fn vol_surface_option_expr(input: &DeriveInput) -> proc_macro2::TokenStream {
    let Some(field_type) = find_field_type(input, "vol_surface_id") else {
        return quote! { Option::<finstack_core::types::CurveId>::None };
    };

    if is_option_curve_id(&field_type) {
        quote! { self.vol_surface_id.clone() }
    } else if is_curve_id(&field_type) {
        quote! { Some(self.vol_surface_id.clone()) }
    } else {
        quote! { Option::<finstack_core::types::CurveId>::None }
    }
}

fn fx_pair_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let has_base = find_field_type(input, "base_currency").is_some();
    let has_quote = find_field_type(input, "quote_currency").is_some();
    if has_base && has_quote {
        quote! {
            deps.add_fx_pair(self.base_currency, self.quote_currency);
        }
    } else {
        quote! {}
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
