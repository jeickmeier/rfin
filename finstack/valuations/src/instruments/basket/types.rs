//! Generic basket instrument for ETFs and equity/bond baskets.
//!
//! This module provides a unified basket instrument that can handle various asset types
//! including equities, bonds, ETFs, and other instruments by leveraging existing
//! pricing infrastructure.

use crate::instruments::common::traits::{Attributes, Instrument};
use finstack_core::prelude::*;
use finstack_core::types::{
    id::PriceId,
    InstrumentId,
};
use finstack_core::F;
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type of asset in the basket
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum AssetType {
    /// Equity security
    Equity,
    /// Fixed income security
    Bond,
    /// Exchange-traded fund
    ETF,
    /// Cash or cash equivalent
    Cash,
    /// Commodity
    Commodity,
    /// Derivative instrument
    Derivative,
}

/// Reference to a constituent asset in the basket
#[derive(Clone)]
pub enum ConstituentReference {
    /// Direct reference to an existing instrument (uses instrument's value() method)
    /// Note: Cannot be serialized due to trait object limitations
    Instrument(Arc<dyn Instrument + Send + Sync>),
    /// Market data reference for simple price lookups
    MarketData {
        /// Price identifier in MarketContext
        price_id: PriceId,
        /// Type of asset for validation
        asset_type: AssetType,
    },
}

impl std::fmt::Debug for ConstituentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstituentReference::Instrument(instrument) => f
                .debug_struct("Instrument")
                .field("type", &format!("{:?}", instrument.key()))
                .field("id", &instrument.id())
                .finish(),
            ConstituentReference::MarketData {
                price_id,
                asset_type,
            } => f
                .debug_struct("MarketData")
                .field("price_id", price_id)
                .field("asset_type", asset_type)
                .finish(),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for ConstituentReference {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ConstituentReference::Instrument(_) => {
                // For instruments, serialize as a placeholder since trait objects can't be serialized
                #[derive(Serialize)]
                struct InstrumentPlaceholder {
                    instrument_type: String,
                }
                let placeholder = InstrumentPlaceholder {
                    instrument_type: "instrument_reference".to_string(),
                };
                placeholder.serialize(serializer)
            }
            ConstituentReference::MarketData {
                price_id,
                asset_type,
            } => {
                #[derive(Serialize)]
                struct MarketDataRef<'a> {
                    price_id: &'a str,
                    asset_type: &'a AssetType,
                }
                let market_ref = MarketDataRef {
                    price_id: price_id.as_str(),
                    asset_type,
                };
                market_ref.serialize(serializer)
            }
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ConstituentReference {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MarketDataRef {
            price_id: PriceId,
            asset_type: AssetType,
        }

        let market_data = MarketDataRef::deserialize(deserializer)?;
        Ok(ConstituentReference::MarketData {
            price_id: market_data.price_id,
            asset_type: market_data.asset_type,
        })
    }
}

/// Individual constituent in a basket
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasketConstituent {
    /// Unique identifier for the constituent
    pub id: String,
    /// Reference to the underlying asset
    pub reference: ConstituentReference,
    /// Weight in the basket (as a fraction, e.g., 0.05 = 5%)
    pub weight: F,
    /// Number of units for physical replication (optional)
    pub units: Option<F>,
    /// Optional ticker symbol for reporting
    pub ticker: Option<String>,
}



/// Configuration for basket pricing behaviour.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasketPricingConfig {
    /// Day basis used for fee accrual (e.g., 365.0 or 365.25). Avoid hardcoding in logic.
    pub days_in_year: F,
    /// FX policy hint for conversions when constituent currency != basket currency.
    pub fx_policy: FxConversionPolicy,
}

impl Default for BasketPricingConfig {
    fn default() -> Self {
        Self {
            days_in_year: 365.25,
            fx_policy: FxConversionPolicy::CashflowDate,
        }
    }
}

/// Simplified basket instrument focused on pricing essentials.
///
/// This basket represents a collection of financial instruments or market data references
/// that can be valued as a portfolio. It focuses purely on pricing functionality without
/// ETF-specific operational features like creation/redemption mechanics.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Basket {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Basket constituents (the actual holdings)
    pub constituents: Vec<BasketConstituent>,
    /// Total expense ratio (as decimal, e.g., 0.0025 = 0.25%)
    /// This affects pricing through expense drag calculations
    pub expense_ratio: F,
    /// Base currency of the basket
    pub currency: Currency,
    /// Discount curve identifier for present value calculations
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
    /// Pricing configuration
    pub pricing_config: BasketPricingConfig,
}

impl Basket {
    // Builder provided by derive

    /// Create a new basket with custom pricing configuration.
    pub fn with_pricing_config(mut self, config: BasketPricingConfig) -> Self {
        self.pricing_config = config;
        self
    }

    /// Get a configured calculator for this basket.
    /// 
    /// This centralizes calculator creation and avoids duplication across
    /// metrics, pricers, and other components.
    pub fn calculator(&self) -> crate::instruments::basket::pricer::BasketCalculator {
        crate::instruments::basket::pricer::BasketCalculator::new(self.pricing_config.clone())
    }

    /// Get constituent by ID
    pub fn get_constituent(&self, id: &str) -> Option<&BasketConstituent> {
        self.constituents.iter().find(|c| c.id == id)
    }

    /// Get total number of constituents
    pub fn constituent_count(&self) -> usize {
        self.constituents.len()
    }

    /// Validate basket consistency (weights sum to ~1.0, currency consistency, etc.)
    pub fn validate(&self) -> Result<()> {
        // Check weight sum
        let total_weight: F = self.constituents.iter().map(|c| c.weight).sum();
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        }

        // Validate each constituent's currency compatibility would happen
        // during pricing through the existing instrument validation

        Ok(())
    }


}


// Implement traits manually to handle InstrumentId properly
impl Instrument for Basket {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    // === Pricing Methods ===

    fn value(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // For the Instrument trait, we use the calculator with default shares of 1.0
        // This represents the NAV per unit of the basket
        self.calculator().nav(self, curves, as_of, 1.0)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, curves, as_of, base_value, metrics,
        )
    }
}

// Implement HasDiscountCurve trait
impl crate::instruments::common::HasDiscountCurve for Basket {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basket_creation() {
        let basket = Basket {
            id: InstrumentId::new("TEST_BASKET"),
            constituents: vec![],
            expense_ratio: 0.001,
            currency: Currency::USD,
            discount_curve_id: "USD-OIS".into(),
            attributes: Attributes::new(),
            pricing_config: BasketPricingConfig::default(),
        };

        assert_eq!(basket.id.as_str(), "TEST_BASKET");
        assert_eq!(basket.expense_ratio, 0.001);
    }

    #[test]
    fn test_validate_weights() {
        let mut basket = Basket {
            id: InstrumentId::new("TEST"),
            constituents: vec![
                BasketConstituent {
                    id: "CONST1".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "AAPL".to_string().into(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.6,
                    units: None,
                    ticker: Some("AAPL".to_string()),
                },
                BasketConstituent {
                    id: "CONST2".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "MSFT".to_string().into(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.4,
                    units: None,
                    ticker: Some("MSFT".to_string()),
                },
            ],
            expense_ratio: 0.001,
            currency: Currency::USD,
            discount_curve_id: "USD-OIS".into(),
            attributes: Attributes::new(),
            pricing_config: BasketPricingConfig::default(),
        };

        // Should pass with weights summing to 1.0
        assert!(basket.validate().is_ok());

        // Should fail with weights not summing to 1.0
        basket.constituents[0].weight = 0.8;
        assert!(basket.validate().is_err());
    }
}
