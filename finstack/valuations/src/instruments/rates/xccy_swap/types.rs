//! XCCY swap types and pricing.
//!
//! Market-standard conventions implemented:
//! - Floating coupons projected from forward curves on accrual boundaries
//! - Cashflows discounted with the leg-specific discount curve (multi-curve)
//! - Leg PVs converted into the reporting currency using spot FX at `as_of`
//! - Explicit calendars / business-day conventions; no implicit calendar fallbacks
//!
//! Notes:
//! - This is a deterministic-curve pricer (no fixings). Reset lag is therefore not modeled
//!   separately; the forward rate is taken directly from the forward curve for the accrual period.

use finstack_core::currency::Currency;
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::{
    BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, HolidayCalendar, Schedule,
    ScheduleBuilder, StubKind, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::NeumaierAccumulator;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Whether the holder pays or receives a leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum LegSide {
    /// Receive the leg's coupons (and final notional, if exchanged).
    Receive,
    /// Pay the leg's coupons (and final notional, if exchanged).
    Pay,
}

impl LegSide {
    #[inline]
    fn coupon_sign(self) -> f64 {
        match self {
            Self::Receive => 1.0,
            Self::Pay => -1.0,
        }
    }

    #[inline]
    fn initial_principal_sign(self) -> f64 {
        // Market convention: the leg you "receive" is typically a lend position:
        // you pay principal at start, receive principal at end.
        match self {
            Self::Receive => -1.0,
            Self::Pay => 1.0,
        }
    }

    #[inline]
    fn final_principal_sign(self) -> f64 {
        -self.initial_principal_sign()
    }
}

/// Notional exchange convention for XCCY swaps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts_export", ts(export, rename_all = "snake_case"))]
pub enum NotionalExchange {
    /// No principal exchange.
    None,
    /// Exchange principal at maturity only.
    Final,
    /// Exchange principal at start and maturity (typical for XCCY basis swaps).
    #[default]
    InitialAndFinal,
}

/// One floating leg of an XCCY swap.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct XccySwapLeg {
    /// Leg currency.
    pub currency: Currency,
    /// Leg notional (in leg currency).
    pub notional: Money,
    /// Pay/receive direction for this leg.
    pub side: LegSide,
    /// Projection forward curve.
    pub forward_curve_id: CurveId,
    /// Discount curve for PV in leg currency.
    pub discount_curve_id: CurveId,
    /// Coupon frequency.
    pub frequency: Tenor,
    /// Accrual day count.
    pub day_count: DayCount,
    /// Business day convention for schedule dates.
    pub bdc: BusinessDayConvention,
    /// Spread added to the forward rate (decimal, e.g. 0.0001 = 1bp).
    #[cfg_attr(feature = "serde", serde(default))]
    pub spread: f64,
    /// Payment lag in business days after period end (default: 0).
    #[cfg_attr(feature = "serde", serde(default))]
    pub payment_lag_days: i32,
    /// Calendar identifier for schedule generation and lags.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub calendar_id: Option<String>,
    /// Allow calendar-day fallback when the calendar cannot be resolved.
    ///
    /// When `false` (default), missing calendars are treated as input errors.
    #[cfg_attr(feature = "serde", serde(default))]
    pub allow_calendar_fallback: bool,
}

impl XccySwapLeg {
    fn resolve_calendar(
        &self,
        instrument_id: &InstrumentId,
    ) -> Result<Option<&'static dyn HolidayCalendar>> {
        match self.calendar_id.as_deref() {
            Some(id) => {
                if let Some(cal) = CalendarRegistry::global().resolve_str(id) {
                    Ok(Some(cal))
                } else if self.allow_calendar_fallback {
                    tracing::warn!(
                        instrument_id = %instrument_id.as_str(),
                        calendar_id = id,
                        "Calendar not found; falling back to unadjusted schedule and calendar-day lags"
                    );
                    Ok(None)
                } else {
                    Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: format!("calendar '{}'", id),
                        },
                    ))
                }
            }
            None => {
                if self.allow_calendar_fallback {
                    tracing::warn!(
                        instrument_id = %instrument_id.as_str(),
                        "No calendar_id set; falling back to unadjusted schedule and calendar-day lags"
                    );
                    Ok(None)
                } else {
                    Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: "XccySwap leg calendar_id".to_string(),
                        },
                    ))
                }
            }
        }
    }
}

