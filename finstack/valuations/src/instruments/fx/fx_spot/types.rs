//! FX Spot types and implementations.
//!
//! This file defines the `FxSpot` instrument shape and integrates with the
//! standard instrument macro. Pricing is delegated to `pricer::FxSpotPricer`
//! to match the repository conventions (pricing separated from types), and
//! metrics live under `metrics/`.
//!
//! # FX Quote Conventions (Market Standards Review - Week 5)
//!
//! ## Base Currency vs Quote Currency
//!
//! FX rates are always quoted as **CCY1/CCY2** where:
//! - **CCY1 (base):** The currency being priced (numerator)
//! - **CCY2 (quote):** The currency used for pricing (denominator)
//!
//! **Example: EUR/USD = 1.10**
//! - Base: EUR (1 Euro)
//! - Quote: USD (costs 1.10 US Dollars)
//! - Interpretation: "1 EUR costs 1.10 USD"
//!
//! ## Common Market Conventions
//!
//! | Pair | Direction | Interpretation |
//! |------|-----------|----------------|
//! | EUR/USD | Euro vs Dollar | "Euro in Dollar" - price of 1 EUR in USD |
//! | GBP/USD | Pound vs Dollar | "Cable" - price of 1 GBP in USD |
//! | USD/JPY | Dollar vs Yen | "Dollar-yen" - price of 1 USD in JPY |
//! | AUD/USD | Aussie vs Dollar | Price of 1 AUD in USD |
//!
//! ## Reciprocal Rates
//!
//! The reciprocal rate swaps base and quote:
//! - EUR/USD = 1.10 → USD/EUR = 1/1.10 = 0.909
//! - GBP/USD = 1.25 → USD/GBP = 1/1.25 = 0.80
//!
//! ## In This Implementation
//!
//! The `FxSpot` instrument stores:
//! ```rust,no_run
//! use finstack_core::currency::Currency;
//! use finstack_core::types::InstrumentId;
//! use finstack_valuations::instruments::FxSpot;
//!
//! // CCY1 (base) is the currency being priced; CCY2 (quote) is the pricing currency.
//! let mut eur_usd = FxSpot::new(InstrumentId::from("EURUSD"), Currency::EUR, Currency::USD);
//! // How many units of `quote` per 1 unit of `base`
//! eur_usd.spot_rate = Some(1.10);
//! ```
//!
//! Example:
//! ```rust,no_run
//! use finstack_core::currency::Currency;
//! use finstack_core::types::InstrumentId;
//! use finstack_valuations::instruments::FxSpot;
//!
//! let eur_usd = FxSpot::new(InstrumentId::from("EURUSD"), Currency::EUR, Currency::USD);
//! // If spot_rate = 1.10, this means: 1 EUR = 1.10 USD
//! ```

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{adjust, BusinessDayConvention, Date, DateExt};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxProvider;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::Result;

