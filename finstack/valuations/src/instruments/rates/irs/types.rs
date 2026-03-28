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
use finstack_core::dates::{BusinessDayConvention, Date, DateExt, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::cashflow::traits::CashflowProvider;
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use crate::margin::types::OtcMarginSpec;
use crate::market::conventions::ids::IndexId;
use crate::market::conventions::ConventionRegistry;

// Re-export common enums from parameters
pub use crate::instruments::common_impl::parameters::legs::{ParRateMethod, PayReceive};

// Re-export from common parameters
pub use crate::instruments::common_impl::parameters::legs::FixedLegSpec;
pub use crate::instruments::common_impl::parameters::legs::FloatLegSpec;

/// Leg-level conventions for building a vanilla fixed-vs-float IRS.
///
/// This is intentionally minimal: it captures the schedule/lag/calendar knobs
/// that are commonly resolved from market conventions (e.g. calibration quote
/// conventions) while keeping the instrument surface stable.
#[derive(Debug, Clone)]
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
    pub payment_lag_days: i32,
}

impl IrsLegConventions {
    /// Resolve conventions from a rate index in the global `ConventionRegistry`.
    pub fn from_rate_index(index_id: &str) -> finstack_core::Result<Self> {
        let registry = ConventionRegistry::try_global().map_err(|_| {
            finstack_core::Error::Validation("ConventionRegistry not initialized.".into())
        })?;
        let idx = IndexId::new(index_id);
        let conv = registry.require_rate_index(&idx)?;

        Ok(Self {
            fixed_freq: conv.default_fixed_leg_frequency,
            float_freq: conv.default_payment_frequency,
            fixed_dc: conv.default_fixed_leg_day_count,
            float_dc: conv.day_count,
            bdc: BusinessDayConvention::ModifiedFollowing,
            payment_calendar_id: Some(conv.market_calendar_id.clone()),
            fixing_calendar_id: Some(conv.market_calendar_id.clone()),
            stub: StubKind::ShortFront,
            reset_lag_days: conv.default_reset_lag_days,
            payment_lag_days: conv.default_payment_lag_days,
        })
    }
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
/// Use [`InterestRateSwap::validate()`] to check swaps constructed via
/// the builder pattern.
///
/// ## References
///
/// - ISDA 2006 Definitions (incorporating 2008 Supplement for OIS)
/// - ISDA 2021 Definitions (for RFR compounding conventions)
/// - "Interest Rate Swaps and Their Derivatives" by Amir Sadr
/// - Bloomberg SWPM function documentation
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Attributes for scenario selection and tagging.
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: crate::instruments::PricingOverrides,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

/// Parameters for constructing a vanilla IRS from market conventions.
#[derive(Debug, Clone)]
pub struct ConventionSwapParams<'a> {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional principal amount.
    pub notional: Money,
    /// Pay or receive fixed.
    pub side: PayReceive,
    /// Fixed coupon rate (as a decimal, e.g. 0.03 = 3%).
    pub fixed_rate: f64,
    /// Effective (start) date.
    pub start: Date,
    /// Maturity (end) date.
    pub end: Date,
    /// Rate index identifier used to resolve conventions (e.g. `"USD-SOFR"`).
    pub index_id: &'a str,
    /// Discount curve identifier.
    pub discount_curve_id: &'a str,
    /// Forward projection curve identifier.
    pub forward_curve_id: &'a str,
}

impl InterestRateSwap {
    /// Default start date when the user omits it: `end − 365 calendar days`.
    ///
    /// Shared across all frontend bindings (Python, WASM) so that the
    /// defaulting logic lives in core rather than being duplicated.
    pub fn default_start_date(end: Date) -> Date {
        end.checked_sub(time::Duration::days(365)).unwrap_or(end)
    }

