//! Bond builder for flexible construction.

use super::{Bond, CallPutSchedule};
use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::AmortizationSpec;
use crate::instruments::common::{DateRange, InstrumentScheduleParams, MarketRefs, PricingOverrides};
use finstack_core::prelude::*;
use finstack_core::F;

/// Enhanced bond builder using parameter groups and required fields.
///
/// Supports both traditional bond specification and custom cashflow schedules.
/// Reduces 10 optional fields to 4 required parameter groups.
#[derive(Default)]
pub struct BondBuilder {
    // Core required parameters
    id: Option<String>,
    notional: Option<Money>,
    coupon: Option<F>,
    
    // Parameter groups (required for traditional bonds)
    date_range: Option<DateRange>,
    schedule_params: Option<InstrumentScheduleParams>,
    market_refs: Option<MarketRefs>,
    
    // Optional parameters
    pricing_overrides: Option<PricingOverrides>,
    call_put: Option<CallPutSchedule>,
    amortization: Option<AmortizationSpec>,
    custom_cashflows: Option<CashFlowSchedule>,
}

impl BondBuilder {
    /// Create a new bond builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the bond identifier (required)
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the notional amount (required)
    pub fn notional(mut self, notional: Money) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Set the coupon rate (required)
    pub fn coupon(mut self, coupon: F) -> Self {
        self.coupon = Some(coupon);
        self
    }

    /// Set date range (required for traditional bonds)
    pub fn date_range(mut self, value: DateRange) -> Self {
        self.date_range = Some(value);
        self
    }

    /// Set dates directly
    pub fn dates(mut self, issue: Date, maturity: Date) -> Self {
        self.date_range = Some(DateRange::new(issue, maturity));
        self
    }

    /// Set date range from tenor
    pub fn tenor(mut self, issue: Date, tenor_years: F) -> Self {
        self.date_range = Some(DateRange::from_tenor(issue, tenor_years));
        self
    }

    /// Set schedule parameters (required for traditional bonds)
    pub fn schedule_params(mut self, value: InstrumentScheduleParams) -> Self {
        self.schedule_params = Some(value);
        self
    }

    /// Set market data references (required)
    pub fn market_refs(mut self, value: MarketRefs) -> Self {
        self.market_refs = Some(value);
        self
    }

    /// Set discount curve ID directly (convenience)
    pub fn disc_curve(mut self, disc_id: &'static str) -> Self {
        self.market_refs = Some(MarketRefs::discount_only(disc_id));
        self
    }

    /// Set pricing overrides (optional)
    pub fn pricing_overrides(mut self, value: PricingOverrides) -> Self {
        self.pricing_overrides = Some(value);
        self
    }

    /// Set quoted clean price (convenience)
    pub fn quoted_clean(mut self, price: F) -> Self {
        self.pricing_overrides = Some(
            self.pricing_overrides
                .unwrap_or_default()
                .with_clean_price(price)
        );
        self
    }

    /// Set call/put schedule (optional)
    pub fn call_put(mut self, schedule: CallPutSchedule) -> Self {
        self.call_put = Some(schedule);
        self
    }

    /// Set amortization specification (optional)
    pub fn amortization(mut self, spec: AmortizationSpec) -> Self {
        self.amortization = Some(spec);
        self
    }

    /// Set custom cashflow schedule (overrides traditional bond parameters)
    ///
    /// When provided, this overrides coupon generation from the bond's
    /// coupon rate and schedule specifications.
    pub fn cashflows(mut self, schedule: CashFlowSchedule) -> Self {
        // Extract parameters from the schedule if not already set
        if self.notional.is_none() {
            self.notional = Some(schedule.notional.initial);
        }

        // Extract issue and maturity dates if not set
        let dates = schedule.dates();
        if !dates.is_empty() {
            if self.date_range.is_none() {
                self.date_range = Some(DateRange::new(dates[0], *dates.last().unwrap()));
            }
            // Convert day count to schedule params
            if self.schedule_params.is_none() {
                self.schedule_params = Some(InstrumentScheduleParams {
                    frequency: finstack_core::dates::Frequency::semi_annual(), // Default
                    day_count: schedule.day_count,
                    bdc: finstack_core::dates::BusinessDayConvention::Following,
                    calendar_id: None,
                    stub: finstack_core::dates::StubKind::None,
                });
            }
        }

        self.custom_cashflows = Some(schedule);
        self
    }

    /// Build the bond instance.
    pub fn build(self) -> finstack_core::Result<Bond> {
        // Handle custom cashflows case
        if let Some(ref custom) = self.custom_cashflows {
            // Extract parameters from custom schedule
            let id = self.id.ok_or_else(|| {
                finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                    id: "bond_id".to_string(),
                })
            })?;
            let notional = self.notional.unwrap_or(custom.notional.initial);
            let dates = custom.dates();
            if dates.len() < 2 {
                return Err(finstack_core::error::InputError::TooFewPoints.into());
            }
            let issue = dates[0];
            let maturity = *dates.last().unwrap();
            let dc = custom.day_count;
            let market_refs = self.market_refs.unwrap_or_else(|| MarketRefs::discount_only("USD-OIS"));
            let pricing = self.pricing_overrides.unwrap_or_default();

            return Ok(Bond {
                id,
                notional,
                coupon: self.coupon.unwrap_or(0.0),
                freq: finstack_core::dates::Frequency::semi_annual(), // Default
                dc,
                issue,
                maturity,
                disc_id: market_refs.disc_id,
                quoted_clean: pricing.quoted_clean_price,
                call_put: self.call_put,
                amortization: self.amortization,
                custom_cashflows: self.custom_cashflows,
                attributes: crate::instruments::traits::Attributes::new(),
            });
        }

        // Traditional bond case - validate all required parameter groups
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "bond_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "bond_notional".to_string(),
            })
        })?;
        let coupon = self.coupon.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "bond_coupon".to_string(),
            })
        })?;
        let date_range = self.date_range.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "bond_dates".to_string(),
            })
        })?;
        let schedule_params = self.schedule_params.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "schedule_params".to_string(),
            })
        })?;
        let market_refs = self.market_refs.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "market_refs".to_string(),
            })
        })?;

        // Apply pricing overrides if present
        let pricing = self.pricing_overrides.unwrap_or_default();

        Ok(Bond {
            id,
            notional,
            coupon,
            freq: schedule_params.frequency,
            dc: schedule_params.day_count,
            issue: date_range.start,
            maturity: date_range.end,
            disc_id: market_refs.disc_id,
            quoted_clean: pricing.quoted_clean_price,
            call_put: self.call_put,
            amortization: self.amortization,
            custom_cashflows: self.custom_cashflows,
            attributes: crate::instruments::traits::Attributes::new(),
        })
    }
}
