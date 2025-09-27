//! Declarative metric registry builder to eliminate registration boilerplate.
//!
//! This module provides a fluent interface for registering metrics with instruments,
//! replacing the repetitive per-instrument registration functions.

use crate::metrics::{MetricCalculator, MetricId, MetricRegistry};
use std::sync::Arc;

/// Builder for creating a metric registry with declarative syntax.
pub struct MetricRegistryBuilder {
    /// Pending registrations to be applied
    registrations: Vec<MetricRegistration>,
}

/// Internal structure for a single metric registration
struct MetricRegistration {
    metric_id: MetricId,
    calculator: Arc<dyn MetricCalculator>,
    instruments: Vec<&'static str>,
}

impl Default for MetricRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }
    
    /// Start registering a metric.
    pub fn metric(self, metric_id: MetricId) -> MetricBuilder {
        MetricBuilder {
            registry_builder: self,
            metric_id,
        }
    }
    
    /// Build the final MetricRegistry with all registrations applied.
    pub fn build(self) -> MetricRegistry {
        let mut registry = MetricRegistry::new();
        
        for registration in self.registrations {
            registry.register_metric(
                registration.metric_id,
                registration.calculator,
                &registration.instruments,
            );
        }
        
        registry
    }
    
    /// Add a registration (internal method)
    fn add_registration(
        mut self,
        metric_id: MetricId,
        calculator: Arc<dyn MetricCalculator>,
        instruments: Vec<&'static str>,
    ) -> Self {
        self.registrations.push(MetricRegistration {
            metric_id,
            calculator,
            instruments,
        });
        self
    }
}

/// Builder for configuring a single metric registration.
pub struct MetricBuilder {
    registry_builder: MetricRegistryBuilder,
    metric_id: MetricId,
}

impl MetricBuilder {
    /// Register this metric for a single instrument.
    pub fn for_instrument(self, instrument: &'static str) -> CalculatorBuilder {
        CalculatorBuilder {
            registry_builder: self.registry_builder,
            metric_id: self.metric_id,
            instruments: vec![instrument],
        }
    }
    
    /// Register this metric for multiple instruments.
    pub fn for_instruments(self, instruments: &[&'static str]) -> CalculatorBuilder {
        CalculatorBuilder {
            registry_builder: self.registry_builder,
            metric_id: self.metric_id,
            instruments: instruments.to_vec(),
        }
    }
}

/// Builder for specifying the calculator for a metric.
pub struct CalculatorBuilder {
    registry_builder: MetricRegistryBuilder,
    metric_id: MetricId,
    instruments: Vec<&'static str>,
}

impl CalculatorBuilder {
    /// Use a specific calculator instance.
    pub fn with_calculator<C: MetricCalculator + 'static>(self, calculator: C) -> MetricRegistryBuilder {
        self.registry_builder.add_registration(
            self.metric_id,
            Arc::new(calculator),
            self.instruments,
        )
    }
    
    /// Use a generic calculator parameterized by instrument type.
    /// 
    /// This is particularly useful for metrics like BucketedDv01 that have generic implementations.
    pub fn with_generic<C: MetricCalculator + Default + 'static>(self) -> MetricRegistryBuilder {
        self.registry_builder.add_registration(
            self.metric_id,
            Arc::new(C::default()),
            self.instruments,
        )
    }
}

/// Create the standard metric registry with all instrument metrics registered.
pub fn create_standard_registry() -> MetricRegistry {
    MetricRegistryBuilder::new()
        // Generic metrics used across multiple instruments
        .metric(MetricId::BucketedDv01)
            .for_instruments(&["Bond", "Deposit", "FRA"])  
            .with_generic::<crate::instruments::common::GenericBucketedDv01<crate::instruments::Bond>>()
        .metric(MetricId::BucketedDv01)
            .for_instruments(&["InterestRateSwap"])
            .with_generic::<crate::instruments::common::GenericBucketedDv01WithContext<crate::instruments::InterestRateSwap>>()
        
        // Bond-specific metrics
        .metric(MetricId::Accrued)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::AccruedInterestCalculator)
        .metric(MetricId::DirtyPrice)
            .for_instrument("Bond")  
            .with_calculator(crate::instruments::bond::metrics::DirtyPriceCalculator)
        .metric(MetricId::CleanPrice)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::CleanPriceCalculator)
        .metric(MetricId::Ytm)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::YtmCalculator)
        .metric(MetricId::DurationMac)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::MacaulayDurationCalculator)
        .metric(MetricId::DurationMod)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::ModifiedDurationCalculator)
        .metric(MetricId::Convexity)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::ConvexityCalculator)
        .metric(MetricId::Ytw)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::YtwCalculator)
        .metric(MetricId::Oas)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::OasCalculator)
        .metric(MetricId::ZSpread)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::ZSpreadCalculator)
        .metric(MetricId::ISpread)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::ISpreadCalculator)
        .metric(MetricId::DiscountMargin)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::DiscountMarginCalculator)
        .metric(MetricId::ASWPar)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::AssetSwapParCalculator)
        .metric(MetricId::ASWMarket)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::AssetSwapMarketCalculator)
        .metric(MetricId::Cs01)
            .for_instrument("Bond")
            .with_calculator(crate::instruments::bond::metrics::Cs01Calculator)
        
        // Deposit-specific metrics
        .metric(MetricId::Yf)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::YearFractionCalculator)
        .metric(MetricId::DfStart)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::DfStartCalculator)
        .metric(MetricId::DfEnd)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::DfEndCalculator)
        .metric(MetricId::DepositParRate)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::DepositParRateCalculator)
        .metric(MetricId::DfEndFromQuote)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::DfEndFromQuoteCalculator)
        .metric(MetricId::QuoteRate)
            .for_instrument("Deposit")
            .with_calculator(crate::instruments::deposit::metrics::QuoteRateCalculator)
        
        // FRA-specific metrics
        .metric(MetricId::custom("fra_pv"))
            .for_instrument("FRA")
            .with_calculator(crate::instruments::fra::metrics::FraPvCalculator)
        .metric(MetricId::Dv01)
            .for_instrument("FRA")
            .with_calculator(crate::instruments::fra::metrics::FraDv01Calculator)
        .metric(MetricId::ParRate)
            .for_instrument("FRA")
            .with_calculator(crate::instruments::fra::metrics::FraParRateCalculator)
        
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declarative_registry_builder() {
        // Test that the builder pattern works
        let registry = MetricRegistryBuilder::new()
            .metric(MetricId::Accrued)
                .for_instrument("Bond")
                .with_calculator(crate::instruments::bond::metrics::AccruedInterestCalculator)
            .build();
        
        // Verify the metric was registered
        assert!(registry.has_metric(MetricId::Accrued));
    }
    
    #[test]
    fn test_multiple_instruments() {
        // Test registering a metric for multiple instruments
        let registry = MetricRegistryBuilder::new()
            .metric(MetricId::Dv01)
                .for_instruments(&["Bond", "Deposit", "FRA"])
                .with_calculator(crate::instruments::bond::metrics::AccruedInterestCalculator) // dummy calc for test
            .build();
        
        assert!(registry.has_metric(MetricId::Dv01));
    }
}