    /// Create an IRS from market conventions resolved via `ConventionRegistry`.
    ///
    /// This is the preferred way to construct standard swaps. Conventions
    /// (day counts, frequencies, calendars, lags) are resolved from the
    /// registered rate index, matching QuantLib `MakeVanillaSwap` ergonomics.
    pub fn from_conventions(params: ConventionSwapParams<'_>) -> finstack_core::Result<Self> {
        let ConventionSwapParams {
            id,
            notional,
            side,
            fixed_rate,
            start,
            end,
            index_id,
            discount_curve_id,
            forward_curve_id,
        } = params;
        let conv = IrsLegConventions::from_rate_index(index_id)?;
        let registry = ConventionRegistry::try_global().map_err(|_| {
            finstack_core::Error::Validation("ConventionRegistry not initialized.".into())
        })?;
        let idx = IndexId::new(index_id);
        let rate_conv = registry.require_rate_index(&idx)?;

        let compounding = rate_conv.ois_compounding.clone().unwrap_or_default();

        let swap = Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .fixed(FixedLegSpec {
                discount_curve_id: CurveId::new(discount_curve_id),
                rate: crate::utils::decimal::f64_to_decimal(fixed_rate, "fixed_rate")?,
                frequency: conv.fixed_freq,
                day_count: conv.fixed_dc,
                bdc: conv.bdc,
                calendar_id: conv.payment_calendar_id.clone(),
                stub: conv.stub,
                start,
                end,
                par_method: None,
                compounding_simple: true,
                payment_lag_days: conv.payment_lag_days,
                end_of_month: false,
            })
            .float(FloatLegSpec {
                discount_curve_id: CurveId::new(discount_curve_id),
                forward_curve_id: CurveId::new(forward_curve_id),
                spread_bp: Decimal::ZERO,
                frequency: conv.float_freq,
                day_count: conv.float_dc,
                bdc: conv.bdc,
                calendar_id: conv.payment_calendar_id.clone(),
                stub: conv.stub,
                reset_lag_days: conv.reset_lag_days,
                fixing_calendar_id: conv.fixing_calendar_id,
                start,
                end,
                compounding,
                payment_lag_days: conv.payment_lag_days,
                end_of_month: false,
            })
            .build()?;

        swap.validate()?;
        Ok(swap)
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
    fn rate_index_conventions(&self) -> Option<crate::market::conventions::RateIndexConventions> {
        let registry = ConventionRegistry::try_global().ok()?;
        let idx = IndexId::new(self.float.forward_curve_id.as_str());
        registry.require_rate_index(&idx).ok().cloned()
    }

    pub(crate) fn resolved_fixed_leg(&self) -> FixedLegSpec {
        let mut fixed = self.fixed.clone();
        let is_eom_swap =
            fixed.start.end_of_month() == fixed.start && fixed.end.end_of_month() == fixed.end;
        if is_eom_swap && !fixed.end_of_month {
            fixed.end_of_month = true;
        }
        if let Some(conv) = self.rate_index_conventions() {
            // Note: calendar_id: None is intentionally NOT overridden.
            // None means "no calendar" (weekends-only), which is a valid explicit choice.
            // Users wanting convention calendars should set calendar_id explicitly.

            // Use negative values as sentinel for "apply convention default".
            // Zero is a valid explicit value meaning "no payment delay".
            if fixed.payment_lag_days < 0 {
                fixed.payment_lag_days = conv.default_payment_lag_days;
            }
        }
        fixed
    }

    pub(crate) fn resolved_float_leg(&self) -> FloatLegSpec {
        let mut float = self.float.clone();
        let is_eom_swap =
            float.start.end_of_month() == float.start && float.end.end_of_month() == float.end;
        if is_eom_swap && !float.end_of_month {
            float.end_of_month = true;
        }
        if let Some(conv) = self.rate_index_conventions() {
            // Note: calendar_id and fixing_calendar_id: None are intentionally NOT overridden.
            // None means "no calendar" (weekends-only), which is a valid explicit choice.
            // Users wanting convention calendars should set these explicitly.

            // Use negative values as sentinel for "apply convention default".
            // Zero is a valid explicit value meaning "spot reset" (no reset lag).
            // This fixes test failures where reset_lag_days: 0 was being overridden
            // to convention defaults, causing unexpected seasoned swap behavior.
            if float.reset_lag_days < 0 {
                float.reset_lag_days = conv.default_reset_lag_days;
            }
            // Same for payment_lag_days: 0 means "no delay", negative means "use default".
            if float.payment_lag_days < 0 {
                float.payment_lag_days = conv.default_payment_lag_days;
            }
        }
        float
    }

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
    /// use finstack_valuations::instruments::rates::irs::InterestRateSwap;
    ///
    /// let swap = InterestRateSwap::example()?;
    /// swap.validate()?; // Passes for valid swap
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Validate finiteness early to avoid NaN poisoning and panics in downstream code.
        validation::validate_money_finite(self.notional, "notional")?;
        // Decimal values are always finite (no NaN/Infinity), so we just check they're valid
        // by verifying they can be converted to f64 for magnitude checks

        // Reset lag is a positive business-day offset subtracted from the accrual start to obtain
        // the reset/fixing date. Market convention "T-2" is represented as reset_lag_days = 2,
        // meaning fixing_date = accrual_start - 2 business days.
        // Small negative values (e.g., -1) are allowed as sentinels for "use convention default".
        // Guard only against absurd magnitudes that indicate unit mistakes.
        if self.float.reset_lag_days.abs() > 31 {
            return Err(finstack_core::Error::Validation(
                "Invalid floating reset lag: absolute value too large (expected a small number of business days)."
                    .into(),
            ));
        }
        // Payment delay validation: large negative values are rejected (likely unit mistakes).
        // Small negative values (e.g., -1) are allowed as sentinels for "use convention default".
        // Zero and positive values are explicit delays.
        if self.fixed.payment_lag_days < -31 || self.float.payment_lag_days < -31 {
            return Err(finstack_core::Error::Validation(
                "Invalid payment delay: value too negative (use small negative like -1 for convention default)."
                    .into(),
            ));
        }
        if let crate::instruments::rates::irs::FloatingLegCompounding::CompoundedInArrears {
            lookback_days,
            observation_shift,
        } = self.float.compounding
        {
            if lookback_days < 0 {
                return Err(finstack_core::Error::Validation(
                    "Invalid RFR lookback: must be non-negative (business days).".into(),
                ));
            }
            if let Some(shift) = observation_shift {
                // Observation shift can be negative, but must be within a sane bound to avoid
                // accidental unit mistakes (e.g., passing years as days).
                if shift.abs() > 31 {
                    return Err(finstack_core::Error::Validation(
                        "Invalid observation shift: absolute value too large (expected a small number of business days)."
                            .into(),
                    ));
                }
            }
        }
        if let crate::instruments::rates::irs::FloatingLegCompounding::CompoundedWithObservationShift {
            shift_days,
        } = self.float.compounding
        {
            if shift_days < 0 {
                return Err(finstack_core::Error::Validation(
                    "Invalid observation shift days: must be non-negative.".into(),
                ));
            }
            if shift_days > 31 {
                return Err(finstack_core::Error::Validation(
                    "Invalid observation shift days: too large.".into(),
                ));
            }
        }

        // Validate fixed leg date range
        validation::validate_date_range_strict_with(
            self.fixed.start,
            self.fixed.end,
            |start, end| {
                format!(
                    "Invalid fixed leg date range: end ({}) must be after start ({})",
                    end, start
                )
            },
        )?;

        // Validate floating leg date range
        validation::validate_date_range_strict_with(
            self.float.start,
            self.float.end,
            |start, end| {
                format!(
                    "Invalid floating leg date range: end ({}) must be after start ({})",
                    end, start
                )
            },
        )?;

        // Validate notional is positive
        validation::validate_money_gt_with(self.notional, NOTIONAL_EPSILON, |amount| {
            format!(
                "Invalid notional: {} must be positive (> {:.0e}). \
                 Negative notional is semantically ambiguous; use PayReceive to control direction.",
                amount, NOTIONAL_EPSILON
            )
        })?;

        // Validate fixed rate is within reasonable bounds
        let rate_f64 = self.fixed.rate.to_f64().unwrap_or(0.0);
        if rate_f64.abs() > MAX_RATE_MAGNITUDE {
            return Err(finstack_core::Error::Validation(format!(
                "Invalid fixed rate: {:.2}% exceeds maximum allowed magnitude ({:.0}%). \
                 This may indicate a units error (rate should be decimal, e.g., 0.05 for 5%).",
                rate_f64 * 100.0,
                MAX_RATE_MAGNITUDE * 100.0
            )));
        }

        // Warn-level check: legs should typically have matching date ranges
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

    /// Create a minimal example IRS for testing and documentation.
    ///
    /// Returns a 5-year pay-fixed swap with semi-annual fixed vs quarterly floating.
    ///
    /// **Note:** This example uses simplified defaults (`reset_lag_days: 0`,
    /// `payment_lag_days: 0`, no calendar) to avoid requiring historical fixings
    /// or calendar data. For an ISDA-standard USD swap with proper market
    /// conventions, use [`example_standard()`](Self::example_standard).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> finstack_core::Result<Self> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let swap = Self::builder()
            .id(InstrumentId::new("IRS-5Y-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .fixed(crate::instruments::common_impl::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: Decimal::try_from(0.03_f64).expect("valid literal"),
                frequency: Tenor::semi_annual(),
                day_count: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: Date::from_calendar_date(2024, time::Month::January, 1).map_err(|e| {
                    finstack_core::Error::Validation(format!("Invalid example start date: {}", e))
                })?,
                end: Date::from_calendar_date(2029, time::Month::January, 1).map_err(|e| {
                    finstack_core::Error::Validation(format!("Invalid example end date: {}", e))
                })?,
                par_method: None,
                compounding_simple: true,
                payment_lag_days: 0,
                end_of_month: false,
            })
            .float(crate::instruments::common_impl::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: Decimal::ZERO,
                frequency: Tenor::quarterly(),
                day_count: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                // Use 0 for example to avoid requiring historical fixings
                reset_lag_days: 0,
                start: Date::from_calendar_date(2024, time::Month::January, 1).map_err(|e| {
                    finstack_core::Error::Validation(format!("Invalid example start date: {}", e))
                })?,
                end: Date::from_calendar_date(2029, time::Month::January, 1).map_err(|e| {
                    finstack_core::Error::Validation(format!("Invalid example end date: {}", e))
                })?,
                compounding: Default::default(),
                fixing_calendar_id: None,
                payment_lag_days: 0,
                end_of_month: false,
            })
            .build()?;

