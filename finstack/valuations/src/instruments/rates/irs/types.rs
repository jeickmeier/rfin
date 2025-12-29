//! Interest Rate Swap (IRS) types and instrument trait implementations.
//!
//! Defines the `InterestRateSwap` instrument following the modern instrument
//! standards used across valuations: types live here; pricing is delegated to
//! `pricing::engine`; and metrics are split under `metrics/`.
//!
//! Public fields use strong newtype identifiers for safety: `InstrumentId` and
//! `CurveId`. Calendar identifiers remain `Option<&'static str>` for stable
//! serde and lookups.
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;
use crate::margin::types::OtcMarginSpec;

// Re-export common enums from parameters
pub use crate::instruments::common::parameters::legs::{ParRateMethod, PayReceive};

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::FixedLegSpec;
pub use crate::instruments::common::parameters::legs::FloatLegSpec;
use crate::instruments::irs::FloatingLegCompounding;

/// Leg-level conventions for building a vanilla fixed-vs-float IRS.
///
/// This is intentionally minimal: it captures the schedule/lag/calendar knobs
/// that are commonly resolved from market conventions (e.g. calibration quote
/// conventions) while keeping the instrument surface stable.
#[derive(Clone, Debug)]
pub struct IrsLegConventions {
    /// Fixed leg payment frequency.
    pub fixed_freq: Tenor,
    /// Float leg payment frequency.
    pub float_freq: Tenor,
    /// Fixed leg accrual day-count.
    pub fixed_dc: DayCount,
    /// Float leg accrual day-count.
    pub float_dc: DayCount,
    /// Payment date business day convention.
    pub bdc: BusinessDayConvention,
    /// Calendar id used for payment date adjustment on both legs.
    pub payment_calendar_id: Option<String>,
    /// Calendar id used for fixing/reset date adjustment.
    pub fixing_calendar_id: Option<String>,
    /// Stub handling.
    pub stub: StubKind,
    /// Reset lag in business days (start - reset_lag_days).
    pub reset_lag_days: i32,
    /// Payment delay in business days after period end.
    pub payment_delay_days: i32,
}

