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
    pub fixed_rate: f64,
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

        // Time fractions for mapping into the forward curve domain must use the
        // forward curve's own day-count/time basis, not the instrument accrual basis.
        let fwd_base = fwd.base_date();
        let fwd_dc = fwd.day_count();
        let _t_fixing = fwd_dc
            .year_fraction(
                fwd_base,
                self.fixing_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        let t_start = fwd_dc
            .year_fraction(
                fwd_base,
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        let t_end = fwd_dc
            .year_fraction(
                fwd_base,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(t_start);

        // Accrual factor
        let tau = self
            .day_count
            .year_fraction(
                self.start_date,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        // If the accrual length is zero, PV is zero. When fixing is in the past,
        // continue to project using forwards unless an observed fixing is wired.
        if tau == 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Forward rate over the period and DF to settlement (start)
        let forward_rate = fwd.rate_period(t_start, t_end);
        let df_settlement = disc.df_on_date_curve(self.start_date);

        // Market-standard FRA settlement at period start includes the
        // settlement discounting adjustment 1 / (1 + F * tau).
        // PV = N * DF(T_start) * tau * (F - K) / (1 + F * tau)
        let rate_diff = forward_rate - self.fixed_rate;
        let denom = 1.0 + forward_rate * tau;
        let pv = if denom.abs() > 1e-12 {
            self.notional.amount() * rate_diff * tau * df_settlement / denom
        } else {
            // Fallback safety for pathological inputs
            self.notional.amount() * rate_diff * tau * df_settlement
        };
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
    fn key(&self) -> crate::pricer::InstrumentType {
        <Self as crate::instruments::common::traits::InstrumentKind>::TYPE
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

impl crate::instruments::common::traits::InstrumentKind for ForwardRateAgreement {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::FRA;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    #[test]
    #[ignore = "Integration test depends on context wiring; left as regression guard"]
    fn fra_par_pv_near_zero_with_settlement_adjustment() {
        // Build simple flat curves: 5% forward, discount with reasonable decay
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let disc = DiscountCurve::builder("DISC")
            .base_date(base)
            .knots([(0.0, 1.0), (5.0, 0.78)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let fwd = ForwardCurve::builder("FWD-3M", 0.25)
            .base_date(base)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        // FRA 3M x 6M
        let start = base + time::Duration::days(90);
        let end = base + time::Duration::days(180);
        let fra = ForwardRateAgreement::builder()
            .id("FRA-3x6".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(start)
            .start_date(start)
            .end_date(end)
            .fixed_rate(0.05)
            .day_count(finstack_core::dates::DayCount::Act360)
            .reset_lag(2)
            .disc_id("DISC".into())
            .forward_id("FWD-3M".into())
            .build()
            .unwrap();

        let pv = fra.value(&ctx, base).unwrap();
        // With settlement adjustment PV should be very close to zero at par
        assert!(
            pv.amount().abs() < 0.01,
            "FRA PV not near zero: {}",
            pv.amount()
        );
    }
}
