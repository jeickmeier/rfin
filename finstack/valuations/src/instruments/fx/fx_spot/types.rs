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
use crate::instruments::common_impl::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DateExt};
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
    /// Business day convention to apply when adjusting settlement (default: ModifiedFollowing)
    ///
    /// Note: Default changed from `Following` to `ModifiedFollowing` in v0.8.0 to align
    /// with ISDA standard FX settlement conventions.
    pub bdc: BusinessDayConvention,
    /// Optional base currency calendar for joint calendar settlement adjustment.
    ///
    /// Per market convention, FX settlement uses the joint calendar of both currencies.
    /// A date is a good business day only if it's valid in both calendars.
    #[builder(optional)]
    pub base_calendar_id: Option<String>,
    /// Optional quote currency calendar for joint calendar settlement adjustment.
    ///
    /// Per market convention, FX settlement uses the joint calendar of both currencies.
    /// A date is a good business day only if it's valid in both calendars.
    #[builder(optional)]
    pub quote_calendar_id: Option<String>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl FxSpot {
    /// Create a new FX spot instrument.
    ///
    /// Default business day convention is `ModifiedFollowing` per ISDA standard.
    pub fn new(id: InstrumentId, base: Currency, quote: Currency) -> Self {
        Self {
            id,
            base,
            quote,
            settlement: None,
            settlement_lag_days: None,
            spot_rate: None,
            notional: None,
            bdc: BusinessDayConvention::ModifiedFollowing,
            base_calendar_id: None,
            quote_calendar_id: None,
            attributes: Attributes::new(),
        }
    }

    /// Compute the effective settlement date for this FX spot.
    ///
    /// Returns the settlement date adjusted for business days according to the
    /// configured calendars and business day convention.
    ///
    /// # Joint Calendar Support
    ///
    /// When both `base_calendar_id` and `quote_calendar_id` are provided, settlement
    /// uses joint calendar logic: a date is valid only if it's a business day in both
    /// currencies' calendars. This matches professional FX settlement conventions.
    ///
    pub fn effective_settlement_date(&self, as_of: Date) -> Result<Date> {
        use crate::instruments::common_impl::fx_dates::{adjust_joint_calendar, roll_spot_date};

        // Check if we should use joint calendar logic
        let use_joint_calendar =
            self.base_calendar_id.is_some() || self.quote_calendar_id.is_some();

        if let Some(date) = self.settlement {
            // Explicit settlement date provided - adjust for business days
            if use_joint_calendar {
                adjust_joint_calendar(
                    date,
                    self.bdc,
                    self.base_calendar_id.as_deref(),
                    self.quote_calendar_id.as_deref(),
                )
            } else {
                Ok(date)
            }
        } else {
            // Compute T+N from as_of date
            let lag_days = self.settlement_lag_days.unwrap_or(2);

            if use_joint_calendar {
                // Use joint calendar spot roll
                roll_spot_date(
                    as_of,
                    lag_days as u32,
                    self.bdc,
                    self.base_calendar_id.as_deref(),
                    self.quote_calendar_id.as_deref(),
                )
            } else {
                Ok(as_of.add_weekdays(lag_days))
            }
        }
    }

    /// Set the settlement date
    pub fn with_settlement(mut self, date: Date) -> Self {
        self.settlement = Some(date);
        self
    }

    /// Set the spot rate with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the rate is negative or zero, as FX rates must be positive.
    /// A zero rate would imply one currency is worthless, and negative rates are
    /// economically meaningless for spot FX.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::{currency::Currency, types::InstrumentId};
    /// use finstack_valuations::instruments::FxSpot;
    ///
    /// let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
    ///     .with_rate(1.10)
    ///     .expect("valid rate");
    /// ```
    pub fn with_rate(mut self, rate: f64) -> finstack_core::Result<Self> {
        if !rate.is_finite() {
            return Err(finstack_core::Error::Validation(format!(
                "FX spot rate must be finite (got {}). NaN and Infinity are not valid rates.",
                rate
            )));
        }
        if rate < 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FX spot rate cannot be negative (got {}). FX rates must be positive.",
                rate
            )));
        }
        if rate == 0.0 {
            return Err(finstack_core::Error::Validation(
                "FX spot rate cannot be zero. A zero rate implies the base currency is worthless."
                    .to_string(),
            ));
        }
        self.spot_rate = Some(rate);
        Ok(self)
    }

    /// Set the spot rate without validation (unchecked).
    ///
    /// **Warning**: This method bypasses rate validation. Use [`with_rate`] for
    /// normal usage. This method exists for testing edge cases or when the rate
    /// has already been validated externally.
    ///
    /// # Safety
    ///
    /// Caller is responsible for ensuring the rate is valid (positive, non-zero).
    pub fn with_rate_unchecked(mut self, rate: f64) -> Self {
        self.spot_rate = Some(rate);
        self
    }

    /// Set the notional amount with currency validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the notional currency doesn't match the base currency.
    pub fn with_notional(mut self, notional: Money) -> finstack_core::Result<Self> {
        if notional.currency() != self.base {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: self.base,
                actual: notional.currency(),
            });
        }
        self.notional = Some(notional);
        Ok(self)
    }

    /// Set the business day convention
    pub fn with_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.bdc = bdc;
        self
    }

    /// Set the holiday calendar identifier used for settlement adjustment.
    ///
    /// Set the base currency calendar for joint calendar settlement adjustment.
    ///
    /// Per market convention, FX settlement uses the joint calendar of both currencies.
    /// A date is a good business day only if it's valid in both calendars.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::{currency::Currency, types::InstrumentId};
    /// use finstack_valuations::instruments::FxSpot;
    ///
    /// let eur_usd = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
    ///     .with_base_calendar_id("TARGET")    // ECB TARGET calendar for EUR
    ///     .with_quote_calendar_id("USNY");    // US New York calendar for USD
    /// ```
    pub fn with_base_calendar_id(mut self, id: impl Into<String>) -> Self {
        self.base_calendar_id = Some(id.into());
        self
    }

    /// Set the quote currency calendar for joint calendar settlement adjustment.
    ///
    /// Per market convention, FX settlement uses the joint calendar of both currencies.
    /// A date is a good business day only if it's valid in both calendars.
    pub fn with_quote_calendar_id(mut self, id: impl Into<String>) -> Self {
        self.quote_calendar_id = Some(id.into());
        self
    }

    /// Set the settlement lag in business days (positive for T+N, negative for T-N).
    pub fn with_settlement_lag_days(mut self, lag_days: i32) -> Self {
        self.settlement_lag_days = Some(lag_days);
        self
    }

    /// Get the effective notional (defaults to 1 unit of base currency)
    pub fn effective_notional(&self) -> Money {
        self.notional.unwrap_or_else(|| Money::new(1.0, self.base))
    }

    /// Standard FX pair name (e.g., "EURUSD")
    pub fn pair_name(&self) -> String {
        format!("{}{}", self.base, self.quote)
    }

    /// Create an FX spot with T+1 settlement (used for USD/CAD and other same-day pairs).
    ///
    /// Per market convention, USD/CAD settles T+1 rather than the standard T+2.
    /// This convenience method creates an FX spot with the appropriate settlement lag.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::{currency::Currency, types::InstrumentId};
    /// use finstack_valuations::instruments::FxSpot;
    ///
    /// // USD/CAD with standard T+1 settlement
    /// let usd_cad = FxSpot::new_t1(
    ///     InstrumentId::new("USDCAD"),
    ///     Currency::USD,
    ///     Currency::CAD,
    /// );
    /// ```
    pub fn new_t1(id: InstrumentId, base: Currency, quote: Currency) -> Self {
        Self::new(id, base, quote).with_settlement_lag_days(1)
    }

    /// Check if this is a same-region pair that typically settles T+1.
    ///
    /// Returns `true` for pairs like USD/CAD, USD/MXN, USD/TRY that conventionally
    /// settle in one business day rather than the standard T+2.
    ///
    /// # Market Convention Reference
    ///
    /// Per Bloomberg/Reuters FX settlement conventions:
    /// - **USD/CAD**: North American same-day zone (T+1)
    /// - **USD/MXN**: North American same-day zone (T+1)
    /// - **USD/TRY**: Turkish Lira settles T+1 per Istanbul market convention
    ///
    /// Note: This is informational only and does not affect settlement calculation.
    /// Use [`new_t1`] or [`with_settlement_lag_days(1)`] to set T+1 settlement.
    pub fn is_t1_pair(&self) -> bool {
        // USD/CAD, USD/MXN, and USD/TRY are the most common T+1 pairs
        let pair = (self.base, self.quote);
        matches!(
            pair,
            (Currency::USD, Currency::CAD)
                | (Currency::CAD, Currency::USD)
                | (Currency::USD, Currency::MXN)
                | (Currency::MXN, Currency::USD)
                | (Currency::USD, Currency::TRY)
                | (Currency::TRY, Currency::USD)
        )
    }
}