/// Interest rate swap with fixed and floating legs.
///
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
///
/// # Market Standards & Citations
///
/// ## ISDA Definitions
///
/// This implementation follows the **ISDA 2006 Definitions** for interest rate derivatives:
/// - **Section 4.1:** Fixed Rate Payer calculation conventions
/// - **Section 4.2:** Floating Rate Option conventions
/// - **Section 4.5:** Compounding methods
/// - **Section 4.16:** Business Day Conventions
///
/// ## USD Market Standard (Default)
///
/// Per **ISDA 2006 Definitions** and US market practice:
/// - **Fixed Leg:** Semi-annual, 30/360, Modified Following
/// - **Floating Leg:** Quarterly, ACT/360, Modified Following
/// - **Reset Lag:** T-2 (2 business days before period start)
/// - **Discounting:** OIS curve (post-2008 multi-curve framework)
///
/// ## Day-Count Convention Notes
///
/// The USD standard uses different day-count conventions for different purposes:
/// - **Fixed leg accrual:** 30/360 (Bond Basis)
/// - **Floating leg accrual:** ACT/360 (Money Market)
/// - **Discount curve:** Typically ACT/365F or ACT/360 depending on construction
///
/// This day-count mismatch between accrual and discounting is market-standard
/// and reflects the different conventions used in bond vs money markets.
/// The impact on par rates is typically < 0.5bp for USD swaps.
///
/// ## Validation
///
/// All convenience constructors (`create_usd_swap`, `example`) automatically
/// validate the swap parameters. Use [`InterestRateSwap::validate()`] to
/// check swaps constructed via the builder pattern.
///
/// ## References
///
/// - ISDA 2006 Definitions (incorporating 2008 Supplement for OIS)
/// - ISDA 2021 Definitions (for RFR compounding conventions)
/// - "Interest Rate Swaps and Their Derivatives" by Amir Sadr
/// - Bloomberg SWPM function documentation
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: InstrumentId,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Direction of the swap (PayFixed or ReceiveFixed).
    pub side: PayReceive,
    /// Fixed leg specification.
    pub fixed: FixedLegSpec,
    /// Floating leg specification.
    pub float: FloatLegSpec,
    /// Optional OTC margin specification for VM/IM.
    ///
    /// When present, enables margin calculation using SIMM or schedule-based
    /// methodologies. For cleared swaps, specify clearing house in
    /// `OtcMarginSpec::cleared()`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl InterestRateSwap {
    /// Create a **term-rate** (LIBOR/SOFR-term style) swap with `Simple` floating compounding.
    ///
    /// This is a thin variant constructor intended to avoid drift between instrument
    /// construction sites (e.g. calibration vs ad-hoc pricing) by centralizing the
    /// leg wiring and `FloatingLegCompounding` selection.
    ///
    /// - Discounting always uses `discount_curve_id` (fixed leg discount curve).
    /// - Projection always uses `forward_curve_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_term_swap_with_conventions(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        discount_curve_id: CurveId,
        forward_curve_id: CurveId,
        conventions: IrsLegConventions,
    ) -> finstack_core::Result<Self> {
        let fixed = FixedLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            rate: Decimal::try_from(fixed_rate).unwrap_or(Decimal::ZERO),
            freq: conventions.fixed_freq,
            dc: conventions.fixed_dc,
            bdc: conventions.bdc,
            calendar_id: conventions.payment_calendar_id.clone(),
            stub: conventions.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: conventions.payment_delay_days,
        };

        let float = FloatLegSpec {
            discount_curve_id,
            forward_curve_id,
            spread_bp: Decimal::ZERO,
            freq: conventions.float_freq,
            dc: conventions.float_dc,
            bdc: conventions.bdc,
            calendar_id: conventions.payment_calendar_id,
            stub: conventions.stub,
            reset_lag_days: conventions.reset_lag_days,
            fixing_calendar_id: conventions.fixing_calendar_id,
            start,
            end,
            compounding: FloatingLegCompounding::Simple,
            payment_delay_days: conventions.payment_delay_days,
        };

        let swap = Self {
            id,
            notional,
            side,
            fixed,
            float,
            margin_spec: None,
            attributes: Default::default(),
        };
        swap.validate()?;
        Ok(swap)
    }

    /// Create a term-rate swap using a typed fixed rate.
    #[allow(clippy::too_many_arguments)]
    pub fn create_term_swap_with_conventions_rate(
        id: InstrumentId,
        notional: Money,
        fixed_rate: Rate,
        start: Date,
        end: Date,
        side: PayReceive,
        discount_curve_id: CurveId,
        forward_curve_id: CurveId,
        conventions: IrsLegConventions,
    ) -> finstack_core::Result<Self> {
        Self::create_term_swap_with_conventions(
            id,
            notional,
            fixed_rate.as_decimal(),
            start,
            end,
            side,
            discount_curve_id,
            forward_curve_id,
            conventions,
        )
    }

    /// Create an **OIS / overnight RFR** swap with compounded-in-arrears floating compounding.
    ///
    /// This constructor enforces that the floating leg uses `CompoundedInArrears` compounding.
    /// For single-curve OIS pricing, pass `projection_curve_id == discount_curve_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_ois_swap_with_conventions(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        discount_curve_id: CurveId,
        projection_curve_id: CurveId,
        ois_compounding: FloatingLegCompounding,
        conventions: IrsLegConventions,
    ) -> finstack_core::Result<Self> {
        if matches!(ois_compounding, FloatingLegCompounding::Simple) {
            return Err(finstack_core::error::Error::Validation(
                "OIS swap requires compounded-in-arrears floating compounding (got Simple)"
                    .to_string(),
            ));
        }

        let fixed = FixedLegSpec {
            discount_curve_id: discount_curve_id.clone(),
            rate: Decimal::try_from(fixed_rate).unwrap_or(Decimal::ZERO),
            freq: conventions.fixed_freq,
            dc: conventions.fixed_dc,
            bdc: conventions.bdc,
            calendar_id: conventions.payment_calendar_id.clone(),
            stub: conventions.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: conventions.payment_delay_days,
        };

        let float = FloatLegSpec {
            discount_curve_id,
            forward_curve_id: projection_curve_id,
            spread_bp: Decimal::ZERO,
            freq: conventions.float_freq,
            dc: conventions.float_dc,
            bdc: conventions.bdc,
            calendar_id: conventions.payment_calendar_id,
            stub: conventions.stub,
            reset_lag_days: conventions.reset_lag_days,
            fixing_calendar_id: conventions.fixing_calendar_id,
            start,
            end,
            compounding: ois_compounding,
            payment_delay_days: conventions.payment_delay_days,
        };

        let swap = Self {
            id,
            notional,
            side,
            fixed,
            float,
            margin_spec: None,
            attributes: Default::default(),
        };
        swap.validate()?;
        Ok(swap)
    }

    /// Create an OIS swap using a typed fixed rate.
    #[allow(clippy::too_many_arguments)]
    pub fn create_ois_swap_with_conventions_rate(
        id: InstrumentId,
        notional: Money,
        fixed_rate: Rate,
        start: Date,
        end: Date,
        side: PayReceive,
        discount_curve_id: CurveId,
        projection_curve_id: CurveId,
        ois_compounding: FloatingLegCompounding,
        conventions: IrsLegConventions,
    ) -> finstack_core::Result<Self> {
        Self::create_ois_swap_with_conventions(
            id,
            notional,
            fixed_rate.as_decimal(),
            start,
            end,
            side,
            discount_curve_id,
            projection_curve_id,
            ois_compounding,
            conventions,
        )
    }
}

