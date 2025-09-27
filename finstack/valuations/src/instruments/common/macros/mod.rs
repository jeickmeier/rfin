//! Procedural-style macros for instruments
//!
//! This module provides derive-like macros for reducing boilerplate
//! in instrument implementations.

/// Generate a full instrument implementation:
/// - Instrument (including attribute accessors and pricing methods)
#[macro_export]
macro_rules! impl_instrument {
    (
        $type:ident, $type_name:literal,
        pv = |$s:ident, $curves:ident, $as_of:ident| $pv_expr:expr $(,)?
    ) => {
        // Unified Instrument implementation with pricing
        impl $crate::instruments::common::traits::Instrument for $type {
            #[inline]
            fn id(&self) -> &str {
                self.id.as_str()
            }

            #[inline]
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            #[inline]
            fn attributes(&self) -> &$crate::instruments::common::traits::Attributes {
                &self.attributes
            }

            #[inline]
            fn attributes_mut(&mut self) -> &mut $crate::instruments::common::traits::Attributes {
                &mut self.attributes
            }

            #[inline]
            fn clone_box(&self) -> Box<dyn $crate::instruments::common::traits::Instrument> {
                Box::new(self.clone())
            }


            // === Pricing Methods ===

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
                $crate::instruments::common::helpers::build_with_metrics_dyn(
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
                // Route through monomorphized helper to reduce dynamic dispatch on hot path
                $crate::instruments::common::helpers::schedule_pv_impl(
                    s, curves, as_of, &s.$disc, s.$dc,
                )
            }
        );
    };
}

/// Macro for generating convenience constructor methods.
///
/// Creates static methods for common instrument patterns, reducing the need
/// for builder pattern in simple cases.
#[macro_export]
macro_rules! impl_convenience_constructors {
    (
        $type:ident {
            $(
                $method:ident($($param:ident: $param_type:ty),* $(,)?) => $constructor:expr
            ),* $(,)?
        }
    ) => {
        impl $type {
            $(
                /// Convenience constructor
                pub fn $method($($param: $param_type),*) -> Self {
                    $constructor
                }
            )*
        }
    };
}
