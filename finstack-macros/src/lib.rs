use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(FinancialBuilder, attributes(builder))]
pub fn derive_financial_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident.clone();

    // Collect fields and builder annotations
    let mut required_fields: Vec<(syn::Ident, syn::Type)> = Vec::new();
    let mut optional_fields: Vec<(syn::Ident, syn::Type)> = Vec::new();
    // Heuristics for post-build validations
    let mut has_start_date: bool = false;
    let mut has_maturity: bool = false;
    let mut has_strike_variance: bool = false;

    let fields = match input.data {
        Data::Struct(s) => s.fields,
        _ => panic!("FinancialBuilder can only be derived for structs"),
    };

    // By default, treat Option<T> as optional; #[builder(optional)] is honored only when field type is Option<...>
    if let Fields::Named(named) = fields {
        for f in named.named {
            let ident = f.ident.unwrap();
            let ty = f.ty.clone();

            let mut has_optional_attr = false;
            for attr in f.attrs {
                if attr.path().is_ident("builder") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("optional") {
                            has_optional_attr = true;
                        }
                        Ok(())
                    });
                }
            }

            let is_option_ty = matches!(ty, syn::Type::Path(ref tp) if tp.path.segments.last().map(|s| s.ident == "Option").unwrap_or(false));

            // Track presence of well-known fields for generic validations
            if ident == format_ident!("start_date") {
                has_start_date = true;
            }
            if ident == format_ident!("maturity") {
                has_maturity = true;
            }
            if ident == format_ident!("strike_variance") {
                has_strike_variance = true;
            }

            // Optional if Option<T> or the field is `attributes`
            if is_option_ty || ident == format_ident!("attributes") {
                optional_fields.push((ident, ty));
            } else {
                required_fields.push((ident, ty));
            }
        }
    } else {
        panic!("FinancialBuilder requires named fields");
    }

    let builder_name = format_ident!("{}Builder", struct_name);

    // Builder struct fields are Option<...> for required, and same type for optional if already Option<T>, else Option<T>
    let builder_req_fields = required_fields
        .iter()
        .map(|(id, ty)| quote! { #id: ::core::option::Option<#ty> });
    let builder_opt_fields = optional_fields.iter().map(|(id, ty)| {
        if let syn::Type::Path(ref tp) = ty {
            if tp
                .path
                .segments
                .last()
                .map(|s| s.ident == "Option")
                .unwrap_or(false)
            {
                // Keep Option<T> as is
                quote! { #id: #ty }
            } else {
                quote! { #id: ::core::option::Option<#ty> }
            }
        } else {
            quote! { #id: ::core::option::Option<#ty> }
        }
    });

    // Setter methods
    let setter_req = required_fields.iter().map(|(id, ty)| quote! {
        pub fn #id(mut self, value: #ty) -> Self { self.#id = ::core::option::Option::Some(value); self }
    });

    let setter_opt = optional_fields.iter().map(|(id, ty)| {
        if let syn::Type::Path(ref tp) = ty {
            if let Some(seg) = tp.path.segments.last() {
                if seg.ident == "Option" {
                    // Extract inner type Option<Inner>
                    let inner_ty: Option<syn::Type> = match &seg.arguments {
                        syn::PathArguments::AngleBracketed(ab) => ab.args.iter().find_map(|ga| {
                            if let syn::GenericArgument::Type(t) = ga { Some(t.clone()) } else { None }
                        }),
                        _ => None,
                    };
                    if let Some(inner) = inner_ty {
                        let set_opt = format_ident!("{}_opt", id);
                        quote! {
                            pub fn #id(mut self, value: #inner) -> Self { self.#id = ::core::option::Option::Some(value); self }
                            pub fn #set_opt(mut self, value: #ty) -> Self { self.#id = value; self }
                        }
                    } else {
                        quote! { pub fn #id(mut self, value: #ty) -> Self { self.#id = value; self } }
                    }
                } else {
                    quote! { pub fn #id(mut self, value: #ty) -> Self { self.#id = ::core::option::Option::Some(value); self } }
                }
            } else {
                quote! { pub fn #id(mut self, value: #ty) -> Self { self.#id = ::core::option::Option::Some(value); self } }
            }
        } else {
            quote! { pub fn #id(mut self, value: #ty) -> Self { self.#id = ::core::option::Option::Some(value); self } }
        }
    });

    // Build expression: required fields unwrap, optional fields carry through (unwrap_or(None)) and initialize attributes if present
    let assign_req = required_fields
        .iter()
        .map(|(id, _)| quote! { #id: self.#id.ok_or(finstack_core::error::InputError::Invalid)? });

    let assign_opt = optional_fields.iter().map(|(id, ty)| {
        if let syn::Type::Path(ref tp) = ty {
            if tp
                .path
                .segments
                .last()
                .map(|s| s.ident == "Option")
                .unwrap_or(false)
            {
                quote! { #id: self.#id }
            } else if id == "attributes" {
                quote! { attributes: self.attributes.unwrap_or_default() }
            } else {
                quote! { #id: self.#id.unwrap_or_default() }
            }
        } else if id == "attributes" {
            quote! { attributes: self.attributes.unwrap_or_default() }
        } else {
            quote! { #id: self.#id.unwrap_or_default() }
        }
    });

    // Post-build validation snippets based on field presence
    let mut post_build_checks = proc_macro2::TokenStream::new();
    if has_start_date && has_maturity {
        post_build_checks.extend(quote! {
            if __built.start_date >= __built.maturity {
                return ::core::result::Result::Err(finstack_core::error::InputError::InvalidDateRange.into());
            }
        });
    }
    if has_strike_variance {
        post_build_checks.extend(quote! {
            if __built.strike_variance < 0.0 {
                return ::core::result::Result::Err(finstack_core::error::InputError::NegativeValue.into());
            }
        });
    }

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Default)]
        pub struct #builder_name {
            #(#builder_req_fields,)*
            #(#builder_opt_fields,)*
        }

        impl #builder_name {
            pub fn new() -> Self { Self::default() }
            #(#setter_req)*
            #(#setter_opt)*

            pub fn build(self) -> finstack_core::Result<#struct_name> {
                let __built = #struct_name {
                    #(#assign_req,)*
                    #(#assign_opt,)*
                };
                // Generic sanity checks for common domain fields
                #post_build_checks
                Ok(__built)
            }
        }

        impl #struct_name {
            pub fn builder() -> #builder_name { #builder_name::new() }
        }
    };

    TokenStream::from(expanded)
}
