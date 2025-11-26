//! Thin facade for bond cashflow specification.
//!
//! This module provides a clean, ergonomic API for bonds by wrapping the canonical
//! builder coupon specs (`FixedCouponSpec`, `FloatingCouponSpec`) with convenience
//! constructors that apply sensible defaults.
//!
//! # Features
//!
//! - Fixed-rate bonds with configurable coupon rates and frequencies
//! - Floating-rate notes (FRNs) with index spreads and margins
//! - Amortizing bonds with custom principal repayment schedules
//! - Full parity with builder coupon specs (floors/caps, BDC, calendars, PIK, etc.)
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::instruments::bond::CashflowSpec;
//! use finstack_core::dates::{Frequency, DayCount};
//!
//! // Fixed-rate bond: 5% annual coupon, semi-annual payments
//! let fixed = CashflowSpec::fixed(0.05, Frequency::semi_annual(), DayCount::Thirty360);
//!
//! // Floating-rate note: SOFR + 200bps, quarterly payments
//! let floating = CashflowSpec::floating(
//!     "USD-SOFR-3M".into(),
//!     200.0,  // margin in basis points
//!     Frequency::quarterly(),
//!     DayCount::Act360,
//! );
//! ```
//!
//! # See Also
//!
//! - [`Bond`] for bond construction using cashflow specs
//! - [`crate::cashflow::builder::specs`] for full builder coupon specifications

use crate::cashflow::builder::specs::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
};
use crate::cashflow::builder::AmortizationSpec;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::types::CurveId;

/// Thin facade over canonical builder coupon specs for bond cashflows.
///
/// Wraps `FixedCouponSpec` and `FloatingCouponSpec` from the cashflow builder,
/// providing convenience constructors with sensible defaults for common bond use cases.
/// This ensures parity with all builder features (floors/caps, BDC, calendars, PIK, etc.)
/// while keeping the bond API simple.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CashflowSpec {
    /// Fixed-rate bond using the canonical `FixedCouponSpec`.
    Fixed(FixedCouponSpec),

    /// Floating-rate note using the canonical `FloatingCouponSpec`.
    Floating(FloatingCouponSpec),

    /// Amortizing bond (principal payments during life).
    Amortizing {
        /// Base cashflow specification (fixed or floating).
        base: Box<CashflowSpec>,
        /// Amortization schedule.
        schedule: AmortizationSpec,
    },
}

