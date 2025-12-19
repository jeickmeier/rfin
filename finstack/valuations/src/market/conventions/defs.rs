use crate::instruments::irs::FloatingLegCompounding;
use crate::market::conventions::ids::IndexId;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::Currency;
use serde::{Deserialize, Serialize};

/// Type of rate index for convention determination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateIndexKind {
    /// Overnight Risk-Free Rate index (e.g., SOFR, SONIA, ESTR).
    OvernightRfr,
    /// Term index with a fixed period (e.g., 3M LIBOR, 6M EURIBOR).
    Term,
}

/// Convention details for pricing instruments tied to a rate index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RateIndexConventions {
    /// Operating currency of the index.
    pub currency: Currency,
    /// Index category (Overnight vs Term).
    pub kind: RateIndexKind,
    /// Index tenor (None for overnight indices).
    pub tenor: Option<Tenor>,
    /// Market standard day count convention.
    pub day_count: DayCount,
    /// Typical payment frequency for swaps referencing this index.
    pub default_payment_frequency: Tenor,
    /// Business days between accrual end and payment.
    pub default_payment_delay_days: i32,
    /// Business days between fixing and accrual start.
    pub default_reset_lag_days: i32,
    /// Methodology for compounding overnight rates (OIS only).
    pub ois_compounding: Option<FloatingLegCompounding>,

    // Swap market defaults
    /// Market-standard calendar identifier.
    pub market_calendar_id: String,
    /// Market-standard spot settlement lag (business days).
    pub market_settlement_days: i32,
    /// Market-standard business day convention.
    pub market_business_day_convention: BusinessDayConvention,
    /// Market-standard fixed leg day count.
    pub default_fixed_leg_day_count: DayCount,
    /// Market-standard fixed leg frequency.
    pub default_fixed_leg_frequency: Tenor,
}

/// Conventions for Credit Default Swaps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CdsConventions {
    /// The calendar used for business day adjustments.
    pub calendar_id: String,
    /// The day count convention for the premium leg.
    pub day_count: DayCount,
    /// The business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// The number of business days for settlement.
    pub settlement_days: i32,
    /// The payment frequency of the premium leg.
    pub payment_frequency: Tenor,
}

/// Conventions for Options (Equity/Commodity/FX).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionConventions {
    /// Calendar for exercise and settlement.
    pub calendar_id: String,
    /// Settlement lag in business days.
    pub settlement_days: i32,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
}

/// Conventions for Swaptions (Volatility Surfaces).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwaptionConventions {
    /// Calendar for exercise and settlement.
    pub calendar_id: String,
    /// Settlement lag in business days.
    pub settlement_days: i32,
    /// Business day convention for dates.
    pub business_day_convention: BusinessDayConvention,
    /// Fixed leg payment frequency.
    pub fixed_leg_frequency: Tenor,
    /// Fixed leg day count.
    pub fixed_leg_day_count: DayCount,
    /// Floating leg index (implies float leg conventions).
    pub float_leg_index: String,
}

/// Conventions for Inflation Swaps (ZCIS).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InflationSwapConventions {
    /// Calendar for payment/fixing.
    pub calendar_id: String,
    /// Settlement lag in business days.
    pub settlement_days: i32,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Day count for the fixed leg.
    pub day_count: DayCount,
    /// Inflation lag (observation lag) in months/period.
    pub inflation_lag: Tenor,
}

/// Conventions for Interest Rate Futures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IrFutureConventions {
    /// Underlying rate index identifier.
    pub index_id: IndexId,
    /// Calendar for business day adjustments.
    pub calendar_id: String,
    /// Settlement lag in business days between expiry and period start.
    pub settlement_days: i32,
    /// Number of delivery months for the underlying rate period.
    pub delivery_months: u8,
    /// Face value of the contract.
    pub face_value: f64,
    /// Tick size in price points.
    pub tick_size: f64,
    /// Tick value in currency units.
    pub tick_value: f64,
    /// Optional convexity adjustment in rate terms.
    #[serde(default)]
    pub convexity_adjustment: Option<f64>,
}