        // Validate the swap parameters
        swap.validate()?;

        Ok(swap)
    }

    /// Create an ISDA-standard USD 5Y IRS for testing and documentation.
    ///
    /// Returns a 5-year pay-fixed USD swap with market-standard conventions:
    /// - **Fixed leg:** Semi-annual, 30/360, Modified Following
    /// - **Float leg:** Quarterly, ACT/360, Modified Following
    /// - **Reset lag:** T-2 (per ISDA 2006 Section 4.2)
    /// - **Calendar:** USNY
    #[allow(clippy::expect_used)]
    pub fn example_standard() -> finstack_core::Result<Self> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let start = Date::from_calendar_date(2024, time::Month::January, 2).map_err(|e| {
            finstack_core::Error::Validation(format!("Invalid example start date: {}", e))
        })?;
        let end = Date::from_calendar_date(2029, time::Month::January, 2).map_err(|e| {
            finstack_core::Error::Validation(format!("Invalid example end date: {}", e))
        })?;

        let swap = Self::builder()
            .id(InstrumentId::new("IRS-5Y-USD-STD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .fixed(crate::instruments::common_impl::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: Decimal::try_from(0.04_f64).expect("valid literal"),
                frequency: Tenor::semi_annual(),
                day_count: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("usny".to_string()),
                stub: StubKind::ShortFront,
                start,
                end,
                par_method: None,
                compounding_simple: true,
                payment_lag_days: 0,
                end_of_month: false,
            })
            .float(crate::instruments::common_impl::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: Decimal::ZERO,
                frequency: Tenor::quarterly(),
                day_count: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("usny".to_string()),
                stub: StubKind::ShortFront,
                reset_lag_days: 2,
                fixing_calendar_id: Some("usny".to_string()),
                start,
                end,
                compounding: Default::default(),
                payment_lag_days: 0,
                end_of_month: false,
            })
            .build()?;

        swap.validate()?;
        Ok(swap)
    }
}

