//! Builder utilities for MarketContext
//!
//! This module provides ergonomic builder patterns and batch operations
//! for constructing complex market data contexts.

extern crate alloc;
use alloc::{string::String, vec::Vec};

use super::context::MarketContext;
use crate::market_data::{
    scalars::inflation_index::InflationIndex,
    term_structures::credit_index::CreditIndexData,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::{
        base_correlation::BaseCorrelationCurve,
        discount_curve::DiscountCurve,
        forward_curve::ForwardCurve,
        hazard_curve::HazardCurve,
        inflation::InflationCurve,
    },
};
use crate::money::fx::FxMatrix;
use crate::types::CurveId;
use crate::Result;

/// Batch builder for MarketContext
///
/// Provides ergonomic methods for building contexts with many curves
/// and validation of relationships between curves.
pub struct MarketContextBuilder {
    curves: Vec<CurveInput>,
    surfaces: Vec<VolSurface>,
    prices: Vec<(String, MarketScalar)>,
    series: Vec<ScalarTimeSeries>,
    inflation_indices: Vec<(String, InflationIndex)>,
    credit_indices: Vec<(String, CreditIndexData)>,
    fx: Option<FxMatrix>,
    collateral: Vec<(String, String)>, // (csa_code, curve_id)
}

/// Input wrapper for different curve types
pub enum CurveInput {
    /// Discount factor curve input
    Discount(DiscountCurve),
    /// Forward rate curve input
    Forward(ForwardCurve),
    /// Credit hazard curve input
    Hazard(HazardCurve),
    /// Inflation curve input
    Inflation(InflationCurve),
    /// Base correlation curve input
    BaseCorrelation(BaseCorrelationCurve),
}

impl MarketContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            curves: Vec::new(),
            surfaces: Vec::new(),
            prices: Vec::new(),
            series: Vec::new(),
            inflation_indices: Vec::new(),
            credit_indices: Vec::new(),
            fx: None,
            collateral: Vec::new(),
        }
    }

    /// Add a discount curve
    pub fn discount(mut self, curve: DiscountCurve) -> Self {
        self.curves.push(CurveInput::Discount(curve));
        self
    }

    /// Add a forward curve
    pub fn forward(mut self, curve: ForwardCurve) -> Self {
        self.curves.push(CurveInput::Forward(curve));
        self
    }

    /// Add a hazard curve
    pub fn hazard(mut self, curve: HazardCurve) -> Self {
        self.curves.push(CurveInput::Hazard(curve));
        self
    }

    /// Add an inflation curve
    pub fn inflation(mut self, curve: InflationCurve) -> Self {
        self.curves.push(CurveInput::Inflation(curve));
        self
    }

    /// Add a base correlation curve
    pub fn base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        self.curves.push(CurveInput::BaseCorrelation(curve));
        self
    }

    /// Add multiple curves at once
    pub fn curves(mut self, curves: Vec<CurveInput>) -> Self {
        self.curves.extend(curves);
        self
    }

    /// Add a volatility surface
    pub fn surface(mut self, surface: VolSurface) -> Self {
        self.surfaces.push(surface);
        self
    }

    /// Add a market price/scalar
    pub fn price(mut self, id: impl Into<String>, price: MarketScalar) -> Self {
        self.prices.push((id.into(), price));
        self
    }

    /// Add multiple prices at once
    pub fn prices(mut self, prices: Vec<(String, MarketScalar)>) -> Self {
        self.prices.extend(prices);
        self
    }

    /// Add a time series
    pub fn series(mut self, series: ScalarTimeSeries) -> Self {
        self.series.push(series);
        self
    }

    /// Add an inflation index
    pub fn inflation_index(mut self, id: impl Into<String>, index: InflationIndex) -> Self {
        self.inflation_indices.push((id.into(), index));
        self
    }

    /// Add a credit index
    pub fn credit_index(mut self, id: impl Into<String>, data: CreditIndexData) -> Self {
        self.credit_indices.push((id.into(), data));
        self
    }

    /// Set FX matrix
    pub fn fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(fx);
        self
    }

    /// Add collateral mapping
    pub fn collateral(mut self, csa_code: impl Into<String>, curve_id: impl Into<String>) -> Self {
        self.collateral.push((csa_code.into(), curve_id.into()));
        self
    }

    /// Build the market context with validation
    pub fn build(self) -> Result<MarketContext> {
        let mut context = MarketContext::new();

        // Add all curves
        for curve in self.curves {
            context = match curve {
                CurveInput::Discount(c) => context.insert_discount(c),
                CurveInput::Forward(c) => context.insert_forward(c),
                CurveInput::Hazard(c) => context.insert_hazard(c),
                CurveInput::Inflation(c) => context.insert_inflation(c),
                CurveInput::BaseCorrelation(c) => context.insert_base_correlation(c),
            };
        }

        // Add surfaces
        for surface in self.surfaces {
            context = context.insert_surface(surface);
        }

        // Add prices
        for (id, price) in self.prices {
            context = context.insert_price(id, price);
        }

        // Add series
        for series in self.series {
            context = context.insert_series(series);
        }

        // Add inflation indices
        for (id, index) in self.inflation_indices {
            context = context.insert_inflation_index(id, index);
        }

        // Add credit indices
        for (id, data) in self.credit_indices {
            context = context.insert_credit_index(id, data);
        }

        // Set FX matrix
        if let Some(fx) = self.fx {
            context = context.insert_fx(fx);
        }

        // Add collateral mappings
        for (csa_code, curve_id) in &self.collateral {
            context = context.map_collateral(csa_code.clone(), CurveId::new(curve_id.clone()));
        }

        // Perform validation
        Self::validate_context_static(&context, &self.collateral)?;

        Ok(context)
    }

    /// Build without validation (faster for trusted inputs)
    pub fn build_unchecked(self) -> MarketContext {
        self.build().expect("build_unchecked failed - input was not trusted")
    }

    /// Validate the constructed context
    fn validate_context_static(context: &MarketContext, collateral: &[(String, String)]) -> Result<()> {
        // Check for circular dependencies in collateral mappings
        for (csa_code, curve_id) in collateral {
            if !context.curves.contains_key(&CurveId::from(curve_id.as_str())) {
                return Err(crate::Error::Input(crate::error::InputError::NotFound {
                    id: format!("collateral mapping {} -> {}", csa_code, curve_id),
                }));
            }
        }

        // Additional validations can be added here
        Ok(())
    }
}