/// FX Spot instrument (1 unit of `base` priced in `quote`).
///
/// Represents the spot exchange rate between two currencies following
/// standard market quoting conventions (base/quote or CCY1/CCY2).
///
/// # Quote Convention
///
/// The rate is interpreted as: **1 unit of base = rate units of quote**
///
/// For example, if `base = EUR`, `quote = USD`, and `spot_rate = 1.10`:
/// - 1 EUR = 1.10 USD
/// - This is the "EUR/USD" rate
///
/// # Settlement
///
/// FX spot typically settles T+2 (two business days after trade date).
/// This can be customized via `settlement_lag_days`.
///
/// See module-level documentation for comprehensive FX quoting conventions.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct FxSpot {
    /// Unique identifier for the FX pair
    pub id: InstrumentId,
    /// Base currency (the currency being priced)
    pub base: Currency,
    /// Quote currency (the currency used for pricing)
    pub quote: Currency,
    /// Optional settlement date (T+2 typically for spot)
    #[builder(optional)]
    pub settlement: Option<Date>,
    /// Optional settlement lag in business days when `settlement` is not provided (default: 2)
    #[builder(optional)]
    pub settlement_lag_days: Option<i32>,
    /// Optional spot rate (if not provided, will look up from market data)
    #[builder(optional)]
    pub spot_rate: Option<f64>,
    /// Optional notional amount in base currency (defaults to 1)
    #[builder(optional)]
    pub notional: Option<Money>,
    /// Business day convention to apply when adjusting settlement (default: Following)
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier used for business-day logic
    #[builder(optional)]
    pub calendar_id: Option<String>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl FxSpot {
    /// Create a new FX spot instrument
    pub fn new(id: InstrumentId, base: Currency, quote: Currency) -> Self {
        Self {
            id,
            base,
            quote,
            settlement: None,
            settlement_lag_days: None,
            spot_rate: None,
            notional: None,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            attributes: Attributes::new(),
        }
    }

    /// Compute present value in the instrument's quote currency.
    ///
    /// If an explicit `spot_rate` is set on the instrument, that is used directly
    /// to compute `quote_amount = base_notional.amount() * spot_rate`.
    /// Otherwise, the rate is obtained from the `MarketContext`'s `FxMatrix`.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        if let Some(rate) = self.spot_rate {
            let quote_amount = self.effective_notional().amount() * rate;
            return Ok(Money::new(quote_amount, self.quote));
        }

        let matrix = market.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        struct MatrixProvider<'a> {
            m: &'a finstack_core::money::fx::FxMatrix,
        }

        impl FxProvider for MatrixProvider<'_> {
            fn rate(
                &self,
                from: Currency,
                to: Currency,
                on: Date,
                policy: finstack_core::money::fx::FxConversionPolicy,
            ) -> finstack_core::Result<finstack_core::money::fx::FxRate> {
                let result = self.m.rate(finstack_core::money::fx::FxQuery::with_policy(
                    from, to, on, policy,
                ))?;
                Ok(result.rate)
            }
        }

        let provider = MatrixProvider { m: matrix };
        let policy = finstack_core::money::fx::FxConversionPolicy::CashflowDate;
        self.effective_notional()
            .convert(self.quote, as_of, &provider, policy)
    }

    /// Set the settlement date
    pub fn with_settlement(mut self, date: Date) -> Self {
        self.settlement = Some(date);
        self
    }

    /// Set the spot rate
    pub fn with_rate(mut self, rate: f64) -> Self {
        self.spot_rate = Some(rate);
        self
    }

    /// Set the notional amount (fallible version - preferred)
    pub fn with_notional_checked(self, notional: Money) -> finstack_core::Result<Self> {
        self.try_with_notional(notional)
    }

    /// Set the business day convention
    pub fn with_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.bdc = bdc;
        self
    }

    /// Set the holiday calendar identifier used for settlement adjustment
    pub fn with_calendar_id(mut self, id: impl Into<String>) -> Self {
        self.calendar_id = Some(id.into());
        self
    }

    /// Set the settlement lag in business days (positive for T+N, negative for T-N).
    pub fn with_settlement_lag_days(mut self, lag_days: i32) -> Self {
        self.settlement_lag_days = Some(lag_days);
        self
    }

    /// Fallible setter for notional that validates currency matches base
    pub fn try_with_notional(mut self, notional: Money) -> finstack_core::Result<Self> {
        if notional.currency() != self.base {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: self.base,
                actual: notional.currency(),
            });
        }
        self.notional = Some(notional);
        Ok(self)
    }

    /// Get the effective notional (defaults to 1 unit of base currency)
    pub fn effective_notional(&self) -> Money {
        self.notional.unwrap_or_else(|| Money::new(1.0, self.base))
    }

    /// Standard FX pair name (e.g., "EURUSD")
    pub fn pair_name(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }
}

impl crate::instruments::common::traits::Instrument for FxSpot {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxSpot
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for FxSpot {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        // FxSpot has no discount curve; return a dummy static placeholder
        // Generic DV01 will find no curves in MarketContext and return 0
        static DUMMY_ID: std::sync::OnceLock<finstack_core::types::CurveId> =
            std::sync::OnceLock::new();
        DUMMY_ID.get_or_init(|| finstack_core::types::CurveId::new("_fx_spot_no_curve"))
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for FxSpot {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        // FxSpot has no curve dependencies
        crate::instruments::common::traits::InstrumentCurves::builder().build()
    }
}

impl CashflowProvider for FxSpot {
    fn notional(&self) -> Option<Money> {
        self.notional
    }

    fn build_schedule(
        &self,
        _curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // FX spot settles on provided settlement date (BDC-adjusted if calendar present)
        // or computed T+N BUSINESS days when settlement is not provided.
        let settle_date = if let Some(date) = self.settlement {
            if let Some(id) = self.calendar_id.as_deref() {
                if let Some(cal) = calendar_by_id(id) {
                    adjust(date, self.bdc, cal)?
                } else {
                    date
                }
            } else {
                date
            }
        } else {
            // Compute T+N in a calendar-aware way if a calendar is available; otherwise
            // fall back to weekend-only business-day addition.
            let lag_days = self.settlement_lag_days.unwrap_or(2);
            if let Some(id) = self.calendar_id.as_deref() {
                if let Some(cal) = calendar_by_id(id) {
                    as_of.add_business_days(lag_days, cal)?
                } else {
                    as_of.add_weekdays(lag_days)
                }
            } else {
                as_of.add_weekdays(lag_days)
            }
        };

        if settle_date > as_of {
            // Future settlement - use explicit spot_rate if provided, otherwise query FX matrix
            let rate = if let Some(rate) = self.spot_rate {
                rate
            } else {
                // Try market context FX matrix
                let matrix = _curves.fx.as_ref().ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })
                })?;
                let q = finstack_core::money::fx::FxQuery::new(self.base, self.quote, settle_date);
                (**matrix).rate(q)?.rate
            };
            let value = Money::new(self.effective_notional().amount() * rate, self.quote);
            Ok(vec![(settle_date, value)])
        } else {
            // Already settled
            Ok(vec![])
        }
    }
}
