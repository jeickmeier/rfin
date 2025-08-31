//! Procedural-style macros for instruments
//!
//! This module provides derive-like macros for reducing boilerplate
//! in instrument implementations.

//! Note: The legacy `impl_priceable!` macro has been removed.

/// Generate standard Attributable implementation.
///
/// Requirements:
/// - Struct must have an `attributes: Attributes` field
#[macro_export]
macro_rules! impl_attributable {
    ($type:ident) => {
        impl $crate::instruments::traits::Attributable for $type {
            fn attributes(&self) -> &$crate::instruments::traits::Attributes {
                &self.attributes
            }

            fn attributes_mut(&mut self) -> &mut $crate::instruments::traits::Attributes {
                &mut self.attributes
            }
        }
    };
}

/// Generate builder pattern for an instrument.
///
/// Creates a builder struct with setter methods for all fields.
#[macro_export]
macro_rules! impl_builder {
    (
        $type:ident,
        $builder:ident,
        required: [$($req_field:ident: $req_type:ty),* $(,)?],
        optional: [$($opt_field:ident: $opt_type:ty),* $(,)?]
    ) => {
        #[derive(Default)]
        pub struct $builder {
            $($req_field: Option<$req_type>,)*
            $($opt_field: Option<$opt_type>,)*
        }

        impl $builder {
            pub fn new() -> Self {
                Self::default()
            }

            $(
                pub fn $req_field(mut self, value: $req_type) -> Self {
                    self.$req_field = Some(value);
                    self
                }
            )*

            $(
                pub fn $opt_field(mut self, value: $opt_type) -> Self {
                    self.$opt_field = Some(value);
                    self
                }
            )*

            pub fn build(self) -> finstack_core::Result<$type> {
                Ok($type {
                    $(
                        $req_field: self.$req_field
                            .ok_or_else(|| finstack_core::Error::from(
                                finstack_core::error::InputError::Invalid
                            ))?,
                    )*
                    $(
                        $opt_field: self.$opt_field,
                    )*
                    attributes: $crate::instruments::traits::Attributes::default(),
                })
            }
        }

        impl $type {
            pub fn builder() -> $builder {
                $builder::new()
            }
        }
    };
}