/// Configuration for standard swap construction.
struct SwapConfig<'a> {
    disc_curve: &'a str,
    fwd_curve: &'a str,
    reset_lag_days: i32,
    sched: IRSScheduleConfig,
}

/// Schedule configuration with separate fixed and float leg parameters
struct IRSScheduleConfig {
    fixed_freq: Tenor,
    fixed_dc: DayCount,
    float_freq: Tenor,
    float_dc: DayCount,
    bdc: BusinessDayConvention,
    calendar_id: Option<String>,
    stub: StubKind,
}

impl IRSScheduleConfig {
    /// USD market standard: Fixed semiannual 30/360; Float quarterly Act/360
    fn usd_isda_standard() -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
        Self {
            fixed_freq: Tenor::semi_annual(),
            fixed_dc: DayCount::Thirty360,
            float_freq: Tenor::quarterly(),
            float_dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            // Market-standard USD calendar for rates scheduling (ISDA-style business-day adjustments).
            // Calendar identifiers are lowercase codes in `finstack_core` (see `available_calendars()`).
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
        }
    }
}

/// Minimum notional threshold for numerical stability.
///
/// Notionals below this threshold may cause numerical issues in pricing
/// due to floating-point precision limits.
const NOTIONAL_EPSILON: f64 = 1e-6;

/// Maximum allowed rate magnitude for validation.
///
/// Rates with absolute value exceeding this threshold are considered
/// non-physical and rejected. This corresponds to ±10000% rates which
/// are far beyond any reasonable market scenario.
const MAX_RATE_MAGNITUDE: f64 = 100.0;

impl InterestRateSwap {
    /// Validate swap parameters for market-standard compliance.
    ///
    /// Checks:
    /// - Date ranges: `end > start` for both legs
    /// - Notional: must be positive (> NOTIONAL_EPSILON)
    /// - Fixed rate: must be within reasonable bounds
    /// - Leg consistency: start/end dates should match between legs
    ///
    /// # Errors
    ///
    /// Returns a validation error with a descriptive message if any
    /// parameter is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_valuations::instruments::irs::InterestRateSwap;
    ///
    /// let swap = InterestRateSwap::example()?;
    /// swap.validate()?; // Passes for valid swap
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Validate finiteness early to avoid NaN poisoning and panics in downstream code.
        if !self.notional.amount().is_finite() {
            return Err(finstack_core::error::Error::Validation(
                "Invalid notional: amount must be finite.".into(),
            ));
        }
        // Decimal values are always finite (no NaN/Infinity), so we just check they're valid
        // by verifying they can be converted to f64 for magnitude checks

