//! Fixed Income Index Total Return Swap instrument definitions and helpers.

use super::types::{FinancingLegSpec, IndexUnderlyingParams, TrsScheduleSpec, TrsSide};
use crate::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::common::traits::Attributes,
    margin::types::OtcMarginSpec,
};
use finstack_core::{
    dates::Date, market_data::context::MarketContext, money::Money, types::InstrumentId, Result,
};

/// Fixed Income Index Total Return Swap instrument.
///
/// A TRS where the total return leg is based on a fixed income index (e.g., corporate bond index).
/// The holder receives the total return (carry + roll) of the underlying index in exchange
/// for paying a floating rate plus spread on the notional amount.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FIIndexTotalReturnSwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional amount for the swap.
    pub notional: Money,
    /// Underlying index parameters (index ID, yield, duration, base currency).
    pub underlying: IndexUnderlyingParams,
    /// Financing leg specification (curves, spread, day count).
    pub financing: FinancingLegSpec,
    /// Schedule specification (payment dates and frequency).
    pub schedule: TrsScheduleSpec,
    /// Trade side (receive/pay total return).
    pub side: TrsSide,
    /// Initial index level (if known, otherwise fetched from market).
    pub initial_level: Option<f64>,
    /// Optional OTC margin specification for VM/IM.
    ///
    /// Fixed income index TRS use duration-based margin calculations.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl FIIndexTotalReturnSwap {
    /// Create a canonical example fixed income index TRS (USD Corporate Index, 1Y).
    pub fn example() -> Self {
        use crate::cashflow::builder::ScheduleParams;
        use crate::instruments::trs::types::{TrsScheduleSpec, TrsSide};
        use finstack_core::currency::Currency;
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        let underlying = IndexUnderlyingParams::new("US-CORP-INDEX", Currency::USD)
            .with_yield("US-CORP-YIELD")
            .with_duration("US-CORP-DURATION")
            .with_convexity("US-CORP-CONVEXITY")
            .with_contract_size(1.0);
        let financing = FinancingLegSpec {
            discount_curve_id: finstack_core::types::CurveId::new("USD-OIS"),
            forward_curve_id: finstack_core::types::CurveId::new("USD-SOFR-3M"),
            spread_bp: 100.0,
            day_count: DayCount::Act360,
        };
        let sched = TrsScheduleSpec::from_params(
            Date::from_calendar_date(2024, time::Month::January, 1).expect("Valid example date"),
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid example date"),
            ScheduleParams {
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            },
        );
        FIIndexTotalReturnSwapBuilder::new()
            .id(InstrumentId::new("TRS-US-CORP-1Y"))
            .notional(Money::new(5_000_000.0, Currency::USD))
            .underlying(underlying)
            .financing(financing)
            .schedule(sched)
            .side(TrsSide::ReceiveTotalReturn)
            .initial_level_opt(None)
            .attributes(Attributes::new())
            .build()
            .expect("Example FIIndexTotalReturnSwap construction should not fail")
    }
    /// Calculates the net present value (NPV) of the fixed income index TRS.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Net present value in the instrument's currency.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // Calculate total return leg PV
        let total_return_pv = self.pv_total_return_leg(curves, as_of)?;

        // Calculate financing leg PV
        let financing_pv = self.pv_financing_leg(curves, as_of)?;

        // Net PV depends on side
        let net_pv = match self.side {
            super::TrsSide::ReceiveTotalReturn => (total_return_pv - financing_pv)?,
            super::TrsSide::PayTotalReturn => (financing_pv - total_return_pv)?,
        };

        Ok(net_pv)
    }

    /// Calculates the present value of the total return leg.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the total return leg in the instrument's currency.
    pub fn pv_total_return_leg(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::trs::pricing::fixed_income_index;
        fixed_income_index::pv_total_return_leg(self, curves, as_of)
    }

    /// Calculates the present value of the financing leg.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the financing leg in the instrument's currency.
    pub fn pv_financing_leg(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        use crate::instruments::trs::pricing::engine::TrsEngine;
        TrsEngine::pv_financing_leg(
            &self.financing,
            &self.schedule,
            self.notional,
            curves,
            as_of,
        )
    }

    /// Calculates the financing annuity for par spread calculation.
    ///
    /// # Arguments
    /// * `curves` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Financing annuity (sum of discounted year fractions × notional).
    pub fn financing_annuity(&self, curves: &MarketContext, as_of: Date) -> Result<f64> {
        use crate::instruments::trs::pricing::engine::TrsEngine;
        TrsEngine::financing_annuity(
            &self.financing,
            &self.schedule,
            self.notional,
            curves,
            as_of,
        )
    }
}

// Attributable implementation is provided by the impl_instrument! macro

// Use the macro to implement Instrument with pricing
impl crate::instruments::common::traits::Instrument for FIIndexTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FIIndexTotalReturnSwap
    }

    fn as_any(&self) -> &dyn::std::any::Any {
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
        self.npv(curves, as_of)
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
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl CashflowProvider for FIIndexTotalReturnSwap {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we'll return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = self.schedule.period_schedule()?;

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for FIIndexTotalReturnSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.financing.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for FIIndexTotalReturnSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.financing.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for FIIndexTotalReturnSwap {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        // TRS financing leg typically uses the same curve for projection
        vec![self.financing.discount_curve_id.clone()]
    }
}
