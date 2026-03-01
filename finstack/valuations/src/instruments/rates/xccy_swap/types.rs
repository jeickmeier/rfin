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

use crate::impl_instrument_base;
use crate::instruments::common_impl::pricing::swap_legs::robust_relative_df;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Schedule, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::NeumaierAccumulator;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Threshold for extremely negative forward rates that warrant a warning.
/// Even JPY/CHF/EUR rarely go below -1%, so -5% indicates potential curve issues.
const EXTREME_NEGATIVE_RATE_THRESHOLD: f64 = -0.05;

/// Whether the holder pays or receives a leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
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

    /// Returns the sign for initial principal exchange.
    ///
    /// # Market Convention
    ///
    /// The leg you "receive" is economically a lending position:
    /// - **At start**: you pay out principal (negative cashflow) to the counterparty
    /// - **During**: you receive interest coupons (positive cashflows)
    /// - **At end**: you receive principal back (positive cashflow)
    ///
    /// # Example
    ///
    /// For a USD/EUR XCCY swap where you receive USD:
    /// - Initial exchange: you pay USD notional to counterparty (-1.0 sign)
    /// - Final exchange: you receive USD notional back (+1.0 sign)
    ///
    /// This follows ISDA conventions where the receiver of a leg provides
    /// the initial funding in that currency.
    #[inline]
    fn initial_principal_sign(self) -> f64 {
        match self {
            Self::Receive => -1.0,
            Self::Pay => 1.0,
        }
    }

    /// Returns the sign for final principal exchange (opposite of initial).
    #[inline]
    fn final_principal_sign(self) -> f64 {
        -self.initial_principal_sign()
    }
}

/// Notional exchange convention for XCCY swaps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts_export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts_export", ts(export, rename_all = "snake_case"))]
#[non_exhaustive]
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
///
/// Each leg owns its own dates, discount curve, calendar, and stub conventions,
/// following the IRS leg-centric pattern.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Start date of the leg.
    pub start: Date,
    /// End date of the leg.
    pub end: Date,
    /// Coupon frequency.
    pub frequency: Tenor,
    /// Accrual day count.
    pub day_count: DayCount,
    /// Business day convention for schedule dates.
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Stub period handling rule.
    #[serde(
        default = "crate::serde_defaults::stub_short_front",
        alias = "stub_kind"
    )]
    pub stub: StubKind,
    /// Spread in basis points (e.g. `Decimal::from(5)` = 5bp).
    #[serde(default, alias = "spread")]
    pub spread_bp: Decimal,
    /// Payment lag in business days after period end (default: 0).
    #[serde(default)]
    pub payment_lag_days: i32,
    /// Calendar identifier for schedule generation and lags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
    /// Allow calendar-day fallback when the calendar cannot be resolved.
    ///
    /// When `false` (default), missing calendars are treated as input errors.
    #[serde(default)]
    pub allow_calendar_fallback: bool,
}

/// Cross-currency floating-for-floating swap.
///
/// Each leg owns its own dates, stub conventions, and calendar. The parent struct
/// only holds the instrument identity, notional exchange mode, and reporting currency.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct XccySwap {
    /// Unique identifier for this instrument.
    pub id: InstrumentId,
    /// First leg.
    pub leg1: XccySwapLeg,
    /// Second leg.
    pub leg2: XccySwapLeg,
    /// Whether and when principal is exchanged.
    #[serde(default)]
    pub notional_exchange: NotionalExchange,
    /// PV reporting currency (output currency of `value`/`npv`).
    pub reporting_currency: Currency,
    /// Attributes for instrument selection and tagging.
    pub attributes: crate::instruments::common_impl::traits::Attributes,
}

