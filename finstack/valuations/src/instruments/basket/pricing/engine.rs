//! Core Basket pricing engine and helpers.
//!
//! Provides deterministic valuation for generic baskets/ETFs with support for
//! multi-asset constituents. The engine delegates to existing instrument
//! pricing where available (e.g., bonds, equities) and uses market data price
//! lookups for simple references. Expense ratio drag is applied in a simple
//! annualized manner.

use super::super::types::{Basket, BasketConstituent, ConstituentReference};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::{FxConversionPolicy, FxQuery};
use finstack_core::money::Money;
use finstack_core::{Result, F};

/// Internal valuation mode used to interpret weights/units per call site.
enum ValueMode {
    /// Return per-share contribution; units require shares to be present.
    PerShare { shares: Option<F> },
    /// Return total contribution; prefers units, else uses AUM, else shares.
    Total { shares: Option<F>, aum: Option<F> },
}

/// Configuration for basket pricing behaviour.
#[derive(Clone, Debug)]
pub struct BasketPricerConfig {
    /// Day basis used for fee accrual (e.g., 365.0 or 365.25). Avoid hardcoding in logic.
    pub days_in_year: F,
    /// FX policy hint for conversions when constituent currency != basket currency.
    pub fx_policy: FxConversionPolicy,
}

impl Default for BasketPricerConfig {
    fn default() -> Self {
        Self {
            days_in_year: 365.25,
            fx_policy: FxConversionPolicy::CashflowDate,
        }
    }
}

/// Basket pricing engine. Carries configuration; stateless across calls otherwise.
pub struct BasketPricer {
    config: BasketPricerConfig,
}

impl Default for BasketPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl BasketPricer {
    /// Create a new basket pricer with default configuration.
    pub fn new() -> Self {
        Self {
            config: BasketPricerConfig::default(),
        }
    }

    /// Create a basket pricer with a specific configuration.
    pub fn with_config(config: BasketPricerConfig) -> Self {
        Self { config }
    }

    /// Calculate Net Asset Value per share.
    pub fn nav(&self, basket: &Basket, context: &MarketContext, as_of: Date) -> Result<Money> {
        let mut per_share = 0.0;
        for constituent in &basket.constituents {
            let c = self.value_constituent(
                basket,
                constituent,
                context,
                as_of,
                ValueMode::PerShare {
                    shares: basket.shares_outstanding,
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
    pub fn basket_value(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let mut total = 0.0;
        for constituent in &basket.constituents {
            let c = self.value_constituent(
                basket,
                constituent,
                context,
                as_of,
                ValueMode::Total {
                    shares: basket.shares_outstanding,
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
    pub fn nav_with_aum(
        &self,
        basket: &Basket,
        context: &MarketContext,
        as_of: Date,
        aum: Money,
    ) -> Result<Money> {
        let aum_basket = self.to_basket_currency(aum, basket.currency, context, as_of)?;
        let total = self.basket_value_with_aum(basket, context, as_of, aum_basket)?;
        let nav_value = if let Some(shares) = basket.shares_outstanding {
            if shares > 0.0 {
                total.amount() / shares
            } else {
                total.amount()
            }
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
        let mut total = 0.0;
        let aum_amount = aum_basket.amount();
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
            total += c.amount();
        }
        let expense_drag = self.calculate_expense_drag(basket, total, as_of)?;
        Ok(Money::new(total - expense_drag, basket.currency))
    }

    /// Calculate tracking error vs benchmark index from provided returns.
    pub fn tracking_error(
        &self,
        basket: &Basket,
        context: &MarketContext,
        benchmark_returns: &[(Date, F)],
        _as_of: Date,
    ) -> Result<F> {
        // Calculate basket returns over the same periods
        let mut basket_returns = Vec::new();
        let mut prev_nav = None;

        for &(date, _) in benchmark_returns {
            let nav = self.nav(basket, context, date)?;
            if let Some(prev) = prev_nav {
                let return_rate = (nav.amount() / prev - 1.0) as F;
                basket_returns.push(return_rate);
            }
            prev_nav = Some(nav.amount());
        }

        if basket_returns.len() != benchmark_returns.len() - 1 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::DimensionMismatch,
            ));
        }

        // Std dev of differences
        let mut sum_sq_diff = 0.0;
        let n = basket_returns.len() as F;
        for i in 1..benchmark_returns.len() {
            let bench_return = benchmark_returns[i].1;
            let basket_return = basket_returns[i - 1];
            let diff = basket_return - bench_return;
            sum_sq_diff += diff * diff;
        }
        let tracking_error = (sum_sq_diff / (n - 1.0)).sqrt();
        Ok(tracking_error)
    }

    // ----- local helpers -----

    fn value_constituent(
        &self,
        basket: &Basket,
        constituent: &BasketConstituent,
        context: &MarketContext,
        as_of: Date,
        mode: ValueMode,
    ) -> Result<Money> {
        let raw_value = match &constituent.reference {
            ConstituentReference::Instrument(instrument) => instrument.value_dyn(context, as_of)?,
            ConstituentReference::MarketData { price_id, .. } => {
                let scalar = context.price(price_id)?;
                match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Price(money) => *money,
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                        Money::new(*v, basket.currency)
                    }
                }
            }
        };

        // Convert to basket currency if needed using context FX matrix
        let base_value = self.to_basket_currency(raw_value, basket.currency, context, as_of)?;

        let out = match mode {
            ValueMode::PerShare { shares } => {
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
                    base_value * units
                } else if let Some(a) = aum {
                    Money::new(a * constituent.weight, basket.currency)
                } else if let Some(s) = shares {
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

    #[inline]
    fn to_basket_currency(
        &self,
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
            .rate(FxQuery {
                from: money.currency(),
                to: target,
                on: as_of,
                policy: self.config.fx_policy,
                closure_check: None,
                want_meta: false,
            })?
            .rate;

        Ok(Money::new(money.amount() * rate, target))
    }
}
