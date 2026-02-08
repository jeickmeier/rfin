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
//! use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
//! use finstack_core::dates::{Tenor, DayCount};
//! use finstack_core::types::Bps;
//!
//! // Fixed-rate bond: 5% annual coupon, semi-annual payments
//! let fixed = CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360);
//!
//! // Floating-rate note: SOFR + 200bps, quarterly payments
//! let floating = CashflowSpec::floating_bps(
//!     "USD-SOFR-3M".into(),
//!     Bps::new(200),  // margin in basis points
//!     Tenor::quarterly(),
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
use crate::market::conventions::ids::IndexId;
use crate::market::conventions::ConventionRegistry;
use crate::market::conventions::RateIndexConventions;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::types::{Bps, CurveId, Rate};
use rust_decimal::Decimal;

fn rate_index_defaults(index_id: &CurveId) -> Option<RateIndexConventions> {
    let registry = ConventionRegistry::try_global().ok()?;
    let id = IndexId::new(index_id.as_str());
    registry.require_rate_index(&id).ok().cloned()
}

/// Thin facade over canonical builder coupon specs for bond cashflows.
///
/// Wraps `FixedCouponSpec` and `FloatingCouponSpec` from the cashflow builder,
/// providing convenience constructors with sensible defaults for common bond use cases.
/// This ensures parity with all builder features (floors/caps, BDC, calendars, PIK, etc.)
/// while keeping the bond API simple.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// * `freq` - Payment frequency (e.g., `Tenor::semi_annual()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Thirty360`)
    ///
    /// # Defaults
    ///
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: "weekends_only"
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Fixed` variant with the specified coupon rate, frequency, and day count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
    /// use finstack_core::dates::{Tenor, DayCount};
    /// use finstack_core::types::Rate;
    ///
    /// // US Treasury-style: 4% coupon, semi-annual, 30/360
    /// let spec = CashflowSpec::fixed_rate(
    ///     Rate::from_percent(4.0),
    ///     Tenor::semi_annual(),
    ///     DayCount::Thirty360,
    /// );
    /// ```
    ///
    /// # See Also
    ///
    /// For full control (PIK, custom calendars, stubs), construct `FixedCouponSpec` directly
    /// and wrap in `CashflowSpec::Fixed(...)`.
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn fixed(coupon: f64, freq: Tenor, dc: DayCount) -> Self {
        // Convert f64 to Decimal for exact representation
        let rate = Decimal::try_from(coupon).unwrap_or(Decimal::ZERO);
        Self::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate,
            freq,
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: "weekends_only".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        })
    }

    /// Create a fixed-rate specification using a typed rate.
    pub fn fixed_rate(coupon: Rate, freq: Tenor, dc: DayCount) -> Self {
        let rate = Decimal::try_from(coupon.as_decimal()).unwrap_or(Decimal::ZERO);
        Self::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate,
            freq,
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: "weekends_only".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        })
    }

    /// Create a floating-rate specification with sensible defaults.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `margin_bp` - Spread over index in basis points (e.g., 200.0 for 200bps)
    /// * `freq` - Payment frequency (e.g., `Tenor::quarterly()`)
    /// * `dc` - Day count convention (e.g., `DayCount::Act360`)
    ///
    /// # Defaults
    ///
    /// - `coupon_type`: Cash (100% cash payment)
    /// - `gearing`: 1.0
    /// - `reset_lag_days`: Market default from index registry (fallback: T-2)
    /// - `floor_bp`: None
    /// - `cap_bp`: None
    /// - `reset_freq`: Same as payment frequency
    /// - `bdc`: Following
    /// - `stub`: None
    /// - `calendar_id`: Market default from index registry (fallback: "weekends_only")
    ///
    /// # Market Conventions for Reset Lag
    ///
    /// Different indices use different reset lag conventions:
    /// - **SOFR**: T-2 (2 business days before period start)
    /// - **EURIBOR**: T-2
    /// - **LIBOR (historical)**: T-0 to T-2 depending on currency
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
    /// use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
    /// use finstack_core::dates::{Tenor, DayCount};
    /// use finstack_core::types::CurveId;
    ///
    /// // FRN: 3M SOFR + 200bps, quarterly payments (default T-2 reset)
    /// let spec = CashflowSpec::floating(
    ///     CurveId::new("USD-SOFR-3M"),
    ///     200.0,  // 200 basis points
    ///     Tenor::quarterly(),
    ///     DayCount::Act360,
    /// );
    /// ```
    ///
    /// # See Also
    ///
    /// - `floating_with_reset_lag()` for custom reset lag
    /// - For full control (floors/caps/gearing), construct `FloatingCouponSpec` directly
    ///   and wrap in `CashflowSpec::Floating(...)`.
    #[allow(clippy::expect_used)] // Builder with valid inputs should not fail
    pub fn floating(index_id: CurveId, margin_bp: f64, freq: Tenor, dc: DayCount) -> Self {
        let reset_lag = rate_index_defaults(&index_id)
            .map(|conv| conv.default_reset_lag_days)
            .unwrap_or(2);
        Self::floating_with_reset_lag(index_id, margin_bp, freq, dc, reset_lag)
    }

    /// Create a floating-rate specification using a typed margin in basis points.
    pub fn floating_bps(index_id: CurveId, margin_bp: Bps, freq: Tenor, dc: DayCount) -> Self {
        let spread_bp = Decimal::try_from(margin_bp.as_bps() as f64).unwrap_or(Decimal::ZERO);
        let defaults = rate_index_defaults(&index_id);
        let reset_lag_days = defaults
            .as_ref()
            .map(|conv| conv.default_reset_lag_days)
            .unwrap_or(2);
        let calendar_id = defaults
            .map(|conv| conv.market_calendar_id)
            .unwrap_or_else(|| "weekends_only".to_string());
        Self::Floating(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id,
                spread_bp,
                gearing: Decimal::ONE,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: freq,
                reset_lag_days,
                dc,
                bdc: BusinessDayConvention::Following,
                calendar_id,
                fixing_calendar_id: None,
                end_of_month: false,
                payment_lag_days: 0,
                overnight_compounding: None,
            },
            coupon_type: CouponType::Cash,
            freq,
            stub: StubKind::None,
        })
    }

    /// Create a floating-rate specification with explicit reset lag.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `margin_bp` - Spread over index in basis points (e.g., 200.0 for 200bps)
    /// * `freq` - Payment frequency (e.g., `Tenor::quarterly()`)
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
    /// | LIBOR (historical) | T-0 to T-2 |
    ///
    /// # Returns
    ///
    /// A `CashflowSpec::Floating` variant with the specified parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
    /// use finstack_core::dates::{Tenor, DayCount};
    /// use finstack_core::types::CurveId;
    ///
    /// // SONIA-linked FRN with T-0 reset (same day fixing)
    /// let sonia_frn = CashflowSpec::floating_with_reset_lag(
    ///     CurveId::new("GBP-SONIA"),
    ///     150.0,  // 150 basis points
    ///     Tenor::quarterly(),
    ///     DayCount::Act365F,
    ///     0,  // T-0 reset for SONIA
    /// );
    ///
    /// // SOFR-linked FRN with standard T-2 reset
    /// let sofr_frn = CashflowSpec::floating_with_reset_lag(
    ///     CurveId::new("USD-SOFR-3M"),
    ///     200.0,
    ///     Tenor::quarterly(),
    ///     DayCount::Act360,
    ///     2,  // T-2 reset for SOFR
    /// );
    /// ```
    pub fn floating_with_reset_lag(
        index_id: CurveId,
        margin_bp: f64,
        freq: Tenor,
        dc: DayCount,
        reset_lag_days: i32,
    ) -> Self {
        // Convert f64 to Decimal for exact representation
        let spread_bp = Decimal::try_from(margin_bp).unwrap_or(Decimal::ZERO);
        let calendar_id = rate_index_defaults(&index_id)
            .map(|conv| conv.market_calendar_id)
            .unwrap_or_else(|| "weekends_only".to_string());
        Self::Floating(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id,
                spread_bp,
                gearing: Decimal::ONE,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: freq,
                reset_lag_days,
                dc,
                bdc: BusinessDayConvention::Following,
                calendar_id,
                fixing_calendar_id: None,
                end_of_month: false,
                payment_lag_days: 0,
                overnight_compounding: None,
            },
            coupon_type: CouponType::Cash,
            freq,
            stub: StubKind::None,
        })
    }

    /// Create a floating-rate specification with explicit reset lag using a typed margin.
    pub fn floating_with_reset_lag_bps(
        index_id: CurveId,
        margin_bp: Bps,
        freq: Tenor,
        dc: DayCount,
        reset_lag_days: i32,
    ) -> Self {
        let spread_bp = Decimal::try_from(margin_bp.as_bps() as f64).unwrap_or(Decimal::ZERO);
        let calendar_id = rate_index_defaults(&index_id)
            .map(|conv| conv.market_calendar_id)
            .unwrap_or_else(|| "weekends_only".to_string());
        Self::Floating(FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id,
                spread_bp,
                gearing: Decimal::ONE,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: freq,
                reset_lag_days,
                dc,
                bdc: BusinessDayConvention::Following,
                calendar_id,
                fixing_calendar_id: None,
                end_of_month: false,
                payment_lag_days: 0,
                overnight_compounding: None,
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
    /// use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
    /// use finstack_valuations::cashflow::builder::AmortizationSpec;
    /// use finstack_core::dates::{Tenor, DayCount, Date};
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use time::Month;
    ///
    /// // Base fixed-rate spec
    /// let base = CashflowSpec::fixed(0.05, Tenor::annual(), DayCount::Act365F);
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
    /// The payment frequency (e.g., `Tenor::semi_annual()`).
    ///
    /// For amortizing bonds, returns the frequency from the base specification.
    pub fn frequency(&self) -> Tenor {
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
        Self::fixed(0.0, Tenor::semi_annual(), DayCount::Thirty360)
    }
}
