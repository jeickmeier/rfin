//! Procedural-style macros for instruments
//!
//! This module provides derive-like macros for reducing boilerplate 
//! in instrument implementations.

/// Generate standard Priceable implementation for an instrument.
/// 
/// Requirements:
/// - Struct must have an `id: String` field
/// - Struct must implement Clone
/// - Must specify which metrics are standard for this instrument type
#[macro_export]
macro_rules! impl_priceable {
    ($type:ident, [$($metric:expr),* $(,)?]) => {
        impl $crate::traits::Priceable for $type {
            fn value(&self, curves: &finstack_core::market_data::multicurve::CurveSet, as_of: finstack_core::dates::Date) -> finstack_core::Result<finstack_core::money::Money> {
                use $crate::pricing::discountable::Discountable;
                // Default implementation - instruments should override if needed
                let flows = <Self as $crate::traits::CashflowProvider>::build_schedule(self, curves, as_of)?;
                let disc = curves.discount(self.disc_id)?;
                flows.npv(&*disc, disc.base_date(), self.day_count)
            }
            
            fn price_with_metrics(
                &self, 
                curves: &finstack_core::market_data::multicurve::CurveSet, 
                as_of: finstack_core::dates::Date, 
                metrics: &[$crate::metrics::MetricId]
            ) -> finstack_core::Result<$crate::pricing::result::ValuationResult> {
                use $crate::metrics::{MetricContext, standard_registry};
                use std::sync::Arc;
                
                // Compute base value
                let base_value = self.value(curves, as_of)?;
                
                // Create metric context with self wrapped in Instrument enum
                let instrument: $crate::instruments::Instrument = 
                    $crate::instruments::Instrument::$type(self.clone());
                let mut context = MetricContext::new(
                    Arc::new(instrument),
                    Arc::new(curves.clone()),
                    as_of,
                    base_value,
                );
                
                // Get registry and compute requested metrics
                let registry = standard_registry();
                let metric_measures = registry.compute(metrics, &mut context)?;
                
                // Convert MetricId keys to String keys for ValuationResult
                let measures: hashbrown::HashMap<String, finstack_core::F> = metric_measures
                    .into_iter()
                    .map(|(k, v)| (k.as_str().to_string(), v))
                    .collect();
                
                // Create result
                let mut result = $crate::pricing::result::ValuationResult::stamped(self.id.clone(), as_of, base_value);
                result.measures = measures;
                
                Ok(result)
            }
            
            fn price(&self, curves: &finstack_core::market_data::multicurve::CurveSet, as_of: finstack_core::dates::Date) -> finstack_core::Result<$crate::pricing::result::ValuationResult> {
                let standard_metrics = vec![$($metric),*];
                self.price_with_metrics(curves, as_of, &standard_metrics)
            }
        }
    };
}

/// Generate standard Attributable implementation.
/// 
/// Requirements:
/// - Struct must have an `attributes: Attributes` field
#[macro_export]
macro_rules! impl_attributable {
    ($type:ident) => {
        impl $crate::traits::Attributable for $type {
            fn attributes(&self) -> &$crate::traits::Attributes {
                &self.attributes
            }
            
            fn attributes_mut(&mut self) -> &mut $crate::traits::Attributes {
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
                    attributes: $crate::traits::Attributes::default(),
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

/// Combined macro that implements all common patterns for an instrument.
/// 
/// This macro:
/// 1. Implements Priceable with standard metrics
/// 2. Implements Attributable 
/// 3. Generates a builder pattern
/// 4. Adds conversion to/from Instrument enum
#[macro_export]
macro_rules! instrument {
    (
        $type:ident {
            metrics: [$($metric:expr),* $(,)?],
            required: [$($req_field:ident: $req_type:ty),* $(,)?],
            optional: [$($opt_field:ident: $opt_type:ty),* $(,)?]
        }
    ) => {
        // Generate builder
        paste::paste! {
            $crate::impl_builder!(
                $type,
                [<$type Builder>],
                required: [$($req_field: $req_type),*],
                optional: [$($opt_field: $opt_type),*]
            );
        }
        
        // Implement Priceable
        $crate::impl_priceable!($type, [$($metric),*]);
        
        // Implement Attributable
        $crate::impl_attributable!($type);
        
        // Add conversion to both Instrument enums
        impl From<$type> for $crate::instruments::unified::Instrument {
            fn from(value: $type) -> Self {
                $crate::instruments::unified::Instrument::$type(value)
            }
        }
        
        impl From<$type> for $crate::instruments::Instrument {
            fn from(value: $type) -> Self {
                $crate::instruments::Instrument::$type(value)
            }
        }
        
        impl std::convert::TryFrom<$crate::instruments::unified::Instrument> for $type {
            type Error = finstack_core::Error;
            
            fn try_from(value: $crate::instruments::unified::Instrument) -> finstack_core::Result<Self> {
                match value {
                    $crate::instruments::unified::Instrument::$type(v) => Ok(v),
                    _ => Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
                }
            }
        }
        
        impl std::convert::TryFrom<$crate::instruments::Instrument> for $type {
            type Error = finstack_core::Error;
            
            fn try_from(value: $crate::instruments::Instrument) -> finstack_core::Result<Self> {
                match value {
                    $crate::instruments::Instrument::$type(v) => Ok(v),
                    _ => Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
                }
            }
        }
    };
}