/// Cross-currency floating-for-floating swap.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct XccySwap {
    /// Unique identifier for this instrument.
    pub id: InstrumentId,
    /// Swap start date (typically spot).
    pub start_date: Date,
    /// Swap maturity date.
    pub maturity_date: Date,
    /// First leg.
    pub leg1: XccySwapLeg,
    /// Second leg.
    pub leg2: XccySwapLeg,
    /// Whether and when principal is exchanged.
    #[serde(default)]
    pub notional_exchange: NotionalExchange,
    /// PV reporting currency (output currency of `value`/`npv`).
    pub reporting_currency: Currency,
    /// Stub handling convention for irregular periods.
    #[serde(default)]
    pub stub_kind: StubKind,
    /// Attributes for instrument selection and tagging.
    pub attributes: crate::instruments::common::traits::Attributes,
}

impl XccySwap {
    /// Convenience constructor.
    pub fn new(
        id: impl Into<String>,
        start_date: Date,
        maturity_date: Date,
        leg1: XccySwapLeg,
        leg2: XccySwapLeg,
        reporting_currency: Currency,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            start_date,
            maturity_date,
            leg1,
            leg2,
            notional_exchange: NotionalExchange::InitialAndFinal,
            reporting_currency,
            stub_kind: StubKind::None,
            attributes: crate::instruments::common::traits::Attributes::default(),
        }
    }

    /// Set stub handling convention.
    pub fn with_stub(mut self, stub_kind: StubKind) -> Self {
        self.stub_kind = stub_kind;
        self
    }

    /// Set notional exchange convention.
    pub fn with_notional_exchange(mut self, exchange: NotionalExchange) -> Self {
        self.notional_exchange = exchange;
        self
    }

    fn validate_leg(&self, leg: &XccySwapLeg) -> Result<()> {
        if leg.notional.currency() != leg.currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: leg.currency,
                actual: leg.notional.currency(),
            });
        }
        if leg.payment_lag_days < 0 {
            return Err(finstack_core::Error::Validation(
                "XccySwap payment lag must be non-negative".to_string(),
            ));
        }
        if !leg.spread.is_finite() {
            return Err(finstack_core::Error::Validation(
                "XccySwap spread must be finite".to_string(),
            ));
        }
        Ok(())
    }

    fn leg_schedule(&self, leg: &XccySwapLeg) -> Result<Schedule> {
        let cal = leg.resolve_calendar(&self.id)?;
        let mut builder = ScheduleBuilder::try_new(self.start_date, self.maturity_date)?
            .frequency(leg.frequency)
            .stub_rule(self.stub_kind);

        if let Some(cal) = cal {
            builder = builder.adjust_with(leg.bdc, cal);
        }

        builder.build()
    }

    fn pv_leg_in_leg_ccy(
        &self,
        leg: &XccySwapLeg,
        schedule: &Schedule,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if schedule.dates.len() < 2 {
            return Err(finstack_core::Error::Validation(
                "XccySwap leg schedule must contain at least 2 dates".to_string(),
            ));
        }

        self.validate_leg(leg)?;

        // Curves
        let disc = context.get_discount_ref(&leg.discount_curve_id)?;
        let fwd = context.get_forward_ref(&leg.forward_curve_id)?;
        let cal = leg.resolve_calendar(&self.id)?;

        let dc_ctx = DayCountCtx::default();

        let mut pv = NeumaierAccumulator::new();

        // Notional exchanges (principal) in leg currency
        if matches!(self.notional_exchange, NotionalExchange::InitialAndFinal)
            && self.start_date > as_of
        {
            let df = disc.df_between_dates(as_of, self.start_date)?;
            pv.add(leg.side.initial_principal_sign() * leg.notional.amount() * df);
        }

        if matches!(
            self.notional_exchange,
            NotionalExchange::Final | NotionalExchange::InitialAndFinal
        ) && self.maturity_date > as_of
        {
            let df = disc.df_between_dates(as_of, self.maturity_date)?;
            pv.add(leg.side.final_principal_sign() * leg.notional.amount() * df);
        }

        // Floating coupons
        for i in 1..schedule.dates.len() {
            let period_start = schedule.dates[i - 1];
            let period_end = schedule.dates[i];

            let payment_date = if leg.payment_lag_days == 0 {
                period_end
            } else if let Some(cal) = cal {
                period_end.add_business_days(leg.payment_lag_days, cal)?
            } else {
                period_end + time::Duration::days(leg.payment_lag_days as i64)
            };

            if payment_date <= as_of {
                continue;
            }

            // Forward rate for the accrual period using the forward curve's own time basis
            let fwd_dc = fwd.day_count();
            let fwd_base = fwd.base_date();
            let t_start = fwd_dc.year_fraction(fwd_base, period_start, dc_ctx)?;
            let t_end = fwd_dc.year_fraction(fwd_base, period_end, dc_ctx)?;
            let forward_rate = fwd.rate_period(t_start, t_end);
            if !forward_rate.is_finite() {
                return Err(finstack_core::Error::Validation(
                    "Non-finite forward rate".to_string(),
                ));
            }

            let total_rate = forward_rate + leg.spread;
            let year_frac = leg
                .day_count
                .year_fraction(period_start, period_end, dc_ctx)?;
            let coupon = leg.side.coupon_sign() * leg.notional.amount() * total_rate * year_frac;

            let df = disc.df_between_dates(as_of, payment_date)?;
            pv.add(coupon * df);
        }

        Ok(Money::new(pv.total(), leg.currency))
    }

    fn convert(
        &self,
        amount: Money,
        to: Currency,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if amount.currency() == to {
            return Ok(amount);
        }
        let fx = market.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;
        let rate = fx.rate(FxQuery::new(amount.currency(), to, as_of))?.rate;
        Ok(Money::new(amount.amount() * rate, to))
    }

    /// Net PV (reporting currency): PV(leg1) + PV(leg2), after FX conversion of each leg PV.
    pub fn npv(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.validate_leg(&self.leg1)?;
        self.validate_leg(&self.leg2)?;

        let s1 = self.leg_schedule(&self.leg1)?;
        let s2 = self.leg_schedule(&self.leg2)?;

        let pv1 = self.pv_leg_in_leg_ccy(&self.leg1, &s1, market, as_of)?;
        let pv2 = self.pv_leg_in_leg_ccy(&self.leg2, &s2, market, as_of)?;

        let pv1_rep = self.convert(pv1, self.reporting_currency, market, as_of)?;
        let pv2_rep = self.convert(pv2, self.reporting_currency, market, as_of)?;
        pv1_rep + pv2_rep
    }
}

impl crate::instruments::common::traits::Instrument for XccySwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::XccySwap
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::instruments::common::traits::CurveDependencies for XccySwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.leg1.discount_curve_id.clone())
            .discount(self.leg2.discount_curve_id.clone())
            .forward(self.leg1.forward_curve_id.clone())
            .forward(self.leg2.forward_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for XccySwap {
    fn discount_curve_id(&self) -> &CurveId {
        if self.leg1.currency == self.reporting_currency {
            &self.leg1.discount_curve_id
        } else if self.leg2.currency == self.reporting_currency {
            &self.leg2.discount_curve_id
        } else {
            &self.leg1.discount_curve_id
        }
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for XccySwap {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        vec![
            self.leg1.forward_curve_id.clone(),
            self.leg2.forward_curve_id.clone(),
        ]
    }
}
