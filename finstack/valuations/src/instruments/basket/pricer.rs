//! Basket pricing engine.
//!
//! This module contains all the pricing logic for basket instruments, separated from
//! the type definitions. It handles NAV calculations, constituent valuation, expense
//! drag, and currency conversions.

use super::types::{Basket, BasketConstituent, BasketPricingConfig, ConstituentReference};
use finstack_core::{
    dates::Date,
    market_data::MarketContext,
    money::{fx::FxQuery, Money},
    prelude::*,
    F,
};

/// Internal valuation mode used to interpret weights/units per call site.
#[derive(Debug, Clone)]
enum ValueMode {
    /// Return per-share contribution; units require shares to be present.
    PerShare { shares: Option<F> },
    /// Return total contribution; prefers units, else uses AUM, else shares.
    Total { shares: Option<F>, aum: Option<F> },
}

/// Basket calculation engine that handles all pricing logic.
///
/// This calculator is stateless and can be reused across multiple basket valuations.
/// It encapsulates all the complex logic for valuing basket constituents, handling
/// different value modes, applying expense drag, and managing currency conversions.
#[derive(Debug, Clone)]
pub struct BasketCalculator {
    config: BasketPricingConfig,
}

impl BasketCalculator {
    /// Create a new calculator with the given configuration.
    pub fn new(config: BasketPricingConfig) -> Self {
        Self { config }
    }

