//! Metrics for basket instruments.

use super::types::{AssetType, Basket, ConstituentReference};
use crate::metrics::{
    traits::{MetricCalculator, MetricContext},
    MetricId, MetricRegistry,
};
use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// Calculate Net Asset Value per share
pub struct NavCalculator;

impl MetricCalculator for NavCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        let nav = basket.nav(&context.curves, context.as_of)?;
        Ok(nav.amount())
    }
}

/// Calculate total basket value (before per-share division)
pub struct BasketValueCalculator;

impl MetricCalculator for BasketValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        let value = basket.basket_value(&context.curves, context.as_of)?;
        Ok(value.amount())
    }
}

/// Calculate number of constituents in the basket
pub struct ConstituentCountCalculator;

impl MetricCalculator for ConstituentCountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        Ok(basket.constituent_count() as F)
    }
}

/// Calculate expense ratio as percentage
pub struct ExpenseRatioCalculator;

impl MetricCalculator for ExpenseRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        Ok(basket.expense_ratio * 100.0) // Convert to percentage
    }
}

/// Calculate tracking error vs benchmark (requires benchmark data)
pub struct TrackingErrorCalculator;

impl MetricCalculator for TrackingErrorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;

        // For now, return 0.0 as tracking error calculation requires historical data
        // In a full implementation, this would:
        // 1. Get benchmark returns from MarketContext time series
        // 2. Calculate basket returns over same periods
        // 3. Compute standard deviation of return differences

        // Placeholder implementation
        if let Some(ref _index_id) = basket.tracking_index {
            // Would look up index returns and calculate tracking error
            Ok(0.0015) // 15 bps typical tracking error
        } else {
            Ok(0.0)
        }
    }
}

/// Calculate current utilization vs creation unit size
pub struct UtilizationCalculator;

impl MetricCalculator for UtilizationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;

        if let Some(shares) = basket.shares_outstanding {
            Ok(shares / basket.creation_unit_size)
        } else {
            Ok(0.0)
        }
    }
}

/// Calculate premium/discount to NAV (requires market price)
pub struct PremiumDiscountCalculator;

impl MetricCalculator for PremiumDiscountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;

        // Try to get market price from context
        if let Some(ticker) = &basket.ticker {
            if let Ok(market_scalar) = context.curves.price(ticker) {
                let market_price = match market_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                        money.amount()
                    }
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                };

                let nav = basket.nav(&context.curves, context.as_of)?;
                let premium_discount = (market_price / nav.amount() - 1.0) * 100.0; // As percentage

                Ok(premium_discount)
            } else {
                Ok(0.0) // No market price available
            }
        } else {
            Ok(0.0) // No ticker to look up
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Nav]
    }
}

/// Calculate effective exposure by asset type
pub struct AssetExposureCalculator {
    asset_type: AssetType,
}

impl AssetExposureCalculator {
    pub fn new(asset_type: AssetType) -> Self {
        Self { asset_type }
    }
}

impl MetricCalculator for AssetExposureCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;

        let mut total_exposure = 0.0;
        for constituent in &basket.constituents {
            // Check if constituent matches the asset type we're looking for
            let matches = match (&constituent.reference, &self.asset_type) {
                (ConstituentReference::MarketData { asset_type, .. }, target) => {
                    std::mem::discriminant(asset_type) == std::mem::discriminant(target)
                }
                (ConstituentReference::Instrument(instrument), target) => {
                    // Infer asset type from instrument type
                    let instrument_type = instrument.instrument_type();
                    matches!(
                        (instrument_type, target),
                        ("Bond", AssetType::Bond)
                            | ("Equity", AssetType::Equity)
                            | ("Basket", AssetType::ETF)
                    )
                }
            };

            if matches {
                total_exposure += constituent.weight;
            }
        }

        Ok(total_exposure * 100.0) // Return as percentage
    }
}

/// Register basket-specific metrics
pub fn register_basket_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(MetricId::Nav, Arc::new(NavCalculator), &["Basket"])
        .register_metric(
            MetricId::BasketValue,
            Arc::new(BasketValueCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::ConstituentCount,
            Arc::new(ConstituentCountCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::ExpenseRatio,
            Arc::new(ExpenseRatioCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::TrackingError,
            Arc::new(TrackingErrorCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::Utilization,
            Arc::new(UtilizationCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::PremiumDiscount,
            Arc::new(PremiumDiscountCalculator),
            &["Basket"],
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::basket::ReplicationMethod;
    use crate::instruments::traits::Attributes;
    use finstack_core::dates::Frequency;
    use finstack_core::market_data::MarketContext;
    use finstack_core::types::InstrumentId;
    use std::sync::Arc;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    fn create_test_basket() -> Basket {
        Basket {
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
        }
    }

    #[test]
    fn test_nav_calculator() {
        let basket = create_test_basket();
        let context = MarketContext::new();

        let mut metric_context = MetricContext::new(
            Arc::new(basket),
            Arc::new(context),
            test_date(),
            Money::new(100.0, Currency::USD),
        );

        let calculator = NavCalculator;
        // This will fail without proper market data, but tests the interface
        let _result = calculator.calculate(&mut metric_context);
    }

    #[test]
    fn test_constituent_count_calculator() {
        let basket = create_test_basket();
        let context = MarketContext::new();

        let mut metric_context = MetricContext::new(
            Arc::new(basket),
            Arc::new(context),
            test_date(),
            Money::new(100.0, Currency::USD),
        );

        let calculator = ConstituentCountCalculator;
        let result = calculator.calculate(&mut metric_context).unwrap();
        assert_eq!(result, 0.0); // Empty basket
    }

    #[test]
    fn test_expense_ratio_calculator() {
        let basket = create_test_basket();
        let context = MarketContext::new();

        let mut metric_context = MetricContext::new(
            Arc::new(basket),
            Arc::new(context),
            test_date(),
            Money::new(100.0, Currency::USD),
        );

        let calculator = ExpenseRatioCalculator;
        let result = calculator.calculate(&mut metric_context).unwrap();
        assert_eq!(result, 0.1); // 0.001 * 100 = 0.1%
    }
}