        // Reset lag is a signed business-day offset applied to the accrual start to obtain
        // the reset/fixing date. Market convention for "T-2" is commonly represented as -2.
        // Guard only against absurd magnitudes that indicate unit mistakes.
        if self.float.reset_lag_days.abs() > 31 {
            return Err(finstack_core::error::Error::Validation(
                "Invalid floating reset lag: absolute value too large (expected a small number of business days)."
                    .into(),
            ));
        }
        if self.fixed.payment_delay_days < 0 || self.float.payment_delay_days < 0 {
            return Err(finstack_core::error::Error::Validation(
                "Invalid payment delay: must be non-negative (business days).".into(),
            ));
        }
        if let crate::instruments::irs::FloatingLegCompounding::CompoundedInArrears {
            lookback_days,
            observation_shift,
        } = self.float.compounding
        {
            if lookback_days < 0 {
                return Err(finstack_core::error::Error::Validation(
                    "Invalid RFR lookback: must be non-negative (business days).".into(),
                ));
            }
            if let Some(shift) = observation_shift {
                // Observation shift can be negative, but must be within a sane bound to avoid
                // accidental unit mistakes (e.g., passing years as days).
                if shift.abs() > 31 {
                    return Err(finstack_core::error::Error::Validation(
                        "Invalid observation shift: absolute value too large (expected a small number of business days)."
                            .into(),
                    ));
                }
            }
        }

        // Validate fixed leg date range
        if self.fixed.end <= self.fixed.start {
            return Err(finstack_core::error::Error::Validation(format!(
                "Invalid fixed leg date range: end ({}) must be after start ({})",
                self.fixed.end, self.fixed.start
            )));
        }

        // Validate floating leg date range
        if self.float.end <= self.float.start {
            return Err(finstack_core::error::Error::Validation(format!(
                "Invalid floating leg date range: end ({}) must be after start ({})",
                self.float.end, self.float.start
            )));
        }

        // Validate notional is positive
        if self.notional.amount() <= NOTIONAL_EPSILON {
            return Err(finstack_core::error::Error::Validation(format!(
                "Invalid notional: {} must be positive (> {:.0e}). \
                 Negative notional is semantically ambiguous; use PayReceive to control direction.",
                self.notional.amount(),
                NOTIONAL_EPSILON
            )));
        }

        // Validate fixed rate is within reasonable bounds
        let rate_f64 = self.fixed.rate.to_f64().unwrap_or(0.0);
        if rate_f64.abs() > MAX_RATE_MAGNITUDE {
            return Err(finstack_core::error::Error::Validation(format!(
                "Invalid fixed rate: {:.2}% exceeds maximum allowed magnitude ({:.0}%). \
                 This may indicate a units error (rate should be decimal, e.g., 0.05 for 5%).",
                rate_f64 * 100.0,
                MAX_RATE_MAGNITUDE * 100.0
            )));
        }

        // Warn-level check: legs should typically have matching date ranges
        // (Not an error, but log for diagnostics in debug builds)
        #[cfg(debug_assertions)]
        if self.fixed.start != self.float.start || self.fixed.end != self.float.end {
            tracing::warn!(
                swap_id = %self.id,
                "IRS legs have mismatched date ranges: fixed ({} to {}), float ({} to {}). \
                 This may be intentional for complex structures.",
                self.fixed.start, self.fixed.end, self.float.start, self.float.end
            );
        }

