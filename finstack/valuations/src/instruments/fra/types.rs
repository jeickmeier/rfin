//! Forward Rate Agreement (FRA) instrument types and trait implementations.
//!
//! Defines the `ForwardRateAgreement` instrument in the modern instrument style
//! used across valuations. Core PV logic is delegated to the pricing engine in
//! `pricing::engine`, and metrics are provided in the `metrics` submodule.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct ForwardRateAgreement {
    /// Unique identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Rate fixing date (start of interest period)
    pub fixing_date: Date,
    /// Interest period start date
    pub start_date: Date,
    /// Interest period end date
    pub end_date: Date,
    /// Fixed rate (decimal, e.g., 0.05 for 5%)
    pub fixed_rate: F,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Reset lag in business days (fixing to value date)
    pub reset_lag: i32,
    /// Discount curve identifier
    pub disc_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Pay/receive flag (true = receive fixed, pay floating)
    pub pay_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl ForwardRateAgreement {
    /// Calculate the net present value of this FRA
    pub fn npv(
        &self,
        context: &finstack_core::market_data::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.disc_id.clone())?;
        let fwd = context.get_forward_ref(self.forward_id.clone())?;

        // Time fractions
        let base_date = disc.base_date();
        let _t_fixing = self
            .day_count
            .year_fraction(base_date, self.fixing_date, finstack_core::dates::DayCountCtx::default())?
            .max(0.0);
        let t_start = self
            .day_count
            .year_fraction(base_date, self.start_date, finstack_core::dates::DayCountCtx::default())?
            .max(0.0);
        let t_end = self
            .day_count
            .year_fraction(base_date, self.end_date, finstack_core::dates::DayCountCtx::default())?
            .max(t_start);

        // Accrual factor
        let tau = self
            .day_count
            .year_fraction(self.start_date, self.end_date, finstack_core::dates::DayCountCtx::default())?
            .max(0.0);
        // If the accrual length is zero, PV is zero. When fixing is in the past,
        // continue to project using forwards unless an observed fixing is wired.
        if tau == 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Forward rate over the period and DF to settlement (start)
        let forward_rate = fwd.rate_period(t_start, t_end);
        let df_settlement = disc.df_on_date_curve(self.start_date);

        let rate_diff = forward_rate - self.fixed_rate;
        let pv = self.notional.amount() * rate_diff * tau * df_settlement;
        let signed_pv = if self.pay_fixed { -pv } else { pv };
        Ok(Money::new(signed_pv, self.notional.currency()))
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for ForwardRateAgreement {
    #[inline]
    fn id(&self) -> &str {
        self.id.as_str()
    }

    #[inline]
    fn instrument_type(&self) -> &'static str {
        "FRA"
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

impl crate::instruments::common::HasDiscountCurve for ForwardRateAgreement {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}

impl CashflowProvider for ForwardRateAgreement {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Settlement at start of accrual period; if already settled, no flows
        if self.start_date <= as_of {
            return Ok(vec![]);
        }

        let pv = self.npv(curves, as_of)?;
        Ok(vec![(self.start_date, pv)])
    }
}