    /// Create a calculator with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BasketPricingConfig::default())
    }

    /// Calculate Net Asset Value per share.
    ///
    /// # Arguments
    /// * `basket` - The basket instrument to value
    /// * `context` - Market context with pricing data
    /// * `as_of` - Valuation date
    /// * `shares_outstanding` - Total shares outstanding for per-share calculation
    pub fn nav(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
        shares_outstanding: F,
    ) -> Result<Money> {
        let mut per_share = 0.0;
        for constituent in &basket.constituents {
            let c = self.value_constituent(
                basket,
                constituent,
                context,
                as_of,
                ValueMode::PerShare {
                    shares: Some(shares_outstanding),
                },
            )?;
            per_share += c.amount();
        }

        // Apply expense ratio drag to per-share value
        let expense_drag = self.calculate_expense_drag(basket, per_share, as_of)?;
        let per_share_after_fees = per_share - expense_drag;
        Ok(Money::new(per_share_after_fees, basket.currency))
    }

    /// Calculate total basket value (gross, without per-share division).
    ///
    /// # Arguments
    /// * `basket` - The basket instrument to value
    /// * `context` - Market context with pricing data
    /// * `as_of` - Valuation date
    /// * `shares_outstanding` - Optional shares outstanding for weight-based calculations
    pub fn basket_value(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
        shares_outstanding: Option<F>,
    ) -> Result<Money> {
        let mut total = 0.0;
        for constituent in &basket.constituents {
            let c = self.value_constituent(
                basket,
                constituent,
                context,
                as_of,
                ValueMode::Total {
                    shares: shares_outstanding,
                    aum: None,
                },
            )?;
            total += c.amount();
        }
        let expense_drag = self.calculate_expense_drag(basket, total, as_of)?;
        Ok(Money::new(total - expense_drag, basket.currency))
    }

    /// Calculate Net Asset Value per share using an explicit AUM.
    ///
    /// When constituents lack `units`, contributions are computed as
    /// `weight × AUM (in basket currency)`.
    ///
    /// # Arguments
    /// * `basket` - The basket instrument to value
    /// * `context` - Market context with pricing data
    /// * `as_of` - Valuation date
    /// * `aum` - Assets under management amount
    /// * `shares_outstanding` - Total shares outstanding for per-share calculation
    pub fn nav_with_aum(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
        aum: Money,
        shares_outstanding: F,
    ) -> Result<Money> {
        let aum_basket = self.to_basket_currency(basket, aum, basket.currency, context, as_of)?;
        let total = self.basket_value_with_aum(basket, context, as_of, aum_basket)?;
        let nav_value = if shares_outstanding > 0.0 {
            total.amount() / shares_outstanding
        } else {
            total.amount()
        };
        Ok(Money::new(nav_value, basket.currency))
    }

    /// Calculate total basket value using an explicit AUM for weight-based constituents.
    pub fn basket_value_with_aum(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
        aum_basket: Money,
    ) -> Result<Money> {
        let aum_amount = aum_basket.amount();
        // If all constituents are weight-based (no explicit units), total should equal AUM
        // to avoid floating rounding drift when weights sum to ~1.0.
        let all_weight_based = basket.constituents.iter().all(|c| c.units.is_none());
        let total = if all_weight_based {
            aum_amount
        } else {
            let mut sum = 0.0;
            for constituent in &basket.constituents {
                let c = self.value_constituent(
                    basket,
                    constituent,
                    context,
                    as_of,
                    ValueMode::Total {
                        shares: None,
                        aum: Some(aum_amount),
                    },
                )?;
                sum += c.amount();
            }
            sum
        };
        let expense_drag = self.calculate_expense_drag(basket, total, as_of)?;
        Ok(Money::new(total - expense_drag, basket.currency))
    }

    // ----- Internal Helper Methods -----

    /// Value a single constituent based on the given mode.
    fn value_constituent(
        &self,
        basket: &Basket,
        constituent: &BasketConstituent,
        context: &MarketContext,
        as_of: Date,
        mode: ValueMode,
    ) -> Result<Money> {
        let out = match mode {
            ValueMode::PerShare { shares } => {
                // Resolve price then allocate per share
                let raw_value = self.get_constituent_price(basket, constituent, context, as_of)?;
                let base_value =
                    self.to_basket_currency(basket, raw_value, basket.currency, context, as_of)?;
                if let Some(units) = constituent.units {
                    let s = shares.ok_or(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ))?;
                    if s <= 0.0 {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ));
                    }
                    (base_value * units) / s
                } else {
                    base_value * constituent.weight
                }
            }
            ValueMode::Total { shares, aum } => {
                if let Some(units) = constituent.units {
                    // Price × units (convert to basket currency first)
                    let raw_value =
                        self.get_constituent_price(basket, constituent, context, as_of)?;
                    let base_value = self.to_basket_currency(
                        basket,
                        raw_value,
                        basket.currency,
                        context,
                        as_of,
                    )?;
                    base_value * units
                } else if let Some(a) = aum {
                    Money::new(a * constituent.weight, basket.currency)
                } else if let Some(s) = shares {
                    // Weight-only contribution scaled by shares × price
                    let raw_value =
                        self.get_constituent_price(basket, constituent, context, as_of)?;
                    let base_value = self.to_basket_currency(
                        basket,
                        raw_value,
                        basket.currency,
                        context,
                        as_of,
                    )?;
                    base_value * constituent.weight * s
                } else {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ));
                }
            }
        };
        Ok(out)
    }

    /// Get the price for a single constituent.
    fn get_constituent_price(
        &self,
        basket: &Basket,
        constituent: &BasketConstituent,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        match &constituent.reference {
            ConstituentReference::Instrument(instrument) => instrument.value(context, as_of),
            ConstituentReference::MarketData { price_id, .. } => {
                let scalar = context.price(price_id.as_ref())?;
                match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Price(money) => Ok(*money),
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                        // For unitless scalars, use the basket currency by default
                        Ok(Money::new(*v, basket.currency))
                    }
                }
            }
        }
    }

    /// Calculate expense drag based on the portfolio value.
    fn calculate_expense_drag(
        &self,
        basket: &Basket,
        portfolio_value: F,
        _as_of: Date,
    ) -> Result<F> {
        // Simple daily accrual of expense ratio
        let daily_expense_rate = basket.expense_ratio / self.config.days_in_year;
        Ok(portfolio_value * daily_expense_rate)
    }

    /// Convert money to basket currency using FX rates.
    #[inline]
    fn to_basket_currency(
        &self,
        _basket: &Basket,
        money: Money,
        target: Currency,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if money.currency() == target {
            return Ok(money);
        }

        let fx = context.fx.as_ref().ok_or(finstack_core::Error::Input(
            finstack_core::error::InputError::NotFound {
                id: "fx".to_string(),
            },
        ))?;

        let rate = fx
            .rate(FxQuery::with_policy(
                money.currency(),
                target,
                as_of,
                self.config.fx_policy,
            ))?
            .rate;

        Ok(Money::new(money.amount() * rate, target))
    }
}

impl Default for BasketCalculator {
    fn default() -> Self {
        Self::with_defaults()
    }
}