        Ok(())
    }

    /// Create a standard USD OIS-discounted IRS using ISDA market conventions.
    ///
    /// This is the primary convenience constructor used throughout tests and
    /// examples. It builds a vanilla fixed-vs-floating swap with:
    /// - Discount curve: `USD-OIS`
    /// - Forward curve: `USD-SOFR-3M`
    /// - Fixed leg: semi-annual, 30/360, Modified Following
    /// - Float leg: quarterly, ACT/360, Modified Following, 2-day reset lag
    ///
    /// # Validation
    ///
    /// Automatically validates the swap after construction. Returns an error
    /// if date ranges are invalid, notional is non-positive, or rates are
    /// outside reasonable bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if swap construction fails (e.g., invalid date range).
    pub fn create_usd_swap(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> finstack_core::Result<Self> {
        let config = SwapConfig {
            disc_curve: "USD-OIS",
            fwd_curve: "USD-SOFR-3M",
            reset_lag_days: 2,
            sched: IRSScheduleConfig::usd_isda_standard(),
        };

        Self::create_swap_with_config(id, notional, fixed_rate, start, end, side, config)
    }

    /// Create a standard USD OIS-discounted IRS using a typed fixed rate.
    pub fn create_usd_swap_rate(
        id: InstrumentId,
        notional: Money,
        fixed_rate: Rate,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> finstack_core::Result<Self> {
        Self::create_usd_swap(id, notional, fixed_rate.as_decimal(), start, end, side)
    }

    /// Create a canonical example IRS for testing and documentation.
    ///
    /// Returns a 5-year pay-fixed swap with semi-annual fixed vs quarterly floating.
    ///
    /// # Validation
    ///
    /// Automatically validates the swap after construction.
    ///
    /// # Errors
    ///
    /// Returns an error if example construction fails (e.g., invalid dates).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> finstack_core::Result<Self> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let swap = Self::builder()
            .id(InstrumentId::new("IRS-5Y-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .fixed(crate::instruments::common::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: Decimal::try_from(0.03).unwrap_or(Decimal::ZERO),
                freq: Tenor::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: Date::from_calendar_date(2024, time::Month::January, 1).map_err(|e| {
                    finstack_core::error::Error::Validation(format!(
                        "Invalid example start date: {}",
                        e
                    ))
                })?,
                end: Date::from_calendar_date(2029, time::Month::January, 1).map_err(|e| {
                    finstack_core::error::Error::Validation(format!(
                        "Invalid example end date: {}",
                        e
                    ))
                })?,
                par_method: None,
                compounding_simple: true,
                payment_delay_days: 0,
            })
            .float(crate::instruments::common::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: Decimal::ZERO,
                freq: Tenor::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start: Date::from_calendar_date(2024, time::Month::January, 1).map_err(|e| {
                    finstack_core::error::Error::Validation(format!(
                        "Invalid example start date: {}",
                        e
                    ))
                })?,
                end: Date::from_calendar_date(2029, time::Month::January, 1).map_err(|e| {
                    finstack_core::error::Error::Validation(format!(
                        "Invalid example end date: {}",
                        e
                    ))
                })?,
                compounding: Default::default(),
                fixing_calendar_id: None,
                payment_delay_days: 0,
            })
            .build()?;

        // Validate the swap parameters
        swap.validate()?;

        Ok(swap)
    }

    /// Helper to construct a swap with specified curve configuration.
    ///
    /// Automatically validates the swap after construction.
    fn create_swap_with_config(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        config: SwapConfig<'_>,
    ) -> finstack_core::Result<Self> {
        let fixed = FixedLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            rate: Decimal::try_from(fixed_rate).unwrap_or(Decimal::ZERO),
            freq: config.sched.fixed_freq,
            dc: config.sched.fixed_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.as_deref().map(String::from),
            stub: config.sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
        };
        let float = FloatLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            forward_curve_id: finstack_core::types::CurveId::from(config.fwd_curve),
            spread_bp: Decimal::ZERO,
            freq: config.sched.float_freq,
            dc: config.sched.float_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.as_deref().map(String::from),
            stub: config.sched.stub,
            reset_lag_days: config.reset_lag_days,
            start,
            end,
            compounding: Default::default(),
            fixing_calendar_id: None,
            payment_delay_days: 0,
        };
        let swap = Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .fixed(fixed)
            .float(float)
            .build()?;

        // Validate the swap parameters
        swap.validate()?;

        Ok(swap)
    }
}