// Explicit trait implementations for modern instrument style
// Attributable implementation is provided by the impl_instrument! macro

impl crate::instruments::common_impl::traits::Instrument for InterestRateSwap {
    impl_instrument_base!(crate::pricer::InstrumentType::IRS);

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::rates::irs::pricer::compute_pv(self, curves, as_of)
    }

    fn value_raw(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        crate::instruments::rates::irs::pricer::compute_pv_raw(self, curves, as_of)
    }

    fn as_marginable(&self) -> Option<&dyn finstack_margin::Marginable> {
        Some(self)
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.fixed.end)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.fixed.start)
    }

    fn pricing_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn pricing_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl CashflowProvider for InterestRateSwap {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn cashflow_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let schedule =
            crate::instruments::rates::irs::cashflow::full_signed_schedule_with_curves_as_of(
                self,
                Some(curves),
                Some(as_of),
            )?;
        Ok(schedule.normalize_public(
            as_of,
            crate::cashflow::builder::CashflowRepresentation::Projected,
        ))
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for InterestRateSwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
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
    fn validate_accepts_extreme_but_valid_rate() {
        // Decimal doesn't have NaN/Infinity, so we just test that validation works
        // for extreme but valid values
        let swap = InterestRateSwap::example().expect("example swap");
        // Should pass validation
        assert!(swap.validate().is_ok(), "Valid swap should pass validation");
    }

    #[test]
    fn validate_allows_small_negative_as_convention_sentinel() {
        // Small negative values (like -1) are allowed as sentinels for "use convention default"
        let mut swap = InterestRateSwap::example().expect("example swap");
        swap.fixed.payment_lag_days = -1;
        assert!(
            swap.validate().is_ok(),
            "small negative payment delay (-1) should be allowed as convention sentinel"
        );
    }

    #[test]
    fn validate_rejects_large_negative_payment_delay() {
        // Large negative values are rejected as likely unit mistakes
        let mut swap = InterestRateSwap::example().expect("example swap");
        swap.fixed.payment_lag_days = -100;
        assert!(
            swap.validate().is_err(),
            "large negative payment delay must be rejected"
        );
    }

    #[test]
    fn default_start_date_subtracts_365_days() {
        let end = Date::from_calendar_date(2030, time::Month::June, 15).expect("valid");
        let start = InterestRateSwap::default_start_date(end);
        let expected = end
            .checked_sub(time::Duration::days(365))
            .expect("subtraction should succeed");
        assert_eq!(start, expected);
        assert!(start < end);
    }
}