impl CashflowSpec {
    /// Create a fixed-rate specification with sensible defaults.
    ///
    /// # Arguments
    ///
    /// * `coupon` - Annual coupon rate as decimal (e.g., 0.05 for 5%)
    /// * `freq` - Payment frequency (e.g., `Frequency::semi_annual()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Thirty360`)
    ///
    /// # Defaults
    ///
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: None
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Fixed` variant with the specified coupon rate, frequency, and day count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond::CashflowSpec;
    /// use finstack_core::dates::{Frequency, DayCount};
    ///
    /// // US Treasury-style: 4% coupon, semi-annual, 30/360
    /// let spec = CashflowSpec::fixed(0.04, Frequency::semi_annual(), DayCount::Thirty360);
    /// ```
    ///
    /// # See Also
    ///
    /// For full control (PIK, custom calendars, stubs), construct `FixedCouponSpec` directly
    /// and wrap in `CashflowSpec::Fixed(...)`.
    pub fn fixed(coupon: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: coupon,
            freq,
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        })
    }

    /// Create a floating-rate specification with sensible defaults.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `margin_bp` - Spread over index in basis points (e.g., 200.0 for 200bps)
    /// * `freq` - Payment frequency (e.g., `Frequency::quarterly()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Act360`)
    ///
    /// # Defaults
    ///
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `gearing`: 1.0
    /// - `reset_lag_days`: 2 (T-2 convention, standard for SOFR)
    /// - `floor_bp`: None
    /// - `cap_bp`: None
    /// - `reset_freq`: Same as payment frequency
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: None
    ///
    /// # Market Conventions for Reset Lag
    ///
    /// Different indices use different reset lag conventions:
    /// - **SOFR**: T-2 (2 business days before period start)
    /// - **EURIBOR**: T-2
    /// - **LIBOR (legacy)**: T-0 to T-2 depending on currency
    /// - **SONIA**: T-0 (same day)
    ///
    /// Use `floating_with_reset_lag()` to specify a non-default reset lag.
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Floating` variant with the specified index, margin, frequency, and day count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond::CashflowSpec;
    /// use finstack_core::dates::{Frequency, DayCount};
    /// use finstack_core::types::CurveId;
    ///
    /// // FRN: 3M SOFR + 200bps, quarterly payments (default T-2 reset)
    /// let spec = CashflowSpec::floating(
    ///     CurveId::new("USD-SOFR-3M"),
    ///     200.0,  // 200 basis points
    ///     Frequency::quarterly(),
    ///     DayCount::Act360,
    /// );
    /// ```
    ///
    /// # See Also
    ///
    /// - `floating_with_reset_lag()` for custom reset lag
    /// - For full control (floors/caps/gearing), construct `FloatingCouponSpec` directly
    ///   and wrap in `CashflowSpec::Floating(...)`.
    pub fn floating(index_id: CurveId, margin_bp: f64, freq: Frequency, dc: DayCount) -> Self {
        Self::floating_with_reset_lag(index_id, margin_bp, freq, dc, 2)
    }

    /// Create a floating-rate specification with explicit reset lag.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `margin_bp` - Spread over index in basis points (e.g., 200.0 for 200bps)
    /// * `freq` - Payment frequency (e.g., `Frequency::quarterly()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Act360`)
    /// * `reset_lag_days` - Number of business days before period start for rate fixing
    ///
    /// # Market Conventions for Reset Lag
    ///
    /// | Index | Standard Reset Lag |
    /// |-------|-------------------|
    /// | SOFR | T-2 (2 days) |
    /// | EURIBOR | T-2 (2 days) |
    /// | SONIA | T-0 (same day) |
    /// | TONA | T-2 (2 days) |
    /// | LIBOR (legacy) | T-0 to T-2 |
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Floating` variant with the specified parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond::CashflowSpec;
    /// use finstack_core::dates::{Frequency, DayCount};
    /// use finstack_core::types::CurveId;
    ///
    /// // SONIA-linked FRN with T-0 reset (same day fixing)
    /// let sonia_frn = CashflowSpec::floating_with_reset_lag(
    ///     CurveId::new("GBP-SONIA"),
    ///     150.0,  // 150 basis points
    ///     Frequency::quarterly(),
    ///     DayCount::Act365F,
    ///     0,  // T-0 reset for SONIA
    /// );
    ///
    /// // SOFR-linked FRN with standard T-2 reset
    /// let sofr_frn = CashflowSpec::floating_with_reset_lag(
    ///     CurveId::new("USD-SOFR-3M"),
    ///     200.0,
    ///     Frequency::quarterly(),
    ///     DayCount::Act360,
    ///     2,  // T-2 reset for SOFR
    /// );
    /// ```
    pub fn floating_with_reset_lag(
        index_id: CurveId,
        margin_bp: f64,
        freq: Frequency,
        dc: DayCount,
        reset_lag_days: i32,
    ) -> Self {
        Self::Floating(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id,
                spread_bp: margin_bp,
                gearing: 1.0,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: freq,
                reset_lag_days,
                dc,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                fixing_calendar_id: None,
            },
            coupon_type: CouponType::Cash,
            freq,
            stub: StubKind::None,
        })
    }

    /// Create an amortizing bond specification.
    ///
    /// Combines a base cashflow specification (fixed or floating) with an amortization
    /// schedule that specifies principal repayments during the bond's life.
    ///
    /// # Arguments
    ///
    /// * `base` - Base cashflow specification (fixed or floating) for coupon payments
    /// * `schedule` - Amortization schedule specifying principal repayment dates and amounts
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Amortizing` variant combining the base spec with the amortization schedule.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond::CashflowSpec;
    /// use finstack_valuations::cashflow::builder::AmortizationSpec;
    /// use finstack_core::dates::{Frequency, DayCount, Date};
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use time::Month;
    ///
    /// // Base fixed-rate spec
    /// let base = CashflowSpec::fixed(0.05, Frequency::annual(), DayCount::Act365F);
    ///
    /// // Amortization: 1/3 principal each year
    /// let step1 = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    /// let step2 = Date::from_calendar_date(2027, Month::January, 1).unwrap();
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1).unwrap();
    /// let amort = AmortizationSpec::StepRemaining {
    ///     schedule: vec![
    ///         (step1, Money::new(333_333.33, Currency::USD)),
    ///         (step2, Money::new(666_666.67, Currency::USD)),
    ///         (maturity, Money::new(0.0, Currency::USD)),
    ///     ],
    /// };
    ///
    /// let amortizing_spec = CashflowSpec::amortizing(base, amort);
    /// ```
    pub fn amortizing(base: CashflowSpec, schedule: AmortizationSpec) -> Self {
        Self::Amortizing {
            base: Box::new(base),
            schedule,
        }
    }

    /// Get the payment frequency from this specification.
    ///
    /// # Returns
    ///
    /// The payment frequency (e.g., `Frequency::semi_annual()`).
    ///
    /// For amortizing bonds, returns the frequency from the base specification.
    pub fn frequency(&self) -> Frequency {
        match self {
            Self::Fixed(spec) => spec.freq,
            Self::Floating(spec) => spec.freq,
            Self::Amortizing { base, .. } => base.frequency(),
        }
    }

    /// Get the day count convention from this specification.
    ///
    /// # Returns
    ///
    /// The day count convention (e.g., `DayCount::Thirty360`).
    ///
    /// For amortizing bonds, returns the day count from the base specification.
    pub fn day_count(&self) -> DayCount {
        match self {
            Self::Fixed(spec) => spec.dc,
            Self::Floating(spec) => spec.rate_spec.dc,
            Self::Amortizing { base, .. } => base.day_count(),
        }
    }
}

impl Default for CashflowSpec {
    /// Default to semi-annual fixed bond with 30/360 day count (US convention).
    fn default() -> Self {
        Self::fixed(0.0, Frequency::semi_annual(), DayCount::Thirty360)
    }
}
