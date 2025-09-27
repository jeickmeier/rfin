//! FX Spot types and implementations.
//!
//! This file defines the `FxSpot` instrument shape and integrates with the
//! standard instrument macro. Pricing is delegated to `pricing::FxSpotPricer`
//! to match the repository conventions (pricing separated from types), and
//! metrics live under `metrics/`.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::money::fx::FxProvider;
use finstack_core::prelude::*;
use finstack_core::types::InstrumentId;
use finstack_core::F;

/// FX Spot instrument (1 unit of `base` priced in `quote`).
///
/// Represents the spot exchange rate between two currencies.
/// The value represents how many units of the quote currency
/// are needed to buy one unit of the base currency.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    pub spot_rate: Option<F>,
    /// Optional notional amount in base currency (defaults to 1)
    #[builder(optional)]
    pub notional: Option<Money>,
    /// Business day convention to apply when adjusting settlement (default: Following)
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier used for business-day logic
    #[builder(optional)]
    pub calendar_id: Option<&'static str>,
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
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        if let Some(rate) = self.spot_rate {
            let quote_amount = self.effective_notional().amount() * rate;
            return Ok(Money::new(quote_amount, self.quote));
        }

        let matrix = curves.fx.as_ref().ok_or_else(|| {
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
    pub fn with_rate(mut self, rate: F) -> Self {
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
    pub fn with_calendar_id(mut self, id: &'static str) -> Self {
        self.calendar_id = Some(id);
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

impl_instrument!(
    FxSpot,
    "FxSpot",
    pv = |s, curves, as_of| {
        s.npv(curves, as_of)
    }
);

impl CashflowProvider for FxSpot {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // FX spot settles on provided settlement date (BDC-adjusted if calendar present)
        // or computed T+N BUSINESS days when settlement is not provided.
        let settle_date = if let Some(date) = self.settlement {
            if let Some(id) = self.calendar_id {
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
            if let Some(id) = self.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    // Walk business days using calendar since trait-object calendars
                    // are not accepted by DateExt::add_business_days (requires Sized)
                    let mut d = as_of;
                    let mut remaining = lag_days;
                    let step = if remaining >= 0 { 1 } else { -1 };
                    while remaining != 0 {
                        d = d.saturating_add(time::Duration::days(step as i64));
                        if cal.is_business_day(d) {
                            remaining -= step;
                        }
                    }
                    d
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
