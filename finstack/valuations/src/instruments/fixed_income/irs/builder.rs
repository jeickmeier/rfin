use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Frequency, StubKind};
use finstack_core::prelude::*;
use finstack_core::F;

use super::types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};

/// Builder pattern for IRS instruments
#[derive(Default)]
pub struct IRSBuilder {
    id: Option<String>,
    notional: Option<Money>,
    side: Option<PayReceive>,
    // Fixed leg fields
    fixed_disc_id: Option<&'static str>,
    fixed_rate: Option<F>,
    fixed_freq: Option<Frequency>,
    fixed_dc: Option<DayCount>,
    fixed_bdc: Option<BusinessDayConvention>,
    fixed_calendar_id: Option<&'static str>,
    fixed_stub: Option<StubKind>,
    fixed_start: Option<Date>,
    fixed_end: Option<Date>,
    // Float leg fields
    float_disc_id: Option<&'static str>,
    float_fwd_id: Option<&'static str>,
    float_spread_bp: Option<F>,
    float_freq: Option<Frequency>,
    float_dc: Option<DayCount>,
    float_bdc: Option<BusinessDayConvention>,
    float_calendar_id: Option<&'static str>,
    float_stub: Option<StubKind>,
    float_start: Option<Date>,
    float_end: Option<Date>,
}

impl IRSBuilder {
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
    pub fn side(mut self, value: PayReceive) -> Self {
        self.side = Some(value);
        self
    }

    // Fixed leg setters
    pub fn fixed_disc_id(mut self, value: &'static str) -> Self {
        self.fixed_disc_id = Some(value);
        self
    }
    pub fn fixed_rate(mut self, value: F) -> Self {
        self.fixed_rate = Some(value);
        self
    }
    pub fn fixed_freq(mut self, value: Frequency) -> Self {
        self.fixed_freq = Some(value);
        self
    }
    pub fn fixed_dc(mut self, value: DayCount) -> Self {
        self.fixed_dc = Some(value);
        self
    }
    pub fn fixed_bdc(mut self, value: BusinessDayConvention) -> Self {
        self.fixed_bdc = Some(value);
        self
    }
    pub fn fixed_calendar_id(mut self, value: &'static str) -> Self {
        self.fixed_calendar_id = Some(value);
        self
    }
    pub fn fixed_stub(mut self, value: StubKind) -> Self {
        self.fixed_stub = Some(value);
        self
    }
    pub fn fixed_start(mut self, value: Date) -> Self {
        self.fixed_start = Some(value);
        self
    }
    pub fn fixed_end(mut self, value: Date) -> Self {
        self.fixed_end = Some(value);
        self
    }

    // Float leg setters
    pub fn float_disc_id(mut self, value: &'static str) -> Self {
        self.float_disc_id = Some(value);
        self
    }
    pub fn float_fwd_id(mut self, value: &'static str) -> Self {
        self.float_fwd_id = Some(value);
        self
    }
    pub fn float_spread_bp(mut self, value: F) -> Self {
        self.float_spread_bp = Some(value);
        self
    }
    pub fn float_freq(mut self, value: Frequency) -> Self {
        self.float_freq = Some(value);
        self
    }
    pub fn float_dc(mut self, value: DayCount) -> Self {
        self.float_dc = Some(value);
        self
    }
    pub fn float_bdc(mut self, value: BusinessDayConvention) -> Self {
        self.float_bdc = Some(value);
        self
    }
    pub fn float_calendar_id(mut self, value: &'static str) -> Self {
        self.float_calendar_id = Some(value);
        self
    }
    pub fn float_stub(mut self, value: StubKind) -> Self {
        self.float_stub = Some(value);
        self
    }
    pub fn float_start(mut self, value: Date) -> Self {
        self.float_start = Some(value);
        self
    }
    pub fn float_end(mut self, value: Date) -> Self {
        self.float_end = Some(value);
        self
    }

    /// Convenience method to set both legs to the same start/end dates
    pub fn dates(mut self, start: Date, end: Date) -> Self {
        self.fixed_start = Some(start);
        self.fixed_end = Some(end);
        self.float_start = Some(start);
        self.float_end = Some(end);
        self
    }

    /// Convenience method to set standard fixed leg defaults
    pub fn standard_fixed_leg(
        mut self,
        disc_id: &'static str,
        rate: F,
        freq: Frequency,
        dc: DayCount,
    ) -> Self {
        self.fixed_disc_id = Some(disc_id);
        self.fixed_rate = Some(rate);
        self.fixed_freq = Some(freq);
        self.fixed_dc = Some(dc);
        self.fixed_bdc = Some(BusinessDayConvention::ModifiedFollowing);
        self.fixed_stub = Some(StubKind::None);
        self
    }

    /// Convenience method to set standard float leg defaults
    pub fn standard_float_leg(
        mut self,
        disc_id: &'static str,
        fwd_id: &'static str,
        spread_bp: F,
        freq: Frequency,
        dc: DayCount,
    ) -> Self {
        self.float_disc_id = Some(disc_id);
        self.float_fwd_id = Some(fwd_id);
        self.float_spread_bp = Some(spread_bp);
        self.float_freq = Some(freq);
        self.float_dc = Some(dc);
        self.float_bdc = Some(BusinessDayConvention::ModifiedFollowing);
        self.float_stub = Some(StubKind::None);
        self
    }

    pub fn build(self) -> finstack_core::Result<InterestRateSwap> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let side = self
            .side
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        // Build fixed leg spec
        let fixed = FixedLegSpec {
            disc_id: self.fixed_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            rate: self.fixed_rate.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            freq: self.fixed_freq.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.fixed_dc.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            bdc: self
                .fixed_bdc
                .unwrap_or(BusinessDayConvention::ModifiedFollowing),
            calendar_id: self.fixed_calendar_id,
            stub: self.fixed_stub.unwrap_or(StubKind::None),
            start: self.fixed_start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            end: self.fixed_end.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
        };

        // Build float leg spec
        let float = FloatLegSpec {
            disc_id: self.float_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            fwd_id: self.float_fwd_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            spread_bp: self.float_spread_bp.unwrap_or(0.0),
            freq: self.float_freq.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.float_dc.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            bdc: self
                .float_bdc
                .unwrap_or(BusinessDayConvention::ModifiedFollowing),
            calendar_id: self.float_calendar_id,
            stub: self.float_stub.unwrap_or(StubKind::None),
            start: self.float_start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            end: self.float_end.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
        };

        Ok(InterestRateSwap {
            id,
            notional,
            side,
            fixed,
            float,
            attributes: Attributes::new(),
        })
    }
}
