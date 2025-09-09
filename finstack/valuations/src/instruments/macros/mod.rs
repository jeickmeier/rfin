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

/// Generate InstrumentLike implementation.
///
/// Requirements:
/// - Struct must have an `id: String` field
/// - Must already implement Priceable and Attributable
#[macro_export]
macro_rules! impl_instrument_like {
    ($type:ident, $type_name:literal) => {
        impl $crate::instruments::traits::InstrumentLike for $type {
            fn id(&self) -> &str {
                &self.id
            }

            fn instrument_type(&self) -> &'static str {
                $type_name
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn clone_box(&self) -> Box<dyn $crate::instruments::traits::InstrumentLike> {
                Box::new(self.clone())
            }
        }
    };
}

/// Generate a full instrument implementation:
/// - Attributable
/// - InstrumentLike
/// - Priceable: value (via pv closure), price_with_metrics
#[macro_export]
macro_rules! impl_instrument {
    (
        $type:ident, $type_name:literal,
        pv = |$s:ident, $curves:ident, $as_of:ident| $pv_expr:expr $(,)?
    ) => {
        // Attributes
        impl_attributable!($type);

        // InstrumentLike implementation
        impl_instrument_like!($type, $type_name);

        // Pricing surface (PV + metrics)
        impl $crate::instruments::traits::Priceable for $type {
            fn value(
                &self,
                curves: &finstack_core::market_data::MarketContext,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<finstack_core::money::Money> {
                let $s = self;
                let $curves = curves;
                let $as_of = as_of;
                $pv_expr
            }

            fn price_with_metrics(
                &self,
                curves: &finstack_core::market_data::MarketContext,
                as_of: finstack_core::dates::Date,
                metrics: &[$crate::metrics::MetricId],
            ) -> finstack_core::Result<$crate::results::ValuationResult> {
                let base_value = self.value(curves, as_of)?;
                $crate::instruments::build_with_metrics_dyn(
                    self, curves, as_of, base_value, metrics,
                )
            }
        }
    };
}

/// Schedule-PV variant that uses CashflowProvider + Discountable,
/// and reads `disc_id` and day-count field names from the instrument.
///
/// Usage:
/// impl_instrument_schedule_pv!(Type, "TypeName", disc_field: disc_id, dc_field: dc);
#[macro_export]
macro_rules! impl_instrument_schedule_pv {
    (
        $type:ident, $type_name:literal,
        disc_field: $disc:ident,
        dc_field: $dc:ident
    ) => {
        $crate::impl_instrument!(
            $type,
            $type_name,
            pv = |s, curves, as_of| {
                use $crate::cashflow::traits::CashflowProvider;
                use $crate::instruments::fixed_income::discountable::Discountable;
                // Use trait object to avoid monomorphization
                let flows = CashflowProvider::build_schedule(s, curves, as_of)?;
                let disc = curves.disc(s.$disc)?;
                flows.npv(&*disc, disc.base_date(), s.$dc)
            }
        );
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
