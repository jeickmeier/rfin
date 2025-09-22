//! Procedural-style macros for instruments
//!
//! This module provides derive-like macros for reducing boilerplate
//! in instrument implementations.

/// Generate standard Attributable implementation.
///
/// Requirements:
/// - Struct must have an `attributes: Attributes` field
#[macro_export]
macro_rules! impl_attributable {
    ($type:ident) => {
        impl $crate::instruments::common::traits::Attributable for $type {
            fn attributes(&self) -> &$crate::instruments::common::traits::Attributes {
                &self.attributes
            }

            fn attributes_mut(&mut self) -> &mut $crate::instruments::common::traits::Attributes {
                &mut self.attributes
            }
        }
    };
}

/// Generate a full instrument implementation:
/// - Attributable
/// - Instrument (including pricing methods)
#[macro_export]
macro_rules! impl_instrument {
    (
        $type:ident, $type_name:literal,
        pv = |$s:ident, $curves:ident, $as_of:ident| $pv_expr:expr $(,)?
    ) => {
        // Attributes
        impl_attributable!($type);

        // Unified Instrument implementation with pricing
        impl $crate::instruments::common::traits::Instrument for $type {
            #[inline]
            fn id(&self) -> &str {
                self.id.as_str()
            }

            #[inline]
            fn instrument_type(&self) -> &'static str {
                $type_name
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
                use $crate::cashflow::traits::CashflowProvider;
                use $crate::instruments::common::discountable::Discountable;
                // Use trait object to avoid monomorphization
                let flows = CashflowProvider::build_schedule(s, curves, as_of)?;
                let disc = curves.get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(<str as ::core::convert::AsRef<str>>::as_ref(s.$disc.as_ref()))?;
                // Import not needed here; types expose required methods
                flows.npv(&*disc, disc.base_date(), s.$dc)
            }
        );
    };
}

/// Generate builder pattern for an instrument (legacy version).
///
/// Creates a builder struct with setter methods for all fields.
/// This is kept for backwards compatibility - prefer `impl_enhanced_builder!` for new code.
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
                    attributes: $crate::instruments::common::traits::Attributes::default(),
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

/// Enhanced builder macro that supports parameter groups and compile-time safety.
///
/// This macro generates builders with:
/// - Required core parameters (compile-time checked)
/// - Parameter groups for logical collections
/// - Optional individual fields for flexibility
/// - Convenience methods for common patterns
#[macro_export]
macro_rules! impl_enhanced_builder {
    (
        $type:ident,
        $builder:ident,
        core: [$($core_field:ident: $core_type:ty),* $(,)?],
        $(groups: [$($group_field:ident: $group_type:ty),* $(,)?],)?
        $(optional: [$($opt_field:ident: $opt_type:ty),* $(,)?])?
    ) => {
        /// Enhanced builder with parameter groups and compile-time safety
        #[derive(Default)]
        pub struct $builder {
            // Core required fields
            $($core_field: Option<$core_type>,)*
            // Parameter groups (if any)
            $($($group_field: Option<$group_type>,)*)?
            // Optional individual fields (if any)
            $($($opt_field: Option<$opt_type>,)*)?
        }

        impl $builder {
            /// Create a new builder
            pub fn new() -> Self {
                Self::default()
            }

            // Core field setters (required)
            $(
                pub fn $core_field(mut self, value: $core_type) -> Self {
                    self.$core_field = Some(value);
                    self
                }
            )*

            // Parameter group setters (if any)
            $($(
                pub fn $group_field(mut self, value: $group_type) -> Self {
                    self.$group_field = Some(value);
                    self
                }
            )*)?

            // Optional field setters (if any)
            $($(
                pub fn $opt_field(mut self, value: $opt_type) -> Self {
                    self.$opt_field = Some(value);
                    self
                }
            )*)?

            /// Validate that all required fields are present
            ///
            /// This method provides compile-time-like checking at runtime,
            /// giving clear error messages about missing required fields.
            fn validate_required(&self) -> finstack_core::Result<()> {
                $(
                    if self.$core_field.is_none() {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid
                        ));
                    }
                )*
                Ok(())
            }
        }

        impl $type {
            /// Create a new enhanced builder
            pub fn builder() -> $builder {
                $builder::new()
            }
        }
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

/// Macro for generating builder methods that work with parameter groups.
///
/// Provides commonly needed builder enhancement methods.
#[macro_export]
macro_rules! impl_builder_enhancements {
    ($builder:ident) => {
        impl $builder {
            /// Quick setup for USD market standard parameters
            pub fn usd_standard(
                mut self,
                _disc_id: impl Into<finstack_core::types::CurveId>,
            ) -> Self {
                self.schedule_params =
                    Some($crate::cashflow::builder::ScheduleParams::usd_standard());
                self
            }

            /// Quick setup for EUR market standard parameters
            pub fn eur_standard(
                mut self,
                _disc_id: impl Into<finstack_core::types::CurveId>,
            ) -> Self {
                self.schedule_params =
                    Some($crate::cashflow::builder::ScheduleParams::eur_standard());
                self
            }

            /// Add dates for instruments with start/end dates (legacy helper)
            pub fn date_range(
                mut self,
                _start: finstack_core::dates::Date,
                _end: finstack_core::dates::Date,
            ) -> Self {
                // No-op placeholder retained for compatibility; prefer explicit start/end setters.
                self
            }

            /// Add date range from tenor (legacy helper; prefer explicit start/end)
            pub fn tenor(
                mut self,
                _start: finstack_core::dates::Date,
                _tenor_years: finstack_core::F,
            ) -> Self {
                // No-op placeholder retained for compatibility.
                self
            }
        }
    };
}
