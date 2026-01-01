//! Market convention definitions for indices, options, and credit.
//!
//! This module defines the data structures for all market convention types. Conventions capture
//! market-standard parameters such as day count conventions, business day adjustments, payment
//! frequencies, and settlement lags that are required for accurate instrument construction
//! and pricing.

use crate::instruments::irs::FloatingLegCompounding;
use crate::market::conventions::ids::IndexId;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::Currency;
use serde::{Deserialize, Serialize};

/// Type of rate index for convention determination.
///
/// Distinguishes between overnight risk-free rate (RFR) indices and term indices, which have
/// different conventions for compounding, payment frequencies, and reset lags.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::RateIndexKind;
///
/// let overnight = RateIndexKind::OvernightRfr;
/// let term = RateIndexKind::Term;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateIndexKind {
    /// Overnight Risk-Free Rate index (e.g., SOFR, SONIA, ESTR).
    ///
    /// These indices require compounding conventions and typically use OIS-style swap conventions.
    OvernightRfr,
    /// Term index with a fixed period (e.g., 3M LIBOR, 6M EURIBOR).
    ///
    /// These indices have fixed reset periods and use standard swap conventions.
    Term,
}

/// Convention details for pricing instruments tied to a rate index.
///
/// This structure captures all market-standard parameters for instruments referencing a rate
/// index, including day count, business day conventions, payment frequencies, and settlement
/// lags. Used by builders to construct deposits, FRAs, swaps, and other rate instruments.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::{RateIndexConventions, RateIndexKind};
/// use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
/// use finstack_core::currency::Currency;
///
/// // In practice, conventions are loaded from the registry
/// // let conv = registry.require_rate_index(&IndexId::new("USD-SOFR-OIS"))?;
/// ```
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
///
/// Defines market-standard parameters for CDS instruments, including payment frequencies,
/// day count conventions, business day adjustments, and settlement lags. Used by CDS builders
/// to construct instruments with correct market conventions.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::CdsConventions;
/// use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
///
/// // In practice, conventions are loaded from the registry
/// // let conv = registry.require_cds(&CdsConventionKey { ... })?;
/// ```
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
///
/// Defines market-standard parameters for option instruments, including settlement calendars,
/// business day conventions, and settlement lags. Used by option builders to construct
/// instruments with correct market conventions.
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
///
/// Defines market-standard parameters for swaption instruments, including exercise calendars,
/// business day conventions, fixed leg conventions, and floating leg index references. Used
/// by swaption builders to construct instruments with correct market conventions.
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
///
/// Defines market-standard parameters for inflation swap instruments, including payment
/// calendars, business day conventions, day count conventions, and inflation lag periods.
/// Used by inflation swap builders to construct instruments with correct market conventions.
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
///
/// Defines market-standard parameters for interest rate future contracts, including contract
/// specifications (face value, tick size, tick value), delivery months, settlement lags, and
/// optional convexity adjustments. Used by futures builders to construct instruments with
/// correct market conventions.
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
