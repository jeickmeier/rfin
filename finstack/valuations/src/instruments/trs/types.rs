//! Core types and common engine for Total Return Swaps.

use crate::cashflow::builder::schedule_utils::build_dates;
use crate::cashflow::builder::ScheduleParams;
use finstack_core::types::id::IndexId;
use finstack_core::types::Currency;
use finstack_core::{
    dates::{Date, DayCount},
    types::CurveId,
    F,
};
// Forward trait removed - use direct method calls on curve types

/// Side of the TRS trade from the party's perspective.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::trs::TrsSide;
///
/// let receive_side = TrsSide::ReceiveTotalReturn;
/// let pay_side = TrsSide::PayTotalReturn;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrsSide {
    /// Receive total return, pay financing.
    ReceiveTotalReturn,
    /// Pay total return, receive financing.
    PayTotalReturn,
}

impl TrsSide {
    /// Gets the sign multiplier for present value calculation.
    ///
    /// # Returns
    /// 1.0 for ReceiveTotalReturn, -1.0 for PayTotalReturn.
    pub fn sign(&self) -> F {
        match self {
            TrsSide::ReceiveTotalReturn => 1.0,
            TrsSide::PayTotalReturn => -1.0,
        }
    }
}

/// Specification for the financing leg of a TRS.
///
/// Defines the floating rate leg that pays/receives a spread over a reference rate.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::trs::FinancingLegSpec;
/// use finstack_core::dates::DayCount;
///
/// let financing = FinancingLegSpec::new(
///     "USD-OIS",
///     "USD-SOFR-3M",
///     25.0, // 25bp spread
///     DayCount::Act365F
/// );
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FinancingLegSpec {
    /// Discount curve identifier for present value calculations.
    pub disc_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M).
    pub fwd_id: CurveId,
    /// Spread in basis points over the floating rate.
    pub spread_bp: F,
    /// Day count convention for accrual calculations.
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Creates a new financing leg specification.
    ///
    /// # Arguments
    /// * `disc_id` — Discount curve identifier
    /// * `fwd_id` — Forward curve identifier
    /// * `spread_bp` — Spread in basis points over the floating rate
    /// * `day_count` — Day count convention for accrual
    ///
    /// # Returns
    /// New FinancingLegSpec instance.
    pub fn new(
        disc_id: impl Into<String>,
        fwd_id: impl Into<String>,
        spread_bp: F,
        day_count: DayCount,
    ) -> Self {
        Self {
            disc_id: CurveId::new(disc_id),
            fwd_id: CurveId::new(fwd_id),
            spread_bp,
            day_count,
        }
    }
}

/// Specification for the total return leg of a TRS.
///
/// Defines the leg that pays/receives the total return of the underlying asset.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TotalReturnLegSpec {
    /// Reference index or asset identifier.
    pub reference_id: String,
    /// Initial price/level (if known, otherwise fetched from market).
    pub initial_level: Option<F>,
    /// Whether to include dividends/distributions in the return calculation.
    pub include_distributions: bool,
}

/// Schedule specification for TRS payment periods.
///
/// Defines the payment schedule and frequency for both legs of the TRS.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TrsScheduleSpec {
    /// Start date for the TRS leg.
    pub start: Date,
    /// End date for the TRS leg.
    pub end: Date,
    /// Schedule parameters (frequency, day count, bdc, calendar, stub).
    pub params: ScheduleParams,
}

impl TrsScheduleSpec {
    /// Creates a schedule specification from start/end dates and schedule parameters.
    ///
    /// # Arguments
    /// * `start` — Start date for the TRS leg
    /// * `end` — End date for the TRS leg
    /// * `schedule` — Schedule parameters (frequency, day count, etc.)
    ///
    /// # Returns
    /// New TrsScheduleSpec instance.
    pub fn from_params(start: Date, end: Date, schedule: ScheduleParams) -> Self {
        Self {
            start,
            end,
            params: schedule,
        }
    }

    /// Builds the period date schedule in a canonical way.
    ///
    /// # Returns
    /// PeriodSchedule containing all payment dates for the TRS.
    pub fn period_schedule(&self) -> crate::cashflow::builder::schedule_utils::PeriodSchedule {
        build_dates(
            self.start,
            self.end,
            self.params.freq,
            self.params.stub,
            self.params.bdc,
            self.params.calendar_id,
        )
    }
}

/// Parameters for fixed income index underlying (for TRS and similar instruments).
///
/// Defines the underlying fixed income index and its associated market data identifiers.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::trs::IndexUnderlyingParams;
/// use finstack_core::currency::Currency;
///
/// let params = IndexUnderlyingParams::new("CDX.IG", Currency::USD)
///     .with_yield("CDX.IG.YIELD")
///     .with_duration("CDX.IG.DURATION");
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexUnderlyingParams {
    /// Index identifier (e.g., "CDX.IG", "HY.BOND.INDEX").
    pub index_id: IndexId,
    /// Base currency of the index.
    pub base_currency: Currency,
    /// Optional yield curve/scalar identifier for carry calculation.
    pub yield_id: Option<String>,
    /// Optional duration identifier for risk calculations.
    pub duration_id: Option<String>,
    /// Optional convexity identifier for risk calculations.
    pub convexity_id: Option<String>,
    /// Contract size (index units per contract, defaults to 1.0).
    pub contract_size: F,
}

impl IndexUnderlyingParams {
    /// Creates index underlying parameters.
    ///
    /// # Arguments
    /// * `index_id` — Index identifier
    /// * `base_currency` — Base currency of the index
    ///
    /// # Returns
    /// New IndexUnderlyingParams with default values.
    pub fn new(index_id: impl Into<String>, base_currency: Currency) -> Self {
        Self {
            index_id: IndexId::new(index_id),
            base_currency,
            yield_id: None,
            duration_id: None,
            convexity_id: None,
            contract_size: 1.0,
        }
    }

    /// Sets the yield identifier for carry calculation.
    ///
    /// # Arguments
    /// * `yield_id` — Yield curve or scalar identifier
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_yield(mut self, yield_id: impl Into<String>) -> Self {
        self.yield_id = Some(yield_id.into());
        self
    }

    /// Sets the duration identifier for risk calculations.
    ///
    /// # Arguments
    /// * `duration_id` — Duration scalar identifier
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_duration(mut self, duration_id: impl Into<String>) -> Self {
        self.duration_id = Some(duration_id.into());
        self
    }

    /// Sets the convexity identifier for risk calculations.
    ///
    /// # Arguments
    /// * `convexity_id` — Convexity scalar identifier
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_convexity(mut self, convexity_id: impl Into<String>) -> Self {
        self.convexity_id = Some(convexity_id.into());
        self
    }

    /// Sets the contract size multiplier.
    ///
    /// # Arguments
    /// * `size` — Contract size (index units per contract)
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_contract_size(mut self, size: F) -> Self {
        self.contract_size = size;
        self
    }
}
