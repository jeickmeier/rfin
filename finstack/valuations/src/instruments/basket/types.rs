//! Generic basket instrument for ETFs and equity/bond baskets.
//!
//! This module provides a unified basket instrument that can handle various asset types
//! including equities, bonds, ETFs, and other instruments by leveraging existing
//! pricing infrastructure.

use crate::instruments::traits::{Attributable, Attributes, Instrument, Priceable};
use finstack_core::prelude::*;
use finstack_core::types::{id::IndexId, InstrumentId};
use finstack_core::{dates::Frequency, F};
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
        price_id: String,
        /// Type of asset for validation
        asset_type: AssetType,
    },
}

impl std::fmt::Debug for ConstituentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstituentReference::Instrument(instrument) => f
                .debug_struct("Instrument")
                .field("type", &instrument.instrument_type())
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
    fn deserialize<D>(_deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // For now, trait objects can't be deserialized, so we'll return an error
        Err(serde::de::Error::custom(
            "ConstituentReference with Instrument cannot be deserialized. Use MarketData reference instead.",
        ))
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

/// Replication method for the basket
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ReplicationMethod {
    /// Full physical replication of all constituents
    #[default]
    Physical,
    /// Sampling/optimized replication (subset of constituents)
    Sampling,
    /// Synthetic replication via derivatives
    Synthetic {
        /// Counterparty for synthetic exposure
        counterparty: Option<String>,
    },
}

/// Generic basket instrument supporting multiple asset types
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Basket {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Basket ticker symbol (if traded)
    pub ticker: Option<String>,
    /// Descriptive name
    pub name: String,
    /// Basket constituents
    pub constituents: Vec<BasketConstituent>,
    /// Total expense ratio (as decimal, e.g., 0.0025 = 0.25%)
    pub expense_ratio: F,
    /// Rebalancing frequency
    pub rebalance_freq: Frequency,
    /// Index being tracked (if applicable)
    pub tracking_index: Option<IndexId>,
    /// Creation unit size (shares per creation unit)
    pub creation_unit_size: F,
    /// Base currency of the basket
    pub currency: Currency,
    /// Total shares outstanding
    pub shares_outstanding: Option<F>,
    /// Replication method
    pub replication: ReplicationMethod,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl Basket {
    // Builder provided by derive

    /// Calculate Net Asset Value per share
    pub fn nav(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        crate::instruments::basket::pricing::engine::BasketPricer::new().nav(self, context, as_of)
    }

    /// Calculate total basket value (without per-share division)
    pub fn basket_value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        crate::instruments::basket::pricing::engine::BasketPricer::new()
            .basket_value(self, context, as_of)
    }

    /// Calculate Net Asset Value per share using an explicit AUM.
    ///
    /// When constituents lack `units`, contributions are computed as
    /// `weight × AUM (in basket currency)`.
    pub fn nav_with_aum(&self, context: &MarketContext, as_of: Date, aum: Money) -> Result<Money> {
        crate::instruments::basket::pricing::engine::BasketPricer::new()
            .nav_with_aum(self, context, as_of, aum)
    }

    /// Calculate total basket value using an explicit AUM for weight-based constituents.
    pub fn basket_value_with_aum(
        &self,
        context: &MarketContext,
        as_of: Date,
        aum: Money,
    ) -> Result<Money> {
        crate::instruments::basket::pricing::engine::BasketPricer::new()
            .basket_value_with_aum(self, context, as_of, aum)
    }

    /// Calculate tracking error vs benchmark index
    pub fn tracking_error(
        &self,
        context: &MarketContext,
        benchmark_returns: &[(Date, F)],
        _as_of: Date,
    ) -> Result<F> {
        crate::instruments::basket::pricing::engine::BasketPricer::new().tracking_error(
            self,
            context,
            benchmark_returns,
            _as_of,
        )
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

    /// Calculate creation/redemption basket for ETF mechanics
    pub fn creation_basket(&self, _units: F) -> CreationRedemptionBasket {
        // For now, return a simple implementation
        // In practice, this would calculate the exact securities needed
        CreationRedemptionBasket {
            creation_basket: self.constituents.clone(),
            cash_component: Some(Money::new(0.0, self.currency)),
            transaction_cost: Money::new(0.02, self.currency), // 2 cents per share
        }
    }
}

/// Creation/redemption basket specification
#[derive(Clone, Debug)]
pub struct CreationRedemptionBasket {
    /// Securities required for creation
    pub creation_basket: Vec<BasketConstituent>,
    /// Cash component for fractional shares
    pub cash_component: Option<Money>,
    /// Transaction costs
    pub transaction_cost: Money,
}

// Implement traits manually to handle InstrumentId properly
impl Priceable for Basket {
    fn value(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        self.nav(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> Result<crate::results::ValuationResult> {
        let base_value = Priceable::value(self, curves, as_of)?;
        crate::instruments::helpers::build_with_metrics_dyn(
            self, curves, as_of, base_value, metrics,
        )
    }
}

impl Instrument for Basket {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "Basket"
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
}

impl Attributable for Basket {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basket_creation() {
        let basket = Basket {
            id: InstrumentId::new("TEST_BASKET"),
            ticker: Some("TEST".to_string()),
            name: "Test Basket".to_string(),
            constituents: vec![],
            expense_ratio: 0.001,
            rebalance_freq: Frequency::quarterly(),
            tracking_index: None,
            creation_unit_size: 50000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1000000.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        assert_eq!(basket.id.as_str(), "TEST_BASKET");
        assert_eq!(basket.ticker, Some("TEST".to_string()));
        assert_eq!(basket.expense_ratio, 0.001);
    }

    #[test]
    fn test_validate_weights() {
        let mut basket = Basket {
            id: InstrumentId::new("TEST"),
            ticker: None,
            name: "Test".to_string(),
            constituents: vec![
                BasketConstituent {
                    id: "CONST1".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "AAPL".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.6,
                    units: None,
                    ticker: Some("AAPL".to_string()),
                },
                BasketConstituent {
                    id: "CONST2".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "MSFT".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.4,
                    units: None,
                    ticker: Some("MSFT".to_string()),
                },
            ],
            expense_ratio: 0.001,
            rebalance_freq: Frequency::quarterly(),
            tracking_index: None,
            creation_unit_size: 50000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1000000.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        // Should pass with weights summing to 1.0
        assert!(basket.validate().is_ok());

        // Should fail with weights not summing to 1.0
        basket.constituents[0].weight = 0.8;
        assert!(basket.validate().is_err());
    }
}