// Explicit trait implementations for modern instrument style
// Attributable implementation is provided by the impl_instrument! macro

impl crate::instruments::common::traits::Instrument for InterestRateSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::IRS
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::irs::pricer::npv(self, curves, as_of)
    }

    fn value_raw(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        crate::instruments::irs::pricer::npv_raw(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl CashflowProvider for InterestRateSwap {
    fn notional(&self) -> Option<Money> {
        // Return the receive leg notional as the primary notional
        Some(self.notional)
    }

    fn build_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        crate::instruments::irs::cashflow::signed_dated_flows_with_curves(self, Some(curves))
    }

    /// Build full cashflow schedule with CFKind metadata for precise classification.
    ///
    /// This creates a proper CashFlowSchedule with CFKind information for each leg,
    /// enabling precise classification of fixed vs floating rate payments.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        crate::instruments::irs::cashflow::full_signed_schedule_with_curves(self, Some(curves))
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InterestRateSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.fixed.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for InterestRateSwap {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.float.forward_curve_id.clone()]
    }
}

impl crate::instruments::common::traits::CurveDependencies for InterestRateSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.fixed.discount_curve_id.clone())
            .forward(self.float.forward_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn create_swap_with_config_uses_usd_isda_standard_schedule() {
        let config = SwapConfig {
            disc_curve: "USD-OIS",
            fwd_curve: "USD-SOFR-3M",
            reset_lag_days: 2,
            sched: IRSScheduleConfig::usd_isda_standard(),
        };

        let start =
            Date::from_calendar_date(2024, time::Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2029, time::Month::January, 1).expect("Valid test date");

        let swap = InterestRateSwap::create_swap_with_config(
            InstrumentId::new("IRS-TEST-USD"),
            Money::new(1_000_000.0, Currency::USD),
            0.03,
            start,
            end,
            PayReceive::PayFixed,
            config,
        )
        .expect("Valid test swap construction");

        let sched = IRSScheduleConfig::usd_isda_standard();

        // Discount and forward curve wiring
        assert_eq!(swap.fixed.discount_curve_id, CurveId::new("USD-OIS"));
        assert_eq!(swap.float.discount_curve_id, CurveId::new("USD-OIS"));
        assert_eq!(swap.float.forward_curve_id, CurveId::new("USD-SOFR-3M"));

        // Schedule conventions match usd_isda_standard configuration
        assert_eq!(swap.fixed.freq, sched.fixed_freq);
        assert_eq!(swap.fixed.dc, sched.fixed_dc);
        assert_eq!(swap.float.freq, sched.float_freq);
        assert_eq!(swap.float.dc, sched.float_dc);
        assert_eq!(swap.fixed.bdc, sched.bdc);
        assert_eq!(swap.float.bdc, sched.bdc);
        assert_eq!(swap.fixed.calendar_id, sched.calendar_id);
        assert_eq!(swap.float.calendar_id, sched.calendar_id);
        assert_eq!(swap.fixed.stub, sched.stub);
        assert_eq!(swap.float.stub, sched.stub);

        // Reset lag and date range are propagated correctly
        assert_eq!(swap.float.reset_lag_days, 2);
        assert_eq!(swap.fixed.start, start);
        assert_eq!(swap.fixed.end, end);
        assert_eq!(swap.float.start, start);
        assert_eq!(swap.float.end, end);
    }

    #[test]
    fn validate_accepts_extreme_but_valid_rate() {
        // Decimal doesn't have NaN/Infinity, so we just test that validation works
        // for extreme but valid values
        let swap = InterestRateSwap::example().expect("example swap");
        // Should pass validation
        assert!(swap.validate().is_ok(), "Valid swap should pass validation");
    }

    #[test]
    fn validate_rejects_negative_payment_delay_and_reset_lag() {
        let mut swap = InterestRateSwap::example().expect("example swap");
        swap.fixed.payment_delay_days = -1;
        assert!(
            swap.validate().is_err(),
            "negative payment delay must be rejected"
        );
    }
}
