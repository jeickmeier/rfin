//! Zero-coupon Inflation Swap (boilerplate implementation).
//!
//! This module adds a minimal scaffold for an inflation swap instrument so it
//! can participate in the unified pricing and metrics framework. Valuation
//! logic is intentionally minimal (returns zero) until completed.

pub mod metrics;

use crate::instruments::traits::Attributes;
use crate::metrics::MetricId;
use finstack_core::prelude::*;
use finstack_core::F;

/// Direction from the perspective of paying fixed real vs receiving inflation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg
    PayFixed,
    /// Receive fixed (real) leg, pay inflation leg
    ReceiveFixed,
}

/// Inflation swap definition (boilerplate)
///
/// Minimal fields to represent a zero-coupon inflation swap. We keep this
/// intentionally compact until full pricing is implemented.
#[derive(Clone, Debug)]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of indexation
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed real rate (as decimal)
    pub fixed_rate: F,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_id: &'static str,
    /// Discount curve identifier (quote currency)
    pub disc_id: &'static str,
    /// Day count for any accrual-style metrics if needed
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {
    /// Builder entrypoint
    pub fn builder() -> InflationSwapBuilder {
        InflationSwapBuilder::new()
    }
}

impl_instrument!(
    InflationSwap,
    "InflationSwap",
    pv = |s, _curves, _as_of| {
        // Placeholder PV until full implementation; zero in notional currency
        Ok(Money::new(0.0, s.notional.currency()))
    },
    metrics = |_s| {
        // Keep minimal, custom placeholders. Real metrics to be added later.
        vec![
            MetricId::custom("breakeven"),
            MetricId::custom("fixed_leg_pv"),
            MetricId::custom("inflation_leg_pv"),
        ]
    }
);

/// Builder for `InflationSwap`
#[derive(Default)]
pub struct InflationSwapBuilder {
    id: Option<String>,
    notional: Option<Money>,
    start: Option<Date>,
    maturity: Option<Date>,
    fixed_rate: Option<F>,
    inflation_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    dc: Option<DayCount>,
    side: Option<PayReceiveInflation>,
}

impl InflationSwapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }
    pub fn start(mut self, value: Date) -> Self {
        self.start = Some(value);
        self
    }
    pub fn maturity(mut self, value: Date) -> Self {
        self.maturity = Some(value);
        self
    }
    pub fn fixed_rate(mut self, value: F) -> Self {
        self.fixed_rate = Some(value);
        self
    }
    pub fn inflation_id(mut self, value: &'static str) -> Self {
        self.inflation_id = Some(value);
        self
    }
    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }
    pub fn dc(mut self, value: DayCount) -> Self {
        self.dc = Some(value);
        self
    }
    pub fn side(mut self, value: PayReceiveInflation) -> Self {
        self.side = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<InflationSwap> {
        Ok(InflationSwap {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            notional: self.notional.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            start: self.start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            maturity: self.maturity.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            fixed_rate: self.fixed_rate.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            inflation_id: self.inflation_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            disc_id: self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.dc.unwrap_or(DayCount::ActAct),
            side: self.side.unwrap_or(PayReceiveInflation::PayFixed),
            attributes: Attributes::new(),
        })
    }
}

// CashflowProvider is intentionally omitted for now; when implemented, we can
// switch to impl_instrument_schedule_pv! variant and compute PV from flows.

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_inflation_swap_builder() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let mat = Date::from_calendar_date(2030, Month::January, 15).unwrap();
        let inst = InflationSwap::builder()
            .id("ZCIS_1")
            .notional(Money::new(10_000_000.0, Currency::USD))
            .start(start)
            .maturity(mat)
            .fixed_rate(0.025)
            .inflation_id("US-CPI-U")
            .disc_id("USD-OIS")
            .dc(DayCount::ActAct)
            .side(PayReceiveInflation::PayFixed)
            .build()
            .unwrap();

        assert_eq!(inst.id, "ZCIS_1");
        assert_eq!(inst.fixed_rate, 0.025);
        assert_eq!(inst.inflation_id, "US-CPI-U");
        assert_eq!(inst.disc_id, "USD-OIS");
    }
}
