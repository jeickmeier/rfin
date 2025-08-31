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

/// Generate a full instrument implementation:
/// - Attributable
/// - From<T> / TryFrom<Instrument>
/// - Priceable: value (via pv closure), price_with_metrics, price (via metrics closure)
#[macro_export]
macro_rules! impl_instrument {
    (
        $type:ident, $variant:ident,
        pv = |$s:ident, $curves:ident, $as_of:ident| $pv_expr:expr,
        metrics = |$ms:ident| $metrics_expr:expr $(,)?
    ) => {
        // Attributes
        impl_attributable!($type);

        // Conversions to/from unified Instrument
        impl From<$type> for $crate::instruments::Instrument {
            fn from(value: $type) -> Self {
                $crate::instruments::Instrument::$variant(value)
            }
        }

        impl ::std::convert::TryFrom<$crate::instruments::Instrument> for $type {
            type Error = finstack_core::Error;
            fn try_from(value: $crate::instruments::Instrument) -> finstack_core::Result<Self> {
                match value {
                    $crate::instruments::Instrument::$variant(v) => Ok(v),
                    _ => Err(finstack_core::Error::from(
                        finstack_core::error::InputError::Invalid,
                    )),
                }
            }
        }

        // Pricing surface (PV + metrics)
        impl $crate::instruments::traits::Priceable for $type {
            fn value(
                &self,
                curves: &finstack_core::market_data::multicurve::CurveSet,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<finstack_core::money::Money> {
                let $s = self;
                let $curves = curves;
                let $as_of = as_of;
                $pv_expr
            }

            fn price_with_metrics(
                &self,
                curves: &finstack_core::market_data::multicurve::CurveSet,
                as_of: finstack_core::dates::Date,
                metrics: &[ $crate::metrics::MetricId ],
            ) -> finstack_core::Result<$crate::results::ValuationResult> {
                let base_value = self.value(curves, as_of)?;
                $crate::instruments::build_with_metrics(self.clone(), curves, as_of, base_value, metrics)
            }

            fn price(
                &self,
                curves: &finstack_core::market_data::multicurve::CurveSet,
                as_of: finstack_core::dates::Date,
            ) -> finstack_core::Result<$crate::results::ValuationResult> {
                let $ms = self;
                let standard_metrics: ::std::vec::Vec<$crate::metrics::MetricId> = { $metrics_expr };
                self.price_with_metrics(curves, as_of, &standard_metrics)
            }
        }
    };
}

/// Schedule-PV variant that uses CashflowProvider + Discountable,
/// and reads `disc_id` and day-count field names from the instrument.
///
/// Usage:
/// impl_instrument_schedule_pv!(Type, EnumVariant, disc_field: disc_id, dc_field: dc, metrics = |s| expr);
#[macro_export]
macro_rules! impl_instrument_schedule_pv {
    (
        $type:ident, $variant:ident,
        disc_field: $disc:ident,
        dc_field: $dc:ident
        $(, metrics = |$ms:ident| $metrics_expr:expr)?
    ) => {
        $crate::impl_instrument!(
            $type, $variant,
            pv = |s, curves, as_of| {
                use $crate::instruments::fixed_income::discountable::Discountable;
                let flows = <$type as $crate::cashflow::traits::CashflowProvider>::build_schedule(s, curves, as_of)?;
                let disc = curves.discount(s.$disc)?;
                flows.npv(&*disc, disc.base_date(), s.$dc)
            },
            metrics = |mself| {
                impl_instrument_schedule_pv!(@metrics_body mself $(, metrics = |$ms| $metrics_expr)? )
            }
        );
    };

    (@metrics_body $mself:ident, metrics = |$ms:ident| $metrics_expr:expr) => {{
        let $ms = $mself;
        $metrics_expr
    }};

    (@metrics_body $mself:ident) => {{
        ::std::vec::Vec::new()
    }};
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
