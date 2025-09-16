//! Builder for basket instruments.

use super::types::{AssetType, Basket, BasketConstituent, ConstituentReference, ReplicationMethod};
use crate::instruments::equity::Equity;
use crate::instruments::fixed_income::bond::Bond;
use crate::instruments::traits::{Attributes, Instrument};
use finstack_core::prelude::*;
use finstack_core::types::{id::IndexId, InstrumentId};
use finstack_core::{dates::Frequency, Error, Result, F};
use std::sync::Arc;

/// Builder for basket instruments
pub struct BasketBuilder {
    id: Option<InstrumentId>,
    ticker: Option<String>,
    name: Option<String>,
    constituents: Vec<BasketConstituent>,
    expense_ratio: F,
    rebalance_freq: Frequency,
    tracking_index: Option<IndexId>,
    creation_unit_size: F,
    currency: Currency,
    shares_outstanding: Option<F>,
    replication: ReplicationMethod,
}

impl Default for BasketBuilder {
    fn default() -> Self {
        Self {
            id: None,
            ticker: None,
            name: None,
            constituents: Vec::new(),
            expense_ratio: 0.0,
            rebalance_freq: Frequency::quarterly(),
            tracking_index: None,
            creation_unit_size: 50000.0,
            currency: Currency::USD,
            shares_outstanding: None,
            replication: ReplicationMethod::Physical,
        }
    }
}