impl crate::instruments::common_impl::traits::Instrument for FxSpot {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxSpot
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        let mut deps =
            crate::instruments::common_impl::dependencies::MarketDependencies::from_curve_dependencies(
                self,
            )?;
        deps.add_fx_pair(self.base, self.quote);
        Ok(deps)
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Compute effective settlement date
        let settle_date = self.effective_settlement_date(as_of)?;

        // If settlement is on or before as_of, trade has settled (end-of-day convention)
        if settle_date <= as_of {
            return Ok(finstack_core::money::Money::new(0.0, self.quote));
        }

        if let Some(rate) = self.spot_rate {
            let quote_amount = self.effective_notional().amount() * rate;
            return Ok(finstack_core::money::Money::new(quote_amount, self.quote));
        }

        let matrix = market.fx().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        struct MatrixProvider<'a> {
            m: &'a finstack_core::money::fx::FxMatrix,
        }

        impl FxProvider for MatrixProvider<'_> {
            fn rate(
                &self,
                from: finstack_core::currency::Currency,
                to: finstack_core::currency::Currency,
                on: finstack_core::dates::Date,
                policy: finstack_core::money::fx::FxConversionPolicy,
            ) -> finstack_core::Result<finstack_core::money::fx::FxRate> {
                let result = self.m.rate(finstack_core::money::fx::FxQuery::with_policy(
                    from, to, on, policy,
                ))?;
                Ok(result.rate)
            }
        }

        let provider = MatrixProvider { m: matrix.as_ref() };
        let policy = finstack_core::money::fx::FxConversionPolicy::CashflowDate;
        // Use settlement date for the FX conversion when using CashflowDate policy
        self.effective_notional()
            .convert(self.quote, settle_date, &provider, policy)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        None
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for FxSpot {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        // FxSpot has no curve dependencies
        crate::instruments::common_impl::traits::InstrumentCurves::builder().build()
    }
}

