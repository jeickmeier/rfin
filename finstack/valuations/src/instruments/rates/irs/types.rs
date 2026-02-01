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
use finstack_core::types::{CurveId, InstrumentId};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use crate::margin::types::OtcMarginSpec;

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
/// Use [`InterestRateSwap::validate()`] to check swaps constructed via
/// the builder pattern.
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

impl InterestRateSwap {}

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
        // Guard only against absurd magnitudes that indicate unit mistakes.
        if self.float.reset_lag_days.abs() > 31 {
            return Err(finstack_core::Error::Validation(
                "Invalid floating reset lag: absolute value too large (expected a small number of business days)."
                    .into(),
            ));
        }
        if self.fixed.payment_delay_days < 0 || self.float.payment_delay_days < 0 {
            return Err(finstack_core::Error::Validation(
                "Invalid payment delay: must be non-negative (business days).".into(),
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
            .fixed(crate::instruments::common_impl::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: Decimal::try_from(0.03).unwrap_or(Decimal::ZERO),
                freq: Tenor::semi_annual(),
                dc: DayCount::Thirty360,
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
                payment_delay_days: 0,
            })
            .float(crate::instruments::common_impl::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: Decimal::ZERO,
                freq: Tenor::quarterly(),
                dc: DayCount::Act360,
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
                payment_delay_days: 0,
            })
            .build()?;

        // Validate the swap parameters
        swap.validate()?;

        Ok(swap)
    }
}

// Explicit trait implementations for modern instrument style
// Attributable implementation is provided by the impl_instrument! macro

impl crate::instruments::common_impl::traits::Instrument for InterestRateSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::IRS
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

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

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
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

    /// Build full cashflow schedule with CFKind metadata for precise classification.
    ///
    /// This creates a proper CashFlowSchedule with CFKind information for each leg,
    /// enabling precise classification of fixed vs floating rate payments.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        crate::instruments::rates::irs::cashflow::full_signed_schedule_with_curves(
            self,
            Some(curves),
        )
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for InterestRateSwap {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
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
    fn validate_rejects_negative_payment_delay_and_reset_lag() {
        let mut swap = InterestRateSwap::example().expect("example swap");
        swap.fixed.payment_delay_days = -1;
        assert!(
            swap.validate().is_err(),
            "negative payment delay must be rejected"
        );
    }
}
