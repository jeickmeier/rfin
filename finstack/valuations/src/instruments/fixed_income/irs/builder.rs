use crate::instruments::common::{DateRange, InstrumentScheduleParams};
use crate::instruments::traits::Attributes;
use finstack_core::prelude::*;
use finstack_core::F;

use super::types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};

/// Enhanced IRS builder using parameter groups and required fields.
///
/// This builder eliminates the 18 optional fields of the previous version
/// by using parameter groups and making core parameters required.
///
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::irs::{InterestRateSwap, PayReceive};
/// use finstack_valuations::instruments::common::{DateRange, InstrumentScheduleParams};
/// use finstack_core::dates::{Date, Frequency, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
///
/// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
/// let end = Date::from_calendar_date(2030, Month::January, 15).unwrap();
/// let schedule = InstrumentScheduleParams::usd_standard();
///
/// let swap = InterestRateSwap::builder()
///     .id("IRS-001")
///     .notional(Money::new(10_000_000.0, Currency::USD))
///     .side(PayReceive::PayFixed)
///     .standard_fixed_leg("USD-OIS", 0.05, schedule.clone())
///     .standard_float_leg("USD-OIS", "USD-SOFR-3M", 0.0, schedule)
///     .dates(start, end)
///     .build()
///     .unwrap();
/// ```
#[derive(Default)]
pub struct IRSBuilder {
    // Core required parameters
    id: Option<String>,
    notional: Option<Money>,
    side: Option<PayReceive>,
    
    // Leg specifications (built via convenience methods)
    fixed_leg: Option<FixedLegSpec>,
    float_leg: Option<FloatLegSpec>,
    
    // Date range (shared for both legs)
    date_range: Option<DateRange>,
}

impl IRSBuilder {
    /// Create a new IRS builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set instrument ID (required)
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Set notional amount (required)
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    /// Set side (PayFixed or ReceiveFixed) (required)
    pub fn side(mut self, value: PayReceive) -> Self {
        self.side = Some(value);
        self
    }

    /// Set date range for both legs (required)
    pub fn dates(mut self, start: Date, end: Date) -> Self {
        self.date_range = Some(DateRange::new(start, end));
        self
    }

    /// Set date range from tenor in years
    pub fn tenor(mut self, start: Date, tenor_years: F) -> Self {
        self.date_range = Some(DateRange::from_tenor(start, tenor_years));
        self
    }

    /// Set fixed leg specification directly
    pub fn fixed_leg(mut self, spec: FixedLegSpec) -> Self {
        self.fixed_leg = Some(spec);
        self
    }

    /// Set floating leg specification directly
    pub fn float_leg(mut self, spec: FloatLegSpec) -> Self {
        self.float_leg = Some(spec);
        self
    }

    /// Convenience method to create standard fixed leg
    pub fn standard_fixed_leg(
        mut self,
        disc_id: &'static str,
        rate: F,
        schedule_params: InstrumentScheduleParams,
    ) -> Self {
        // Note: start/end will be set from date_range in build()
        let spec = FixedLegSpec {
            disc_id,
            rate,
            freq: schedule_params.frequency,
            dc: schedule_params.day_count,
            bdc: schedule_params.bdc,
            calendar_id: schedule_params.calendar_id,
            stub: schedule_params.stub,
            start: Date::MIN, // Placeholder - will be overridden
            end: Date::MIN,   // Placeholder - will be overridden
        };
        self.fixed_leg = Some(spec);
        self
    }

    /// Convenience method to create standard floating leg
    pub fn standard_float_leg(
        mut self,
        disc_id: &'static str,
        fwd_id: &'static str,
        spread_bp: F,
        schedule_params: InstrumentScheduleParams,
    ) -> Self {
        // Note: start/end will be set from date_range in build()
        let spec = FloatLegSpec {
            disc_id,
            fwd_id,
            spread_bp,
            freq: schedule_params.frequency,
            dc: schedule_params.day_count,
            bdc: schedule_params.bdc,
            calendar_id: schedule_params.calendar_id,
            stub: schedule_params.stub,
            start: Date::MIN, // Placeholder - will be overridden
            end: Date::MIN,   // Placeholder - will be overridden
        };
        self.float_leg = Some(spec);
        self
    }

    /// Build the Interest Rate Swap
    pub fn build(self) -> finstack_core::Result<InterestRateSwap> {
        // Validate required core parameters
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "swap_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "swap_notional".to_string(),
            })
        })?;
        let side = self.side.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "swap_side".to_string(),
            })
        })?;
        let date_range = self.date_range.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "swap_dates".to_string(),
            })
        })?;
        
        // Validate leg specifications
        let mut fixed_leg = self.fixed_leg.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fixed_leg_spec".to_string(),
            })
        })?;
        let mut float_leg = self.float_leg.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "float_leg_spec".to_string(),
            })
        })?;

        // Override dates from date_range
        fixed_leg.start = date_range.start;
        fixed_leg.end = date_range.end;
        float_leg.start = date_range.start;
        float_leg.end = date_range.end;

        Ok(InterestRateSwap {
            id,
            notional,
            side,
            fixed: fixed_leg,
            float: float_leg,
            attributes: Attributes::new(),
        })
    }
}
