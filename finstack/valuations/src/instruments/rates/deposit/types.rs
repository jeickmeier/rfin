//! Deposit instrument types and trait implementations.
//!
//! Defines the `Deposit` instrument with explicit trait implementations
//! mirroring the modern instrument style used elsewhere in valuations.
//! Pricing logic is implemented as instance methods on the instrument struct.
//!
//! # Market Conventions
//!
//! Money-market deposits settle on business days with currency-specific spot lags:
//! - **USD/EUR/JPY**: T+2 settlement (two business days after trade date)
//! - **GBP**: T+0 settlement (same day)
//!
//! The instrument supports optional spot lag and business day convention fields
//! to properly compute settlement dates when building cashflow schedules.

use finstack_core::currency::Currency;
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::{
    adjust, BusinessDayConvention, Date, DateExt, DayCount, HolidayCalendar,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;

/// Simple deposit instrument with optional quoted rate.
///
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
///
/// # Market Convention Fields
///
/// The instrument supports optional settlement convention fields for proper
/// business-day adjusted cashflow generation:
///
/// - `spot_lag_days`: Number of business days from trade date to spot date (default: 2 for USD/EUR/JPY, 0 for GBP)
/// - `bdc`: Business day convention for date adjustment (default: ModifiedFollowing)
/// - `calendar_id`: Holiday calendar identifier for business day logic (e.g., "nyse", "target")
///
/// When these fields are set and `as_of` is provided to `build_schedule`, the effective
/// start date is computed as `as_of + spot_lag` adjusted by the business day convention.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Deposit {
    /// Unique identifier for the deposit.
    pub id: InstrumentId,
    /// Principal amount of the deposit.
    pub notional: Money,
    /// Start date of the deposit period.
    pub start: Date,
    /// End date of the deposit period.
    pub end: Date,
    /// Day count convention for interest accrual.
    pub day_count: DayCount,

    /// Optional quoted simple rate r (annualised) for the deposit.
    ///
    /// Note: `build_schedule()` requires `quote_rate` to be set. Leaving it as `None`
    /// is only appropriate if the caller never requests cashflow generation/PV from
    /// this instrument (e.g., constructing placeholders).
    #[builder(optional)]
    pub quote_rate: Option<f64>,
    /// Discount curve id used for valuation and par extraction.
    pub discount_curve_id: CurveId,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,

    /// Optional spot lag in business days from trade date to effective start.
    ///
    /// Market convention: T+2 for USD/EUR/JPY, T+0 for GBP.
    /// If not set, the raw `start` date is used without adjustment.
    #[builder(optional)]
    pub spot_lag_days: Option<i32>,

    /// Business day convention for date adjustments.
    ///
    /// Used to adjust the effective start/end dates to valid business days.
    /// Default: `ModifiedFollowing` (standard money market convention).
    #[builder(optional)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub bdc: Option<BusinessDayConvention>,

    /// Optional holiday calendar identifier for business day logic.
    ///
    /// Examples: "nyse", "target", "london", "tokyo".
    /// When set, enables calendar-aware spot date and accrual date adjustments.
    #[builder(optional)]
    pub calendar_id: Option<String>,
}

