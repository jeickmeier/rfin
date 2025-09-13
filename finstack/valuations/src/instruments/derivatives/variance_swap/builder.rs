//! Builder pattern for VarianceSwap construction.

use finstack_core::{
    currency::Currency,
    dates::{Date, DayCount, Frequency},
    math::stats::RealizedVarMethod,
    money::Money,
    types::id::{CurveId, InstrumentId},
    F, Result,
};

use crate::instruments::{
    common::parameter_groups::DateRange,
    traits::Attributes,
};

use super::types::{PayReceive, VarianceSwap};

/// Builder for constructing VarianceSwap instances.
#[derive(Debug, Clone)]
pub struct VarianceSwapBuilder {
    id: Option<InstrumentId>,
    underlying_id: Option<String>,
    notional: Option<Money>,
    strike_variance: Option<F>,
    dates: Option<DateRange>,
    observation_freq: Frequency,
    realized_var_method: RealizedVarMethod,
    side: PayReceive,
    disc_id: Option<CurveId>,
    day_count: DayCount,
}

impl Default for VarianceSwapBuilder {
    fn default() -> Self {
        Self {
            id: None,
            underlying_id: None,
            notional: None,
            strike_variance: None,
            dates: None,
            observation_freq: Frequency::daily(),
            realized_var_method: RealizedVarMethod::CloseToClose,
            side: PayReceive::Receive,
            disc_id: None,
            day_count: DayCount::Act365F,
        }
    }
}

impl VarianceSwapBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the instrument identifier.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(InstrumentId::from(id.into()));
        self
    }

    /// Set the underlying identifier.
    pub fn underlying_id(mut self, id: impl Into<String>) -> Self {
        self.underlying_id = Some(id.into());
        self
    }

    /// Set the notional amount (in variance units).
    pub fn notional(mut self, notional: Money) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Set the notional amount from value and currency.
    pub fn notional_amount(mut self, amount: F, currency: Currency) -> Self {
        self.notional = Some(Money::new(amount, currency));
        self
    }

    /// Set the strike variance (annualized).
    pub fn strike_variance(mut self, variance: F) -> Self {
        self.strike_variance = Some(variance);
        self
    }

    /// Set the strike volatility (will be squared to get variance).
    pub fn strike_volatility(mut self, volatility: F) -> Self {
        self.strike_variance = Some(volatility * volatility);
        self
    }

    /// Set start and maturity dates.
    pub fn dates(mut self, start: Date, maturity: Date) -> Self {
        self.dates = Some(DateRange::new(start, maturity));
        self
    }

    /// Set the date range.
    pub fn date_range(mut self, range: DateRange) -> Self {
        self.dates = Some(range);
        self
    }

    /// Set dates from tenor.
    pub fn tenor(mut self, start: Date, tenor_years: F) -> Self {
        self.dates = Some(DateRange::from_tenor(start, tenor_years));
        self
    }

    /// Set the observation frequency.
    pub fn observation_freq(mut self, freq: Frequency) -> Self {
        self.observation_freq = freq;
        self
    }

    /// Set the realized variance calculation method.
    pub fn realized_var_method(mut self, method: RealizedVarMethod) -> Self {
        self.realized_var_method = method;
        self
    }

    /// Set the side (pay/receive variance).
    pub fn side(mut self, side: PayReceive) -> Self {
        self.side = side;
        self
    }

    /// Set to receive variance (long variance).
    pub fn receive_variance(mut self) -> Self {
        self.side = PayReceive::Receive;
        self
    }

    /// Set to pay variance (short variance).
    pub fn pay_variance(mut self) -> Self {
        self.side = PayReceive::Pay;
        self
    }

    /// Set the discount curve identifier.
    pub fn disc_id(mut self, id: impl Into<String>) -> Self {
        self.disc_id = Some(CurveId::from(id.into()));
        self
    }

    /// Set the day count convention.
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }

    /// Build the VarianceSwap instance.
    pub fn build(self) -> Result<VarianceSwap> {
        let id = self.id.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;
        let underlying_id = self.underlying_id.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;
        let notional = self.notional.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;
        let strike_variance = self.strike_variance.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;
        let dates = self.dates.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;
        let disc_id = self.disc_id.ok_or_else(|| -> finstack_core::Error { finstack_core::error::InputError::Invalid.into() })?;

        // Validate inputs
        if strike_variance <= 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        if dates.start >= dates.end {
            return Err(finstack_core::error::InputError::InvalidDateRange.into());
        }

        Ok(VarianceSwap {
            id,
            underlying_id,
            notional,
            strike_variance,
            start_date: dates.start,
            maturity: dates.end,
            observation_freq: self.observation_freq,
            realized_var_method: self.realized_var_method,
            side: self.side,
            disc_id,
            day_count: self.day_count,
            attributes: Attributes::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;

    fn test_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, time::Month::try_from(month).unwrap(), day).unwrap()
    }

    #[test]
    fn test_builder_basic() {
        let swap = VarianceSwapBuilder::new()
            .id("VAR_SPX_1Y")
            .underlying_id("SPX")
            .notional_amount(100_000.0, Currency::USD)
            .strike_volatility(0.20) // 20% vol -> 0.04 variance
            .dates(test_date(2025, 1, 1), test_date(2026, 1, 1))
            .disc_id("USD_OIS")
            .build()
            .unwrap();

        assert_eq!(swap.id.as_str(), "VAR_SPX_1Y");
        assert!((swap.strike_variance - 0.04).abs() < 1e-10);
        assert_eq!(swap.notional.amount(), 100_000.0);
    }

    #[test]
    fn test_builder_validation() {
        // Missing required field
        let result = VarianceSwapBuilder::new()
            .id("VAR_TEST")
            .build();
        assert!(result.is_err());

        // Invalid strike
        let result = VarianceSwapBuilder::new()
            .id("VAR_TEST")
            .underlying_id("SPX")
            .notional_amount(100_000.0, Currency::USD)
            .strike_variance(-0.04)
            .dates(test_date(2025, 1, 1), test_date(2026, 1, 1))
            .disc_id("USD_OIS")
            .build();
        assert!(result.is_err());
    }
}
