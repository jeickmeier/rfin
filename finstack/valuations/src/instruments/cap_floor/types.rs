//! Interest rate option instrument types and Black model greeks.

use crate::instruments::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, SettlementType};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::F;

use super::parameters::InterestRateOptionParams;

/// Type of interest rate option
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateOptionType {
    /// Cap (series of caplets)
    Cap,
    /// Floor (series of floorlets)
    Floor,
    /// Caplet (single period cap)
    Caplet,
    /// Floorlet (single period floor)
    Floorlet,
}

/// Interest rate option instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct InterestRateOption {
    /// Unique instrument identifier
    pub id: String,
    /// Option type
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate (as decimal, e.g., 0.05 for 5%)
    pub strike_rate: F,
    /// Start date of underlying period
    pub start_date: Date,
    /// End date of underlying period
    pub end_date: Date,
    /// Payment frequency for caps/floors
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Schedule stub convention
    pub stub_kind: StubKind,
    /// Schedule business day convention
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier for schedule and roll conventions
    pub calendar_id: Option<&'static str>,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InterestRateOption {
    /// Create a new interest rate option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &InterestRateOptionParams,
        start_date: Date,
        end_date: Date,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            rate_option_type: option_params.rate_option_type,
            notional: option_params.notional,
            strike_rate: option_params.strike_rate,
            start_date,
            end_date,
            frequency: option_params.frequency,
            day_count: option_params.day_count,
            stub_kind: option_params.stub_kind,
            bdc: option_params.bdc,
            calendar_id: option_params.calendar_id,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            disc_id,
            forward_id,
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a cap instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_cap(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::cap(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            disc_id,
            forward_id,
            vol_id,
        )
    }

    /// Create a floor instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_floor(
        id: impl Into<String>,
        notional: Money,
        strike_rate: F,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::floor(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            disc_id,
            forward_id,
            vol_id,
        )
    }
}

impl_instrument!(
    InterestRateOption,
    "InterestRateOption",
    pv = |s, curves, as_of| {
        // Delegate PV to pricing engine for structure parity with other instruments
        let pricer = crate::instruments::cap_floor::pricing::engine::IrOptionPricer::new();
        pricer.price(s, curves, as_of)
    }
);
