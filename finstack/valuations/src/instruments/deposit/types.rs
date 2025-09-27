//! Deposit instrument types and trait implementations.
//!
//! Defines the `Deposit` instrument with explicit trait implementations
//! mirroring the modern instrument style used elsewhere in valuations.
//! Pricing logic is implemented as instance methods on the instrument struct.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;

/// Simple deposit instrument with optional quoted rate.
///
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: CurveId,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl Deposit {
    /// Calculate the net present value of this deposit
    pub fn npv(
        &self,
        context: &finstack_core::market_data::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.disc_id.clone())?;

        // Accrual factor (instrument basis)
        let yf = self
            .day_count
            .year_fraction(self.start, self.end, finstack_core::dates::DayCountCtx::default())?
            .max(0.0);

        // Quoted simple rate (default to 0 when not provided)
        let r = self.quote_rate.unwrap_or(0.0);

        // Redemption amount at maturity
        let redemption = self.notional * (1.0 + r * yf);

        // Discount both legs using the curve's own time basis
        let df_start = disc.df_on_date_curve(self.start);
        let df_end = disc.df_on_date_curve(self.end);

        // PV = -Notional * DF(start) + Redemption * DF(end)
        let currency = self.notional.currency();
        let pv = -self.notional.amount() * df_start + redemption.amount() * df_end;
        Ok(Money::new(pv, currency))
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for Deposit {
    #[inline]
    fn id(&self) -> &str {
        self.id.as_str()
    }

    #[inline]
    fn instrument_type(&self) -> &'static str {
        "Deposit"
    }

    #[inline]
    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    #[inline]
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    #[inline]
    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    #[inline]
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
            self, curves, as_of, base_value, metrics,
        )
    }
}

impl crate::instruments::common::HasDiscountCurve for Deposit {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
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
