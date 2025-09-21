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

        // Root-sum-square of return differences aligned by period
        let mut sum_sq_diff = 0.0;
        for k in 0..basket_returns.len() {
            let bench_return = benchmark_returns[k].1;
            let basket_return = basket_returns[k];
            let diff = basket_return - bench_return;
            sum_sq_diff += diff * diff;
        }
        let tracking_error = sum_sq_diff.sqrt();
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
        let out = match mode {
            ValueMode::PerShare { shares } => {
                // Resolve price then allocate per share
                let raw_value = match &constituent.reference {
                    ConstituentReference::Instrument(instrument) => {
                        instrument.value_dyn(context, as_of)?
                    }
                    ConstituentReference::MarketData { price_id, .. } => {
                        let scalar = context.price(price_id)?;
                        match scalar {
                            finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                                *money
                            }
                            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                                Money::new(*v, basket.currency)
                            }
                        }
                    }
                };
                let base_value =
                    self.to_basket_currency(raw_value, basket.currency, context, as_of)?;
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
                    let raw_value = match &constituent.reference {
                        ConstituentReference::Instrument(instrument) => {
                            instrument.value_dyn(context, as_of)?
                        }
                        ConstituentReference::MarketData { price_id, .. } => {
                            let scalar = context.price(price_id)?;
                            match scalar {
                                finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                                    *money
                                }
                                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                                    Money::new(*v, basket.currency)
                                }
                            }
                        }
                    };
                    let base_value =
                        self.to_basket_currency(raw_value, basket.currency, context, as_of)?;
                    base_value * units
                } else if let Some(a) = aum {
                    Money::new(a * constituent.weight, basket.currency)
                } else if let Some(s) = shares {
                    // Weight-only contribution scaled by shares × price
                    let raw_value = match &constituent.reference {
                        ConstituentReference::Instrument(instrument) => {
                            instrument.value_dyn(context, as_of)?
                        }
                        ConstituentReference::MarketData { price_id, .. } => {
                            let scalar = context.price(price_id)?;
                            match scalar {
                                finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                                    *money
                                }
                                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                                    Money::new(*v, basket.currency)
                                }
                            }
                        }
                    };
                    let base_value =
                        self.to_basket_currency(raw_value, basket.currency, context, as_of)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::basket::types::{
        AssetType, Basket, BasketConstituent, ConstituentReference, ReplicationMethod,
    };
    use crate::instruments::common::traits::Attributes;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
    use finstack_core::types::InstrumentId;
    use std::sync::Arc;

    fn d(y: i32, m: time::Month, day: u8) -> Date {
        Date::from_calendar_date(y, m, day).unwrap()
    }

    struct StaticFx;
    impl FxProvider for StaticFx {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<f64> {
            let r = match (from, to) {
                (Currency::EUR, Currency::USD) => 1.2,
                (Currency::USD, Currency::EUR) => 1.0 / 1.2,
                (a, b) if a == b => 1.0,
                _ => 1.0,
            };
            Ok(r)
        }
    }

    fn ctx_with_prices(prices: &[(&str, MarketScalar)], with_fx: bool) -> MarketContext {
        let mut ctx = MarketContext::new();
        if with_fx {
            let fx = FxMatrix::new(Arc::new(StaticFx));
            ctx = ctx.insert_fx(fx);
        }
        let mut out = ctx;
        for (id, scalar) in prices {
            out = out.insert_price(*id, scalar.clone());
        }
        out
    }

    #[test]
    fn nav_weighted_domestic_prices() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKT"),
            ticker: None,
            name: "Test".to_string(),
            constituents: vec![
                BasketConstituent {
                    id: "A".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "AAPL".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.6,
                    units: None,
                    ticker: None,
                },
                BasketConstituent {
                    id: "B".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "MSFT".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.4,
                    units: None,
                    ticker: None,
                },
            ],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 50_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1_000_000.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        let ctx = ctx_with_prices(
            &[
                (
                    "AAPL",
                    MarketScalar::Price(Money::new(100.0, Currency::USD)),
                ),
                ("MSFT", MarketScalar::Price(Money::new(50.0, Currency::USD))),
            ],
            false,
        );

        let nav = BasketPricer::new().nav(&basket, &ctx, as_of).unwrap();
        assert!((nav.amount() - 80.0).abs() < 1e-12);
        assert_eq!(nav.currency(), Currency::USD);
    }

    #[test]
    fn nav_units_requires_shares() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTU"),
            ticker: None,
            name: "Units".to_string(),
            constituents: vec![BasketConstituent {
                id: "X".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "XPRICE".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.0,
                units: Some(10.0),
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: None, // missing on purpose
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        let ctx = ctx_with_prices(
            &[(
                "XPRICE",
                MarketScalar::Price(Money::new(200.0, Currency::USD)),
            )],
            false,
        );
        let res = BasketPricer::new().nav(&basket, &ctx, as_of);
        assert!(res.is_err());
    }

    #[test]
    fn nav_units_with_shares() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTUS"),
            ticker: None,
            name: "Units with shares".to_string(),
            constituents: vec![BasketConstituent {
                id: "X".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "PX".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.0,
                units: Some(10.0),
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(100.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        let ctx = ctx_with_prices(
            &[("PX", MarketScalar::Price(Money::new(200.0, Currency::USD)))],
            false,
        );
        let nav = BasketPricer::new().nav(&basket, &ctx, as_of).unwrap();
        // 200 * 10 / 100 = 20
        assert!((nav.amount() - 20.0).abs() < 1e-12);
    }

    #[test]
    fn basket_value_total_mixed_units_weights() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTT"),
            ticker: None,
            name: "Total".to_string(),
            constituents: vec![
                BasketConstituent {
                    id: "W".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "PW".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.7,
                    units: None,
                    ticker: None,
                },
                BasketConstituent {
                    id: "U".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "PU".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.3,
                    units: Some(20.0),
                    ticker: None,
                },
            ],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1000.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        let ctx = ctx_with_prices(
            &[
                ("PW", MarketScalar::Price(Money::new(100.0, Currency::USD))),
                ("PU", MarketScalar::Price(Money::new(50.0, Currency::USD))),
            ],
            false,
        );
        let total = BasketPricer::new()
            .basket_value(&basket, &ctx, as_of)
            .unwrap();
        // Weight-only total: 100 * 0.7 * 1000 = 70,000; Units total: 50 * 20 = 1,000; Sum = 71,000
        assert!((total.amount() - 71_000.0).abs() < 1e-9);
    }

    #[test]
    fn basket_value_with_aum_and_expense_drag() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTAUM"),
            ticker: None,
            name: "AUM".to_string(),
            constituents: vec![
                BasketConstituent {
                    id: "W1".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "P1".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.6,
                    units: None,
                    ticker: None,
                },
                BasketConstituent {
                    id: "W2".to_string(),
                    reference: ConstituentReference::MarketData {
                        price_id: "P2".to_string(),
                        asset_type: AssetType::Equity,
                    },
                    weight: 0.4,
                    units: None,
                    ticker: None,
                },
            ],
            expense_ratio: 0.0025, // 25 bps annual
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: None,
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };
        let ctx = ctx_with_prices(&[], false);
        let aum = Money::new(1_000_000.0, Currency::USD);
        let total = BasketPricer::new()
            .basket_value_with_aum(&basket, &ctx, as_of, aum)
            .unwrap();
        let gross = 1_000_000.0;
        let expected_drag = gross * (0.0025 / BasketPricerConfig::default().days_in_year);
        let expected = gross - expected_drag;
        eprintln!(
            "debug basket_value_with_aum: total={} expected={} drag={}",
            total.amount(),
            expected,
            expected_drag
        );
        assert!((total.amount() - expected).abs() < 1e-2);
    }

    #[test]
    fn fx_conversion_for_foreign_constituent() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTFX"),
            ticker: None,
            name: "FX".to_string(),
            constituents: vec![BasketConstituent {
                id: "EURSEC".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "EURSEC".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 1.0,
                units: None,
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };

        let ctx = ctx_with_prices(
            &[(
                "EURSEC",
                MarketScalar::Price(Money::new(100.0, Currency::EUR)),
            )],
            true,
        );
        let nav = BasketPricer::new().nav(&basket, &ctx, as_of).unwrap();
        // 100 EUR * 1.2 = 120 USD
        assert!((nav.amount() - 120.0).abs() < 1e-12);
        assert_eq!(nav.currency(), Currency::USD);
    }

    #[test]
    fn unitless_price_scalar_is_interpreted_in_basket_currency() {
        let as_of = d(2025, time::Month::January, 2);
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTUL"),
            ticker: None,
            name: "UL".to_string(),
            constituents: vec![BasketConstituent {
                id: "UL1".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "ULX".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 1.0,
                units: None,
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };
        let ctx = ctx_with_prices(&[("ULX", MarketScalar::Unitless(2.5))], false);
        let nav = BasketPricer::new().nav(&basket, &ctx, as_of).unwrap();
        assert!((nav.amount() - 2.5).abs() < 1e-12);
        assert_eq!(nav.currency(), Currency::USD);
    }

    #[test]
    fn tracking_error_against_benchmark() {
        let dates = [
            d(2025, time::Month::January, 1),
            d(2025, time::Month::January, 2),
            d(2025, time::Month::January, 3),
        ];
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTTE"),
            ticker: None,
            name: "TE".to_string(),
            constituents: vec![BasketConstituent {
                id: "S".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "SPOT".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 1.0,
                units: None,
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };
        // Constant price -> zero basket returns
        let ctx = ctx_with_prices(
            &[(
                "SPOT",
                MarketScalar::Price(Money::new(100.0, Currency::USD)),
            )],
            false,
        );
        let bench = vec![(dates[0], 0.01), (dates[1], -0.005), (dates[2], 0.0)];
        let te = BasketPricer::new()
            .tracking_error(&basket, &ctx, &bench, dates[2])
            .unwrap();
        // With zero basket returns, TE is the root-sum-square of benchmark returns
        let expected = (0.01f64.powi(2) + 0.005f64.powi(2) + 0.0f64.powi(2)).sqrt();
        assert!((te - expected).abs() < 1e-9);
    }

    #[test]
    fn creation_basket_has_transaction_cost_and_cash_component() {
        let basket: Basket = Basket {
            id: InstrumentId::new("BKTCB"),
            ticker: None,
            name: "CB".to_string(),
            constituents: vec![BasketConstituent {
                id: "A".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "A".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 1.0,
                units: None,
                ticker: None,
            }],
            expense_ratio: 0.0,
            rebalance_freq: finstack_core::dates::Frequency::monthly(),
            tracking_index: None,
            creation_unit_size: 10_000.0,
            currency: Currency::USD,
            shares_outstanding: Some(1.0),
            replication: ReplicationMethod::Physical,
            attributes: Attributes::new(),
        };
        let cr = basket.creation_basket(100.0);
        assert_eq!(cr.creation_basket.len(), 1);
        assert_eq!(cr.transaction_cost.amount(), 0.02);
        assert_eq!(cr.transaction_cost.currency(), Currency::USD);
        assert!(cr.cash_component.is_some());
    }
}
