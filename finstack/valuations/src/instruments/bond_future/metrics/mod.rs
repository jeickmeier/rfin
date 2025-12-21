//! Bond future metrics and risk calculations.
//!
//! This module provides risk metrics for bond futures, including:
//! - Contract DV01 (dollar value of a basis point) via parallel bump of discount curve
//! - Bucketed DV01 by tenor using key-rate sensitivities
//! - Theta (time decay)
//!
//! # DV01 Calculation
//!
//! Bond future DV01 is calculated by bumping the discount curve and observing the change
//! in the contract's NPV. The conversion factor scaling is handled automatically through
//! the pricing formula:
//!
//! - Model Price = CTD Clean Price / Conversion Factor
//! - NPV = (Quoted Price - Model Price) × Contract Size × Number of Contracts
//!
//! When the discount curve is bumped:
//! 1. CTD bond's clean price changes by Δ
//! 2. Model price changes by Δ / CF (conversion factor scales the sensitivity)
//! 3. NPV changes by -Δ / CF × Contract Size × Num Contracts
//! 4. DV01 = -ΔNPV / (1 basis point)
//!
//! This ensures that the contract DV01 correctly reflects the conversion factor scaling.
//!
//! # Examples
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::bond_future::BondFuture;
//! use finstack_valuations::metrics::{MetricId, MetricContext};
//!
//! // Create a bond future (see BondFuture docs for full example)
//! let future = BondFuture::ust_10y(...);
//!
//! // Calculate DV01 via the pricing registry
//! let metrics = future.price_with_metrics(
//!     &market,
//!     as_of,
//!     &[MetricId::Dv01, MetricId::BucketedDv01],
//! )?;
//!
//! let dv01 = metrics.metric("dv01").unwrap();
//! let bucketed = metrics.get_series(&MetricId::BucketedDv01).unwrap();
//! ```

use crate::metrics::MetricRegistry;

/// Register all bond future metrics to a registry.
///
/// Registers the following metrics:
/// - **Dv01**: Parallel DV01 (all tenor buckets bumped together)
/// - **BucketedDv01**: Key-rate DV01 by standard IR buckets (3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y)
/// - **Theta**: Time decay (universal metric)
///
/// Each metric is registered with the "BondFuture" instrument type to ensure
/// proper applicability filtering.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::metrics::MetricRegistry;
/// use finstack_valuations::instruments::bond_future::metrics::register_bond_future_metrics;
///
/// let mut registry = MetricRegistry::new();
/// register_bond_future_metrics(&mut registry);
/// ```
pub fn register_bond_future_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "BondFuture",
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::bond_future::BondFuture,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::bond_future::BondFuture,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::bond_future::BondFuture,
            >::default()),
        ]
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{MetricId, MetricRegistry};

    #[test]
    fn test_metrics_registration() {
        let mut registry = MetricRegistry::new();
        register_bond_future_metrics(&mut registry);

        // Verify metrics are registered for BondFuture
        let metrics = registry.metrics_for_instrument("BondFuture");
        
        // Verify DV01 is registered
        assert!(
            metrics.contains(&MetricId::Dv01),
            "DV01 metric should be registered for BondFuture"
        );

        // Verify Bucketed DV01 is registered
        assert!(
            metrics.contains(&MetricId::BucketedDv01),
            "BucketedDv01 metric should be registered for BondFuture"
        );

        // Verify Theta is registered
        assert!(
            metrics.contains(&MetricId::Theta),
            "Theta metric should be registered for BondFuture"
        );
    }
}
