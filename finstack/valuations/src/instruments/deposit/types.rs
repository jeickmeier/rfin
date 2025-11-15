//! Deposit instrument types and trait implementations.
//!
//! Defines the `Deposit` instrument with explicit trait implementations
//! mirroring the modern instrument style used elsewhere in valuations.
//! Pricing logic is implemented as instance methods on the instrument struct.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;

/// Simple deposit instrument with optional quoted rate.
///
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
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
    #[builder(optional)]
    pub quote_rate: Option<f64>,
    /// Discount curve id used for valuation and par extraction.
    pub discount_curve_id: CurveId,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl Deposit {
    /// Create a canonical example deposit for testing and documentation.
    ///
    /// Returns a 6-month USD deposit with 4.5% quoted rate.
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("DEP-USD-6M"))
            .notional(Money::new(100_000.0, Currency::USD))
            .start(Date::from_calendar_date(2024, time::Month::January, 1).expect("Valid example date"))
            .end(Date::from_calendar_date(2024, time::Month::July, 1).expect("Valid example date"))
            .day_count(DayCount::Act360)
            .quote_rate_opt(Some(0.045))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(Attributes::new())
            .build()
            .expect("Example deposit construction should not fail")
    }

    /// Calculate the net present value of this deposit using standard cashflow discounting.
    ///
    /// Builds the cashflow schedule (principal out at start, principal + interest at end)
    /// and discounts to the as_of date using the assigned discount curve.
    pub fn npv(
        &self,
        context: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        crate::instruments::common::helpers::schedule_pv_impl(
            self,
            context,
            as_of,
            &self.discount_curve_id,
            self.day_count,
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Call the instrument's own NPV method
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
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
        )
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

impl CashflowProvider for Deposit {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // True single-period deposit: two flows with simple interest
        let yf = self
            .day_count
            .year_fraction(
                self.start,
                self.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let r = self.quote_rate.unwrap_or(0.0);
        let redemption = self.notional * (1.0 + r * yf);
        Ok(vec![
            (self.start, self.notional * -1.0),
            (self.end, redemption),
        ])
    }
}
