//! Deposit instrument implementation.

pub mod metrics;

use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::prelude::*;
use finstack_core::F;

// (no longer using cashflow builder for deposits)
use crate::metrics::MetricId;
use crate::traits::{Attributes, CashflowProvider, DatedFlows, Priceable};
use crate::{impl_attributable, impl_builder};
// (no scheduling knobs needed in the two-flow model)

/// Simple deposit instrument with optional quoted rate.
///
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
#[derive(Clone, Debug)]
pub struct Deposit {
    /// Unique identifier for the deposit.
    pub id: String,
    /// Principal amount of the deposit.
    pub notional: Money,
    /// Start date of the deposit period.
    pub start: Date,
    /// End date of the deposit period.
    pub end: Date,
    /// Day count convention for interest accrual.
    pub day_count: DayCount,
    /// Optional quoted simple rate r (annualised) for the deposit.
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: &'static str,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

// Generate Attributable implementation using macro
impl_attributable!(Deposit);

// Custom Priceable implementation for Deposit (uses day_count field)
impl Priceable for Deposit {
    fn value(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::pricing::discountable::Discountable;
        let flows = self.build_schedule(curves, as_of)?;
        let disc = curves.discount(self.disc_id)?;
        flows.npv(&*disc, disc.base_date(), self.day_count)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::pricing::result::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        let instrument: crate::instruments::Instrument =
            crate::instruments::Instrument::Deposit(self.clone());
        crate::pricing::build_with_metrics(instrument, curves, as_of, base_value, metrics)
    }

    fn price(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::pricing::result::ValuationResult> {
        let standard_metrics = vec![
            MetricId::Yf,
            MetricId::DfStart,
            MetricId::DfEnd,
            MetricId::DepositParRate,
        ];
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

// Generate builder pattern for Deposit
impl_builder!(
    Deposit,
    DepositBuilder,
    required: [
        id: String,
        notional: Money,
        start: Date,
        end: Date,
        day_count: DayCount,
        disc_id: &'static str
    ],
    optional: [
        quote_rate: F
    ]
);

impl From<Deposit> for crate::instruments::Instrument {
    fn from(value: Deposit) -> Self {
        crate::instruments::Instrument::Deposit(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for Deposit {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::Deposit(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

impl CashflowProvider for Deposit {
    fn build_schedule(
        &self,
        _curves: &CurveSet,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // True single-period deposit: two flows with simple interest
        let yf = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
            self.start,
            self.end,
            self.day_count,
        );
        let r = self.quote_rate.unwrap_or(0.0);
        let redemption = self.notional * (1.0 + r * yf);
        Ok(vec![(self.start, self.notional * -1.0), (self.end, redemption)])
    }
}
