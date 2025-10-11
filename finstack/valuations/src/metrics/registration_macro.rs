//! Declarative macro for simplifying metric registration.
//!
//! This module provides a macro to reduce boilerplate in instrument metric registration.

/// Simplifies metric registration by providing a declarative syntax.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::register_metrics;
///
/// register_metrics! {
///     registry: my_registry,
///     instrument: "Bond",
///     metrics: [
///         (Accrued, AccruedInterestCalculator),
///         (Ytm, YtmCalculator),
///         (DurationMod, ModifiedDurationCalculator),
///     ]
/// }
/// ```
///
/// This expands to:
/// ```rust,ignore
/// {
///     use crate::metrics::MetricId;
///     use std::sync::Arc;
///     my_registry.register_metric(MetricId::Accrued, Arc::new(AccruedInterestCalculator), &["Bond"]);
///     my_registry.register_metric(MetricId::Ytm, Arc::new(YtmCalculator), &["Bond"]);
///     my_registry.register_metric(MetricId::DurationMod, Arc::new(ModifiedDurationCalculator), &["Bond"]);
/// }
/// ```
#[macro_export]
macro_rules! register_metrics {
    (
        registry: $registry:expr,
        instrument: $instrument:expr,
        metrics: [
            $(($metric_id:ident, $calculator:expr)),* $(,)?
        ]
    ) => {{
        use $crate::metrics::MetricId;
        use std::sync::Arc;
        
        $(
            $registry.register_metric(
                MetricId::$metric_id,
                Arc::new($calculator),
                &[$instrument],
            );
        )*
    }};
}

#[cfg(test)]
mod tests {
    use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
    use finstack_core::Result;

    struct DummyCalculator;
    impl MetricCalculator for DummyCalculator {
        fn calculate(&self, _context: &mut MetricContext) -> Result<f64> {
            Ok(42.0)
        }
    }

    #[test]
    fn test_register_metrics_macro() {
        let mut registry = MetricRegistry::new();
        
        register_metrics! {
            registry: registry,
            instrument: "TestInstrument",
            metrics: [
                (Accrued, DummyCalculator),
                (Ytm, DummyCalculator),
            ]
        }

        // Verify metrics were registered (basic smoke test)
        assert!(registry.is_applicable(&MetricId::Accrued, "TestInstrument"));
        assert!(registry.is_applicable(&MetricId::Ytm, "TestInstrument"));
    }
}