impl XccySwap {
    /// Convenience constructor.
    ///
    /// Dates and stub conventions are now owned by each leg.
    pub fn new(
        id: impl Into<String>,
        leg1: XccySwapLeg,
        leg2: XccySwapLeg,
        reporting_currency: Currency,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            leg1,
            leg2,
            notional_exchange: NotionalExchange::InitialAndFinal,
            reporting_currency,
            attributes: crate::instruments::common_impl::traits::Attributes::default(),
        }
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
        // Validate notional is finite and positive
        if !leg.notional.amount().is_finite() {
            return Err(finstack_core::Error::Validation(
                "XccySwap leg notional must be finite".to_string(),
            ));
        }
        if leg.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(
                "XccySwap leg notional must be positive".to_string(),
            ));
        }
        if leg.payment_lag_days < 0 {
            return Err(finstack_core::Error::Validation(
                "XccySwap payment lag must be non-negative".to_string(),
            ));
        }
        // Decimal is always finite; no NaN/infinity check required.
        Ok(())
    }

    fn leg_schedule(&self, leg: &XccySwapLeg) -> Result<Schedule> {
        let sched = crate::cashflow::builder::build_dates(
            leg.start,
            leg.end,
            leg.frequency,
            leg.stub,
            leg.bdc,
            false,
            leg.payment_lag_days,
            leg.calendar_id
                .as_deref()
                .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
        )?;
        Ok(Schedule {
            dates: sched.dates,
            warnings: Vec::new(),
        })
    }

    /// Calculate the present value of a leg with per-cashflow FX conversion.
    ///
    /// # Market-Standard FX Conversion
    ///
    /// For cross-currency swaps, each cashflow should be converted to the reporting
    /// currency using the FX rate applicable on the cashflow's payment date, not
    /// the spot rate at `as_of`. This respects covered interest parity (CIP).
    ///
    /// If FxMatrix is not available or only provides spot rates, the conversion is
    /// an approximation (documented in output).
    ///
    /// # Returns
    ///
    /// Present value in the **reporting currency**, not the leg currency.
    fn pv_leg_in_reporting_ccy(
        &self,
        leg: &XccySwapLeg,
        schedule: &Schedule,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        use crate::instruments::common_impl::pricing::time::rate_period_on_dates;

        if schedule.dates.is_empty() {
            return Err(finstack_core::Error::Validation(
                "XccySwap leg schedule must contain at least 2 dates".to_string(),
            ));
        }

        self.validate_leg(leg)?;

        // Curves
        let disc = context.get_discount(&leg.discount_curve_id)?;
        let fwd = context.get_forward(&leg.forward_curve_id)?;
        let fx = context.fx();

        let mut pv = NeumaierAccumulator::new();

        // Helper to convert a single cashflow to reporting currency.
        //
        // Note: FxQuery uses payment_date for forward FX rate lookup per CIP conventions.
        // If FxMatrix only contains spot rates, the conversion is an approximation.
        // For long-dated cashflows (>1Y), this approximation can be material (>1% error
        // depending on interest rate differentials).
        let mut fx_approximation_warned = false;
        let convert_cf = |amount: f64, payment_date: Date, fx_warned: &mut bool| -> Result<f64> {
            if leg.currency == self.reporting_currency {
                return Ok(amount);
            }
            let fx_matrix = fx.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                })
            })?;

            // Warn once if we're converting cashflows >1Y in the future, as spot FX
            // approximation error grows with tenor (roughly proportional to rate differential × time).
            let days_forward = (payment_date - as_of).whole_days();
            if days_forward > 365 && !*fx_warned {
                tracing::warn!(
                    instrument_id = %self.id.as_str(),
                    from_ccy = %leg.currency,
                    to_ccy = %self.reporting_currency,
                    payment_date = %payment_date,
                    days_forward = days_forward,
                    "XCCY swap FX conversion for cashflow >1Y forward. If FxMatrix provides \
                     spot rates only, PV may have material approximation error. For accurate \
                     pricing, provide forward FX rates consistent with covered interest parity."
                );
                *fx_warned = true;
            }

            // Use cashflow-date FX (default policy is CashflowDate)
            let rate = fx_matrix
                .rate(FxQuery::new(
                    leg.currency,
                    self.reporting_currency,
                    payment_date,
                ))?
                .rate;
            Ok(amount * rate)
        };

        // Notional exchanges (principal)
        // Use robust_relative_df for numerical stability (validated against Bloomberg SWPM)
        if matches!(self.notional_exchange, NotionalExchange::InitialAndFinal) && leg.start > as_of
        {
            let df = robust_relative_df(disc.as_ref(), as_of, leg.start)?;
            let cf_leg_ccy = leg.side.initial_principal_sign() * leg.notional.amount() * df;
            let cf_rep = convert_cf(cf_leg_ccy, leg.start, &mut fx_approximation_warned)?;
            pv.add(cf_rep);
        }

        if matches!(
            self.notional_exchange,
            NotionalExchange::Final | NotionalExchange::InitialAndFinal
        ) && leg.end > as_of
        {
            let df = robust_relative_df(disc.as_ref(), as_of, leg.end)?;
            let cf_leg_ccy = leg.side.final_principal_sign() * leg.notional.amount() * df;
            let cf_rep = convert_cf(cf_leg_ccy, leg.end, &mut fx_approximation_warned)?;
            pv.add(cf_rep);
        }

        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: leg.start,
                end: leg.end,
                frequency: leg.frequency,
                stub: leg.stub,
                bdc: leg.bdc,
                calendar_id: leg
                    .calendar_id
                    .as_deref()
                    .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
                end_of_month: false,
                day_count: leg.day_count,
                payment_lag_days: leg.payment_lag_days,
                reset_lag_days: None,
            },
        )?;

        if periods.is_empty() {
            return Err(finstack_core::Error::Validation(
                "XccySwap leg schedule must contain at least 1 period".to_string(),
            ));
        }

        // Floating coupons
        for period in periods {
            if period.payment_date <= as_of {
                continue;
            }

            // Forward rate using forward curve's time basis
            let forward_rate =
                rate_period_on_dates(fwd.as_ref(), period.accrual_start, period.accrual_end)?;
            if !forward_rate.is_finite() {
                return Err(finstack_core::Error::Validation(format!(
                    "Non-finite forward rate for period {} to {}",
                    period.accrual_start, period.accrual_end
                )));
            }
            // Warn about extremely negative forward rates which may indicate curve issues.
            // Even in negative rate environments (JPY/CHF/EUR), rates below -5% are unusual.
            if forward_rate < EXTREME_NEGATIVE_RATE_THRESHOLD {
                tracing::warn!(
                    instrument_id = %self.id.as_str(),
                    period_start = %period.accrual_start,
                    period_end = %period.accrual_end,
                    forward_rate = forward_rate,
                    threshold = EXTREME_NEGATIVE_RATE_THRESHOLD,
                    "Forward rate is highly negative; verify curve construction"
                );
            }

            let total_rate = forward_rate + leg.spread_bp.to_f64().unwrap_or_default() / 10_000.0;
            let coupon = leg.side.coupon_sign()
                * leg.notional.amount()
                * total_rate
                * period.accrual_year_fraction;

            // Use robust_relative_df for numerical stability
            let df = robust_relative_df(disc.as_ref(), as_of, period.payment_date)?;
            let cf_leg_ccy = coupon * df;

            // Convert to reporting currency using cashflow-date FX
            let cf_rep = convert_cf(
                cf_leg_ccy,
                period.payment_date,
                &mut fx_approximation_warned,
            )?;
            pv.add(cf_rep);
        }

        Ok(Money::new(pv.total(), self.reporting_currency))
    }
}

impl crate::instruments::common_impl::traits::Instrument for XccySwap {
    impl_instrument_base!(crate::pricer::InstrumentType::XccySwap);

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        self.validate_leg(&self.leg1)?;
        self.validate_leg(&self.leg2)?;

        if self.leg1.currency == self.leg2.currency {
            return Err(finstack_core::Error::Validation(format!(
                "XccySwap legs must have different currencies; both are {}. \
                 Use BasisSwap for same-currency basis trades.",
                self.leg1.currency
            )));
        }

        let s1 = self.leg_schedule(&self.leg1)?;
        let s2 = self.leg_schedule(&self.leg2)?;

        // Each leg's PV is computed and converted to reporting currency per-cashflow
        let pv1_rep = self.pv_leg_in_reporting_ccy(&self.leg1, &s1, market, as_of)?;
        let pv2_rep = self.pv_leg_in_reporting_ccy(&self.leg2, &s2, market, as_of)?;

        pv1_rep.checked_add(pv2_rep)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.leg1.start)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for XccySwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.leg1.discount_curve_id.clone())
            .discount(self.leg2.discount_curve_id.clone())
            .forward(self.leg1.forward_curve_id.clone())
            .forward(self.leg2.forward_curve_id.clone())
            .build()
    }
}