impl CashflowProvider for FxSpot {
    fn notional(&self) -> Option<Money> {
        self.notional
    }

    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let settle_date = self.effective_settlement_date(as_of)?;

        let flows = if settle_date > as_of {
            // Future settlement - use explicit spot_rate if provided, otherwise query FX matrix
            let rate = if let Some(rate) = self.spot_rate {
                rate
            } else {
                // Try market context FX matrix
                let matrix = curves.fx().ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })
                })?;
                let q = finstack_core::money::fx::FxQuery::new(self.base, self.quote, settle_date);
                matrix.as_ref().rate(q)?.rate
            };
            let value = Money::new(self.effective_notional().amount() * rate, self.quote);
            vec![(settle_date, value)]
        } else {
            // Already settled
            Vec::new()
        };

        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            self.notional(),
            finstack_core::dates::DayCount::Act365F, // Standard for FX spot
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use time::Month;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid test date")
    }

    #[test]
    fn test_fx_spot_creation() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD);
        assert_eq!(spot.base, Currency::EUR);
        assert_eq!(spot.quote, Currency::USD);
        assert_eq!(spot.pair_name(), "EURUSD");
    }

    #[test]
    fn test_fx_spot_with_explicit_rate() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_rate(1.10)
            .expect("valid rate");

        let market = MarketContext::new();
        let as_of = date(2025, Month::January, 15);
        let pv = spot
            .value(&market, as_of)
            .expect("should price with explicit rate");

        // 1 EUR * 1.10 = 1.10 USD
        assert!((pv.amount() - 1.10).abs() < 1e-10);
        assert_eq!(pv.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_spot_effective_settlement_date_default() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD);

        // Wednesday -> should settle Friday (T+2 weekdays)
        let as_of = date(2025, Month::January, 15); // Wednesday
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, date(2025, Month::January, 17)); // Friday
    }

    #[test]
    fn test_fx_spot_effective_settlement_date_explicit() {
        let settle_date = date(2025, Month::January, 20);
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_settlement(settle_date);

        let as_of = date(2025, Month::January, 15);
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, settle_date);
    }

    #[test]
    fn test_fx_spot_effective_settlement_date_custom_lag() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_settlement_lag_days(1);

        // Wednesday -> should settle Thursday (T+1 weekdays)
        let as_of = date(2025, Month::January, 15); // Wednesday
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, date(2025, Month::January, 16)); // Thursday
    }

    #[test]
    fn test_fx_spot_returns_zero_when_settled() {
        let settle_date = date(2025, Month::January, 10);
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_settlement(settle_date)
            .with_rate(1.10)
            .expect("valid rate");

        let market = MarketContext::new();
        let as_of = date(2025, Month::January, 15); // After settlement
        let pv = spot.value(&market, as_of).expect("should price");

        assert_eq!(pv.amount(), 0.0, "Should return zero when settled");
    }

    #[test]
    fn test_fx_spot_notional_currency_validation() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD);
        let wrong_notional = Money::new(1000.0, Currency::GBP);

        let result = spot.with_notional(wrong_notional);
        assert!(result.is_err(), "Should reject notional in wrong currency");
    }

    #[test]
    fn test_fx_spot_with_notional() {
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_notional(Money::new(1_000_000.0, Currency::EUR))
            .expect("valid notional")
            .with_rate(1.10)
            .expect("valid rate");

        let market = MarketContext::new();
        let as_of = date(2025, Month::January, 15);
        let pv = spot.value(&market, as_of).expect("should price");

        // 1,000,000 EUR * 1.10 = 1,100,000 USD
        assert!((pv.amount() - 1_100_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_fx_spot_rejects_negative_rate() {
        let result =
            FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD).with_rate(-1.10);
        assert!(result.is_err(), "Should reject negative rate");

        let err = result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("negative") || err_msg.contains("spot_rate"),
            "Error should mention negative rate"
        );
    }

    #[test]
    fn test_fx_spot_rejects_nan_rate() {
        let result = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_rate(f64::NAN);
        assert!(result.is_err(), "Should reject NaN rate");

        let err = result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("finite") || err_msg.contains("NaN"),
            "Error should mention finite: {}",
            err_msg
        );
    }

    #[test]
    fn test_fx_spot_rejects_infinity_rate() {
        let result = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_rate(f64::INFINITY);
        assert!(result.is_err(), "Should reject Infinity rate");
    }

    #[test]
    fn test_fx_spot_rejects_zero_rate() {
        let result =
            FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD).with_rate(0.0);
        assert!(result.is_err(), "Should reject zero rate");

        let err = result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("zero") || err_msg.contains("spot_rate"),
            "Error should mention zero rate"
        );
    }

    #[test]
    fn test_fx_spot_new_t1() {
        let spot = FxSpot::new_t1(InstrumentId::new("USDCAD"), Currency::USD, Currency::CAD);

        // Wednesday -> should settle Thursday (T+1 weekdays)
        let as_of = date(2025, Month::January, 15); // Wednesday
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, date(2025, Month::January, 16)); // Thursday (T+1)
    }

    #[test]
    fn test_fx_spot_is_t1_pair() {
        let usd_cad = FxSpot::new(InstrumentId::new("USDCAD"), Currency::USD, Currency::CAD);
        assert!(usd_cad.is_t1_pair(), "USD/CAD should be T+1 pair");

        let cad_usd = FxSpot::new(InstrumentId::new("CADUSD"), Currency::CAD, Currency::USD);
        assert!(cad_usd.is_t1_pair(), "CAD/USD should be T+1 pair");

        let usd_mxn = FxSpot::new(InstrumentId::new("USDMXN"), Currency::USD, Currency::MXN);
        assert!(usd_mxn.is_t1_pair(), "USD/MXN should be T+1 pair");

        let usd_try = FxSpot::new(InstrumentId::new("USDTRY"), Currency::USD, Currency::TRY);
        assert!(usd_try.is_t1_pair(), "USD/TRY should be T+1 pair");

        let try_usd = FxSpot::new(InstrumentId::new("TRYUSD"), Currency::TRY, Currency::USD);
        assert!(try_usd.is_t1_pair(), "TRY/USD should be T+1 pair");

        let eur_usd = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD);
        assert!(!eur_usd.is_t1_pair(), "EUR/USD should NOT be T+1 pair");
    }

    #[test]
    fn test_fx_spot_negative_settlement_lag() {
        // Negative lag for historical valuations (T-1)
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_settlement_lag_days(-1);

        // Wednesday with T-1 -> should settle Tuesday
        let as_of = date(2025, Month::January, 15); // Wednesday
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, date(2025, Month::January, 14)); // Tuesday (T-1)
    }

    #[test]
    fn test_fx_spot_zero_settlement_lag() {
        // T+0 same-day settlement
        let spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_settlement_lag_days(0);

        let as_of = date(2025, Month::January, 15);
        let settle = spot
            .effective_settlement_date(as_of)
            .expect("should compute");
        assert_eq!(settle, as_of, "T+0 should settle same day");
    }
}