impl Deposit {
    /// Create a canonical example deposit for testing and documentation.
    ///
    /// Returns a 6-month USD deposit with 4.5% quoted rate and standard
    /// T+2 spot settlement with ModifiedFollowing business day convention.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("DEP-USD-6M"))
            .notional(Money::new(100_000.0, Currency::USD))
            .start(
                Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
            )
            .end(Date::from_calendar_date(2024, time::Month::July, 1).expect("Valid example date"))
            .day_count(DayCount::Act360)
            .quote_rate_opt(Some(0.045))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .spot_lag_days_opt(Some(2))
            .bdc_opt(Some(BusinessDayConvention::ModifiedFollowing))
            .build()
            .expect("Example deposit construction should not fail")
    }

    /// Calculate the net present value of this deposit using standard cashflow discounting.
    ///
    /// Builds the cashflow schedule (principal out at start, principal + interest at end)
    /// and discounts to the as_of date using the assigned discount curve.
    ///
    /// **Note**: Uses the discount curve's day count for discounting (not the instrument's
    /// accrual day count) to ensure consistency with par rate calculations. This means
    /// a deposit priced at its par rate will have zero PV.
    pub fn npv(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        crate::instruments::common::helpers::schedule_pv_using_curve_dc(
            self,
            context,
            as_of,
            &self.discount_curve_id,
        )
    }

    /// Calculate the raw (unrounded) net present value of this deposit.
    pub fn npv_raw(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        crate::instruments::common::helpers::schedule_pv_using_curve_dc_raw(
            self,
            context,
            as_of,
            &self.discount_curve_id,
        )
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for Deposit {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Deposit
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
        // Call the instrument's own NPV method
        self.npv(curves, as_of)
    }

    fn value_raw(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.npv_raw(curves, as_of)
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

impl crate::instruments::common::pricing::HasDiscountCurve for Deposit {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for Deposit {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl Deposit {
    /// Validate the deposit parameters.
    ///
    /// Checks that:
    /// - End date is after start date
    /// - Notional is positive
    ///
    /// This is called automatically during cashflow generation and pricing.
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Validate date ordering
        if self.end <= self.start {
            return Err(finstack_core::Error::Validation(format!(
                "Deposit end date ({}) must be after start date ({})",
                self.end, self.start
            )));
        }

        // Validate positive notional
        if self.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Deposit notional must be positive, got {}",
                self.notional.amount()
            )));
        }

        Ok(())
    }

    /// Compute the effective start date considering spot lag and business day adjustments.
    ///
    /// If `spot_lag_days` is set, computes the start date as `as_of + spot_lag` business days
    /// (or calendar days if no calendar is set), then applies the business day convention.
    ///
    /// If `spot_lag_days` is not set, returns the raw `start` date optionally adjusted by BDC.
    ///
    /// # Arguments
    /// * `as_of` - The valuation/trade date
    ///
    /// # Returns
    /// The effective start date after all adjustments.
    pub fn effective_start_date(&self, as_of: Date) -> finstack_core::Result<Date> {
        let calendar: Option<&dyn HolidayCalendar> = self
            .calendar_id
            .as_deref()
            .and_then(|id| CalendarRegistry::global().resolve_str(id));

        let bdc = self.bdc.unwrap_or(BusinessDayConvention::ModifiedFollowing);

        let base_start = if let Some(lag_days) = self.spot_lag_days {
            // Compute spot date: as_of + spot_lag business days
            if let Some(cal) = calendar {
                as_of.add_business_days(lag_days, cal)?
            } else {
                as_of.add_weekdays(lag_days)
            }
        } else {
            // Use raw start date
            self.start
        };

        // Apply business day adjustment if calendar is available
        if let Some(cal) = calendar {
            adjust(base_start, bdc, cal)
        } else {
            Ok(base_start)
        }
    }

    /// Compute the effective end date considering business day adjustments.
    ///
    /// The end date is adjusted using the business day convention and calendar if set.
    ///
    /// # Returns
    /// The effective end date after all adjustments.
    pub fn effective_end_date(&self) -> finstack_core::Result<Date> {
        let calendar: Option<&dyn HolidayCalendar> = self
            .calendar_id
            .as_deref()
            .and_then(|id| CalendarRegistry::global().resolve_str(id));

        let bdc = self.bdc.unwrap_or(BusinessDayConvention::ModifiedFollowing);

        // Apply business day adjustment if calendar is available
        if let Some(cal) = calendar {
            adjust(self.end, bdc, cal)
        } else {
            Ok(self.end)
        }
    }
}

impl CashflowProvider for Deposit {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_schedule(
        &self,
        _curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Validate deposit parameters before building schedule
        self.validate()?;

        // Compute effective dates with spot lag and business day adjustments
        // When spot_lag_days is set, compute effective start from as_of
        // Otherwise, use the raw start/end dates (optionally BDC-adjusted)
        let effective_start = if self.spot_lag_days.is_some() {
            self.effective_start_date(as_of)?
        } else {
            // Even without spot lag, apply BDC if calendar is set
            self.effective_start_date(self.start)?
        };
        let effective_end = self.effective_end_date()?;

        // Validate effective date ordering
        if effective_end <= effective_start {
            return Err(finstack_core::Error::Validation(format!(
                "Deposit effective end date ({}) must be after effective start date ({})",
                effective_end, effective_start
            )));
        }

        // Validate that effective start is on or after as_of when spot_lag is used
        // This enforces proper settlement timing
        if self.spot_lag_days.is_some() && effective_start < as_of {
            return Err(finstack_core::Error::Validation(format!(
                "Deposit effective start date ({}) must be on or after as_of date ({}) when spot_lag is specified",
                effective_start, as_of
            )));
        }

        // True single-period deposit: two flows with simple interest
        // Use effective dates for proper accrual calculation
        let yf = self.day_count.year_fraction(
            effective_start,
            effective_end,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let r = self.quote_rate.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "deposit quote_rate".to_string(),
            })
        })?;
        let redemption = self.notional * (1.0 + r * yf);
        Ok(vec![
            (effective_start, self.notional * -1.0),
            (effective_end, redemption),
        ])
    }
}