impl BasketBuilder {
    /// Create a new basket builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the basket identifier
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(InstrumentId::new(id.into()));
        self
    }

    /// Set the ticker symbol
    pub fn ticker(mut self, ticker: impl Into<String>) -> Self {
        self.ticker = Some(ticker.into());
        self
    }

    /// Set the descriptive name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the base currency
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Set the expense ratio
    pub fn expense_ratio(mut self, ratio: F) -> Self {
        self.expense_ratio = ratio;
        self
    }

    /// Set the rebalancing frequency
    pub fn rebalance_freq(mut self, freq: Frequency) -> Self {
        self.rebalance_freq = freq;
        self
    }

    /// Set the tracking index
    pub fn tracking_index(mut self, index: impl Into<String>) -> Self {
        self.tracking_index = Some(IndexId::new(index.into()));
        self
    }

    /// Set creation unit size
    pub fn creation_unit_size(mut self, size: F) -> Self {
        self.creation_unit_size = size;
        self
    }

    /// Set shares outstanding
    pub fn shares_outstanding(mut self, shares: F) -> Self {
        self.shares_outstanding = Some(shares);
        self
    }

    /// Set replication method
    pub fn replication(mut self, method: ReplicationMethod) -> Self {
        self.replication = method;
        self
    }

    /// Add an equity constituent using existing Equity instrument
    pub fn add_equity(
        mut self,
        id: impl Into<String>,
        ticker: impl Into<String>,
        weight: F,
        units: Option<F>,
    ) -> Self {
        let id_str = id.into();
        let ticker_str = ticker.into();
        
        // Create an Equity instrument for pricing
        let equity = Equity::new(id_str.clone(), ticker_str.clone(), self.currency);
        
        self.constituents.push(BasketConstituent {
            id: id_str,
            reference: ConstituentReference::Instrument(Arc::new(equity)),
            weight,
            units,
            ticker: Some(ticker_str),
        });
        self
    }

    /// Add a bond constituent using existing Bond instrument
    pub fn add_bond(
        mut self,
        id: impl Into<String>,
        bond: Bond,
        weight: F,
        units: Option<F>,
    ) -> Self {
        let id_str = id.into();
        let ticker = bond.id.to_string();
        
        self.constituents.push(BasketConstituent {
            id: id_str,
            reference: ConstituentReference::Instrument(Arc::new(bond)),
            weight,
            units,
            ticker: Some(ticker),
        });
        self
    }

    /// Add any instrument that implements Instrument
    pub fn add_instrument(
        mut self,
        id: impl Into<String>,
        instrument: Arc<dyn Instrument>,
        weight: F,
        units: Option<F>,
    ) -> Self {
        let id_str = id.into();
        let ticker = Some(instrument.id().to_string());
        
        self.constituents.push(BasketConstituent {
            id: id_str,
            reference: ConstituentReference::Instrument(instrument),
            weight,
            units,
            ticker,
        });
        self
    }

    /// Add a market data reference for simple price lookups
    pub fn add_market_data(
        mut self,
        id: impl Into<String>,
        price_id: impl Into<String>,
        asset_type: AssetType,
        weight: F,
        units: Option<F>,
    ) -> Self {
        let id_str = id.into();
        
        self.constituents.push(BasketConstituent {
            id: id_str.clone(),
            reference: ConstituentReference::MarketData {
                price_id: price_id.into(),
                asset_type,
            },
            weight,
            units,
            ticker: Some(id_str),
        });
        self
    }

    /// Build the basket
    pub fn build(self) -> Result<Basket> {
        let id = self.id.ok_or_else(|| {
            Error::Input(finstack_core::error::InputError::NotFound {
                id: "basket_id".to_string(),
            })
        })?;
        
        let name = self.name.unwrap_or_else(|| "Generic Basket".to_string());
        
        // Validate constituent weights
        if !self.constituents.is_empty() {
            let total_weight: F = self.constituents.iter().map(|c| c.weight).sum();
            if (total_weight - 1.0).abs() > 0.01 {
                return Err(Error::Input(finstack_core::error::InputError::Invalid));
            }
        }
        
        Ok(Basket {
            id,
            ticker: self.ticker,
            name,
            constituents: self.constituents,
            expense_ratio: self.expense_ratio,
            rebalance_freq: self.rebalance_freq,
            tracking_index: self.tracking_index,
            creation_unit_size: self.creation_unit_size,
            currency: self.currency,
            shares_outstanding: self.shares_outstanding,
            replication: self.replication,
            attributes: Attributes::new(),
        })
    }

    /// Pre-configured builder for equity ETF (like SPY)
    pub fn equity_etf(
        id: impl Into<String>,
        ticker: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self::new()
            .id(id)
            .ticker(ticker)
            .name(name)
            .currency(Currency::USD)
            .expense_ratio(0.0009)  // 9 bps typical for equity ETF
            .rebalance_freq(Frequency::quarterly())
            .creation_unit_size(50000.0)
            .replication(ReplicationMethod::Physical)
    }

    /// Pre-configured builder for bond ETF (like LQD, HYG)
    pub fn bond_etf(
        id: impl Into<String>,
        ticker: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self::new()
            .id(id)
            .ticker(ticker)
            .name(name)
            .currency(Currency::USD)
            .expense_ratio(0.0014)  // 14 bps typical for bond ETF
            .rebalance_freq(Frequency::monthly())  // More frequent for bonds
            .creation_unit_size(100000.0)  // Larger for bonds
            .replication(ReplicationMethod::Sampling)  // Often sampling for bonds
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_builder_basic() {
        let basket = BasketBuilder::new()
            .id("TEST_BASKET")
            .ticker("TEST")
            .name("Test Basket")
            .currency(Currency::USD)
            .expense_ratio(0.001)
            .add_market_data(
                "AAPL",
                "EQUITY/AAPL",
                AssetType::Equity,
                1.0,
                Some(100.0),
            )
            .build()
            .unwrap();

        assert_eq!(basket.id.as_str(), "TEST_BASKET");
        assert_eq!(basket.constituents.len(), 1);
        assert_eq!(basket.constituents[0].weight, 1.0);
    }

    #[test]
    fn test_equity_etf_preset() {
        let spy = BasketBuilder::equity_etf("SPY", "SPY", "SPDR S&P 500 ETF")
            .add_equity("AAPL", "AAPL", 0.6, Some(150000.0))
            .add_equity("MSFT", "MSFT", 0.4, Some(120000.0))
            .build()
            .unwrap();

        assert_eq!(spy.expense_ratio, 0.0009);
        assert_eq!(spy.creation_unit_size, 50000.0);
        assert!(matches!(spy.replication, ReplicationMethod::Physical));
    }

    #[test]
    fn test_bond_etf_preset() {
        let lqd = BasketBuilder::bond_etf("LQD", "LQD", "iShares iBoxx $ IG Corporate Bond ETF")
            .build()
            .unwrap();

        assert_eq!(lqd.expense_ratio, 0.0014);
        assert_eq!(lqd.creation_unit_size, 100000.0);
        assert!(matches!(lqd.replication, ReplicationMethod::Sampling));
    }

    #[test]
    fn test_weight_validation() {
        let result = BasketBuilder::new()
            .id("TEST")
            .add_market_data("A", "A", AssetType::Equity, 0.6, None)
            .add_market_data("B", "B", AssetType::Equity, 0.3, None)  // Only sums to 0.9
            .build();

        assert!(result.is_err());
    }
}