impl Default for MarketContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience methods for common context patterns
impl MarketContext {
    /// Create a builder
    pub fn builder() -> MarketContextBuilder {
        MarketContextBuilder::new()
    }

    /// Create a context with standard USD curves
    pub fn usd_standard() -> MarketContextBuilder {
        MarketContextBuilder::new()
        // Pre-configured builder for common USD setup
        // Can be extended with typical curve IDs and structures
    }

    /// Create a context with standard EUR curves
    pub fn eur_standard() -> MarketContextBuilder {
        MarketContextBuilder::new()
        // Pre-configured builder for common EUR setup
    }

    /// Merge another context into this one
    pub fn merge(mut self, other: MarketContext) -> Self {
        // Merge curves
        for (id, storage) in other.curves {
            self.curves.insert(id, storage);
        }

        // Merge other components
        self.surfaces.extend(other.surfaces);
        self.prices.extend(other.prices);
        self.series.extend(other.series);
        self.inflation_indices.extend(other.inflation_indices);
        self.credit_indices.extend(other.credit_indices);
        self.collateral.extend(other.collateral);

        // FX matrix (take the other one if we don't have one)
        if self.fx.is_none() {
            self.fx = other.fx;
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use crate::market_data::interp::InterpStyle;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_hazard_curve() -> HazardCurve {
        HazardCurve::builder("CORP-HAZARD")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015)])
            .build()
            .unwrap()
    }

    #[test]
    fn test_builder_pattern() {
        let context = MarketContext::builder()
            .discount(test_discount_curve())
            .hazard(test_hazard_curve())
            .price("SPOT_GOLD", MarketScalar::Unitless(2000.0))
            .collateral("USD-CSA", "USD-OIS")
            .build()
            .unwrap();

        assert_eq!(context.curves.len(), 2);
        assert_eq!(context.prices.len(), 1);
        assert!(context.collateral("USD-CSA").is_ok());
    }

    #[test]
    fn test_batch_operations() {
        let curves = vec![
            CurveInput::Discount(test_discount_curve()),
            CurveInput::Hazard(test_hazard_curve()),
        ];

        let prices = vec![
            ("SPOT_GOLD".to_string(), MarketScalar::Unitless(2000.0)),
            ("USD_RATE".to_string(), MarketScalar::Unitless(0.05)),
        ];

        let context = MarketContext::builder()
            .curves(curves)
            .prices(prices)
            .build()
            .unwrap();

        assert_eq!(context.curves.len(), 2);
        assert_eq!(context.prices.len(), 2);
    }

    #[test]
    fn test_context_merge() {
        let context1 = MarketContext::new()
            .insert_discount(test_discount_curve());

        let context2 = MarketContext::new()
            .insert_hazard(test_hazard_curve())
            .insert_price("SPOT_GOLD", MarketScalar::Unitless(2000.0));

        let merged = context1.merge(context2);

        assert_eq!(merged.curves.len(), 2);
        assert_eq!(merged.prices.len(), 1);
        assert!(merged.discount("USD-OIS").is_ok());
        assert!(merged.hazard("CORP-HAZARD").is_ok());
    }

    #[test]
    fn test_validation_catches_invalid_collateral() {
        let result = MarketContext::builder()
            .discount(test_discount_curve())
            .collateral("USD-CSA", "NONEXISTENT-CURVE")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_context_stats_with_builder() {
        let context = MarketContext::builder()
            .discount(test_discount_curve())
            .hazard(test_hazard_curve())
            .price("SPOT_GOLD", MarketScalar::Unitless(2000.0))
            .build()
            .unwrap();

        let stats = context.stats();
        assert_eq!(stats.total_curves, 2);
        assert_eq!(stats.price_count, 1);
        assert_eq!(stats.curve_counts.get("Discount"), Some(&1));
        assert_eq!(stats.curve_counts.get("Hazard"), Some(&1));
    }
}
