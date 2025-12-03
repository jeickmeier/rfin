//! Forward Rate Agreement (FRA) instrument types and trait implementations.
//!
//! Defines the `ForwardRateAgreement` instrument in the modern instrument style
//! used across valuations. Core PV logic is delegated to the pricing engine in
//! `pricing::engine`, and metrics are provided in the `metrics` submodule.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{
    adjust, calendar::registry::CalendarRegistry, BusinessDayConvention, Date, DayCount,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
    /// Optional fixing calendar identifier for business day adjustment
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_calendar_id: Option<String>,
    /// Optional business day convention for fixing date adjustment (default: ModifiedFollowing)
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_bdc: Option<BusinessDayConvention>,
    /// Optional observed fixing (locked rate) when known
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub observed_fixing: Option<f64>,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Pay/receive flag (true = receive fixed, pay floating)
    pub pay_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl ForwardRateAgreement {
    /// Create a canonical example FRA for testing and documentation.
    ///
    /// Returns a 3x6 FRA (3 months forward, 3 month tenor).
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FRA-3X6-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .fixing_date(
                Date::from_calendar_date(2024, time::Month::April, 1).expect("Valid example date"),
            )
            .start_date(
                Date::from_calendar_date(2024, time::Month::April, 3).expect("Valid example date"),
            )
            .end_date(
                Date::from_calendar_date(2024, time::Month::July, 3).expect("Valid example date"),
            )
            .fixed_rate(0.045)
            .day_count(DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .forward_id(CurveId::new("USD-SOFR-3M"))
            .pay_fixed(true)
            .attributes(Attributes::new())
            .build()
            .expect("Example FRA construction should not fail")
    }

    /// Calculate the net present value of this FRA
    pub fn npv(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Settlement for a FRA occurs at the start of the accrual period; past
        // settlement implies zero PV.
        if as_of >= self.start_date {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Derive fixing date from reset lag with calendar/BDC adjustment.
        let mut inferred_fixing_date =
            self.start_date - time::Duration::days(self.reset_lag as i64);
        let bdc = self
            .fixing_bdc
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
        if let Some(cal_id) = self.fixing_calendar_id.as_deref() {
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                inferred_fixing_date = adjust(inferred_fixing_date, bdc, cal)?;
            }
        }
        let fixing_date = if self.fixing_date != inferred_fixing_date {
            inferred_fixing_date
        } else {
            self.fixing_date
        };

        let disc = context.get_discount_ref(&self.discount_curve_id)?;
        let fwd = context.get_forward_ref(&self.forward_id)?;

        // Time fractions for mapping into the forward curve domain must use the
        // forward curve's own day-count/time basis, not the instrument accrual basis.
        let fwd_base = fwd.base_date();
        let fwd_dc = fwd.day_count();
        let t_fixing = fwd_dc
            .year_fraction(
                fwd_base,
                fixing_date,
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
        // If fixing date has passed, prefer observed fixing when available; otherwise
        // anchor projection at the fixing horizon to avoid theta drift.
        let forward_rate = if as_of >= fixing_date {
            if let Some(obs) = self.observed_fixing {
                obs
            } else {
                fwd.rate_period(t_start.max(t_fixing), t_end)
            }
        } else {
            fwd.rate_period(t_start, t_end)
        };

        // Discount from as_of date for correct theta calculation
        let disc_dc = disc.day_count();
        let t_settle_from_as_of = disc_dc
            .year_fraction(
                as_of,
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_as_of_from_base = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let df_as_of = disc.df(t_as_of_from_base);
        let df_settle_abs = disc.df(t_as_of_from_base + t_settle_from_as_of);
        let df_settlement = if df_as_of != 0.0 {
            df_settle_abs / df_as_of
        } else {
            1.0
        };

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
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FRA
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

impl crate::instruments::common::pricing::HasDiscountCurve for ForwardRateAgreement {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for ForwardRateAgreement {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for ForwardRateAgreement {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.forward_id.clone()]
    }
}

impl CashflowProvider for ForwardRateAgreement {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

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
    #[cfg(feature = "slow")]
    use super::*;
    #[cfg(feature = "slow")]
    use crate::instruments::common::traits::Instrument;
    #[cfg(feature = "slow")]
    use finstack_core::currency::Currency;
    #[cfg(feature = "slow")]
    use finstack_core::dates::Date;
    #[cfg(feature = "slow")]
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    #[cfg(feature = "slow")]
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    #[cfg(feature = "slow")]
    use finstack_core::math::interp::InterpStyle;
    #[cfg(feature = "slow")]
    use time::Month;

    #[test]
    #[cfg(feature = "slow")]
    fn fra_par_pv_near_zero_with_settlement_adjustment() {
        // Build simple flat curves: 5% forward, discount with reasonable decay
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = DiscountCurve::builder("DISC")
            .base_date(base)
            .knots([(0.0, 1.0), (5.0, 0.78)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let fwd = ForwardCurve::builder("FWD-3M", 0.25)
            .base_date(base)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        // FRA 3M x 6M
        let start = base + time::Duration::days(90);
        let end = base + time::Duration::days(180);
        let fixing = start - time::Duration::days(2); // 2 days before start for reset_lag
        let fra = ForwardRateAgreement::builder()
            .id("FRA-3x6".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(fixing)
            .start_date(start)
            .end_date(end)
            .fixed_rate(0.05)
            .day_count(finstack_core::dates::DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id("DISC".into())
            .forward_id("FWD-3M".into())
            .pay_fixed(false) // Receive fixed, pay floating
            .build()
            .expect("FRA builder should succeed in test");

        let pv = fra
            .value(&ctx, base)
            .expect("FRA valuation should succeed in test");
        // With settlement adjustment PV should be very close to zero at par
        assert!(
            pv.amount().abs() < 0.01,
            "FRA PV not near zero: {}",
            pv.amount()
        );
    }

    #[test]
    #[cfg(feature = "slow")]
    fn fra_par_rate_metric() {
        // Build simple flat curves
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = DiscountCurve::builder("DISC")
            .base_date(base)
            .knots([(0.0, 1.0), (5.0, 0.78)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let fwd_rate = 0.05;
        let fwd = ForwardCurve::builder("FWD-3M", 0.25)
            .base_date(base)
            .knots([(0.0, fwd_rate), (5.0, fwd_rate)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        let start = base + time::Duration::days(90);
        let end = base + time::Duration::days(180);
        let fixing = start - time::Duration::days(2);

        let fra = ForwardRateAgreement::builder()
            .id("FRA-TEST".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(fixing)
            .start_date(start)
            .end_date(end)
            .fixed_rate(0.04) // Different from market rate
            .day_count(finstack_core::dates::DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id("DISC".into())
            .forward_id("FWD-3M".into())
            .pay_fixed(true)
            .build()
            .expect("Builder failed");

        use crate::instruments::common::traits::Instrument;
        use crate::instruments::fra::metrics::FraParRateCalculator;
        use crate::metrics::{MetricCalculator, MetricContext};
        use std::sync::Arc;

        // Wrap in Arc for metric context
        let fra_arc = Arc::new(fra);
        let ctx_arc = Arc::new(ctx);

        // Calculate base PV
        let base_pv = fra_arc
            .value(&ctx_arc, base)
            .expect("PV calculation failed");

        let calc = FraParRateCalculator;
        let mut m_ctx = MetricContext::new(fra_arc as Arc<dyn Instrument>, ctx_arc, base, base_pv);

        let par_rate = calc
            .calculate(&mut m_ctx)
            .expect("Par rate calculation failed");

        assert!(
            (par_rate - fwd_rate).abs() < 1e-10,
            "Par rate {} should equal forward rate {}",
            par_rate,
            fwd_rate
        );
    }
}
