//! Interest Rate Swap (IRS) types and instrument trait implementations.
//!
//! Defines the `InterestRateSwap` instrument following the modern instrument
//! standards used across valuations: types live here; pricing is delegated to
//! `pricing::engine`; and metrics are split under `metrics/`.
//!
//! Public fields use strong newtype identifiers for safety: `InstrumentId` and
//! `CurveId`. Calendar identifiers remain `Option<&'static str>` for stable
//! serde and lookups.
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::market_data::traits::Forward;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::currency::Currency;
use finstack_core::{dates::Date, Result};

use crate::cashflow::builder::{
    CashFlowSchedule, CouponType, FixedCouponSpec, FloatingRateSpec, ScheduleParams,
};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
// discountable helpers not used after switching to curve-based df_on_date_curve
use crate::instruments::common::traits::Attributes;
// Risk types used in risk.rs

// Re-export common enums from parameters
pub use crate::instruments::common::parameters::legs::{ParRateMethod, PayReceive};

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::FixedLegSpec;
pub use crate::instruments::common::parameters::legs::FloatLegSpec;

/// Interest rate swap with fixed and floating legs.
///
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
///
/// # Market Standards & Citations (Week 5)
///
/// ## ISDA Definitions
///
/// This implementation follows the **ISDA 2006 Definitions** for interest rate derivatives:
/// - **Section 4.1:** Fixed Rate Payer calculation conventions
/// - **Section 4.2:** Floating Rate Option conventions
/// - **Section 4.5:** Compounding methods
/// - **Section 4.16:** Business Day Conventions
///
/// ## USD Market Standard (Default)
///
/// Per **ISDA 2006 Definitions** and US market practice:
/// - **Fixed Leg:** Semi-annual, 30/360, Modified Following
/// - **Floating Leg:** Quarterly, ACT/360, Modified Following
/// - **Reset Lag:** T-2 (2 business days before period start)
/// - **Discounting:** OIS curve (post-2008 multi-curve framework)
///
/// ## References
///
/// - ISDA 2006 Definitions (incorporating 2008 Supplement for OIS)
/// - "Interest Rate Swaps and Their Derivatives" by Amir Sadr
/// - Bloomberg SWPM function documentation
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: InstrumentId,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Direction of the swap (PayFixed or ReceiveFixed).
    pub side: PayReceive,
    /// Fixed leg specification.
    pub fixed: FixedLegSpec,
    /// Floating leg specification.
    pub float: FloatLegSpec,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

/// Configuration for standard swap construction.
struct SwapConfig<'a> {
    disc_curve: &'a str,
    fwd_curve: &'a str,
    reset_lag_days: i32,
    sched: IRSScheduleConfig,
}

/// Schedule configuration with separate fixed and float leg parameters
struct IRSScheduleConfig {
    fixed_freq: finstack_core::dates::Frequency,
    fixed_dc: finstack_core::dates::DayCount,
    float_freq: finstack_core::dates::Frequency,
    float_dc: finstack_core::dates::DayCount,
    bdc: finstack_core::dates::BusinessDayConvention,
    calendar_id: Option<String>,
    stub: finstack_core::dates::StubKind,
}

impl IRSScheduleConfig {
    /// USD market standard: Fixed semiannual 30/360; Float quarterly Act/360
    fn usd_isda_standard() -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        Self {
            fixed_freq: Frequency::semi_annual(),
            fixed_dc: DayCount::Thirty360,
            float_freq: Frequency::quarterly(),
            float_dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USD".to_string()),
            stub: StubKind::None,
        }
    }
}

impl InterestRateSwap {
    /// Create a canonical example IRS for testing and documentation.
    ///
    /// Returns a 5-year pay-fixed swap with semi-annual fixed vs quarterly floating.
    pub fn example() -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        
        Self::builder()
            .id(InstrumentId::new("IRS-5Y-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .fixed(crate::instruments::common::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: 0.03,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                end: Date::from_calendar_date(2029, time::Month::January, 1).unwrap(),
                par_method: None,
                compounding_simple: true,
            })
            .float(crate::instruments::common::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start: Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                end: Date::from_calendar_date(2029, time::Month::January, 1).unwrap(),
            })
            .build()
            .expect("Example IRS construction should not fail")
    }

    /// Helper to construct a swap with specified curve configuration.
    fn create_swap(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        config: SwapConfig<'_>,
    ) -> Self {
        let fixed = FixedLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            rate: fixed_rate,
            freq: config.sched.fixed_freq,
            dc: config.sched.fixed_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.clone(),
            stub: config.sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            forward_curve_id: finstack_core::types::CurveId::from(config.fwd_curve),
            spread_bp: 0.0,
            freq: config.sched.float_freq,
            dc: config.sched.float_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.clone(),
            stub: config.sched.stub,
            reset_lag_days: config.reset_lag_days,
            start,
            end,
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("Swap construction should not fail")
    }

    /// Create a standard interest rate swap (most common use case).
    ///
    /// Creates a USD swap with standard market conventions. For other
    /// currencies or custom conventions, use `::with_convention()` or `::builder()`.
    ///
    /// # Example
    /// ```ignore
    /// let swap = InterestRateSwap::new(
    ///     "IRS-5Y".into(),
    ///     Money::new(10_000_000.0, Currency::USD),
    ///     0.03,
    ///     start,
    ///     end,
    ///     PayReceive::PayFixed
    /// );
    /// ```
    pub fn new(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> Self {
        Self::create_swap(
            id,
            notional,
            fixed_rate,
            start,
            end,
            side,
            SwapConfig {
                disc_curve: "USD-OIS",
                fwd_curve: "USD-SOFR-3M",
                reset_lag_days: 2,
                sched: IRSScheduleConfig::usd_isda_standard(),
            },
        )
    }

    /// Test-only constructor preserving historical quarterly/quarterly settings.
    /// Intended for internal tests that assume Q/Q Act/360 both legs.
    #[cfg(test)]
    pub fn new_quarterly_test_only(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        let sched = IRSScheduleConfig {
            fixed_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Act360,
            float_freq: Frequency::quarterly(),
            float_dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USD".to_string()),
            stub: StubKind::None,
        };
        Self::create_swap(
            id,
            notional,
            fixed_rate,
            start,
            end,
            side,
            SwapConfig {
                disc_curve: "USD-OIS",
                fwd_curve: "USD-SOFR-3M",
                reset_lag_days: 2,
                sched,
            },
        )
    }

    /// Create a swap with standard market conventions.
    ///
    /// Applies region-specific conventions for day count, frequency, calendars,
    /// and curve identifiers. For full customization, use `::builder()`.
    ///
    /// # Example
    /// ```ignore
    /// let swap = InterestRateSwap::with_convention(
    ///     "EUR-IRS-10Y".into(),
    ///     notional,
    ///     0.02,
    ///     start,
    ///     end,
    ///     PayReceive::PayFixed,
    ///     IRSConvention::EURStandard
    /// );
    /// ```
    pub fn with_convention(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        convention: crate::instruments::common::parameters::IRSConvention,
    ) -> Self {
        use finstack_core::dates::StubKind;
        use finstack_core::types::CurveId;

        let fixed_freq = convention.fixed_frequency();
        let float_freq = convention.float_frequency();
        let fixed_dc = convention.fixed_day_count();
        let float_dc = convention.float_day_count();
        let bdc = convention.business_day_convention();
        let calendar_id = convention.calendar_id();

        let fixed = FixedLegSpec {
            discount_curve_id: CurveId::from(convention.disc_curve_id()),
            rate: fixed_rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: calendar_id.clone(),
            stub: StubKind::None,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            discount_curve_id: CurveId::from(convention.disc_curve_id()),
            forward_curve_id: CurveId::from(convention.forward_curve_id()),
            spread_bp: 0.0,
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id,
            stub: StubKind::None,
            reset_lag_days: convention.reset_lag_days(),
            start,
            end,
        };

        Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("IRS with convention construction should not fail")
    }

    /// Create a basis swap (float vs float with different indices/spreads).
    pub fn usd_basis_swap(
        id: InstrumentId,
        notional: Money,
        start: Date,
        end: Date,
        primary_spread_bp: f64, // Spread on the "fixed" leg (really floating)
        reference_spread_bp: f64, // Spread on the "float" leg
    ) -> Self {
        // Approximate basis swap by using fixed leg to carry the primary spread as a fixed coupon
        let sched = ScheduleParams::usd_standard();
        let fixed = FixedLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from("USD-OIS"),
            rate: primary_spread_bp * 1e-4,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id.clone(),
            stub: sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from("USD-OIS"),
            forward_curve_id: finstack_core::types::CurveId::from("USD-SOFR-6M"),
            spread_bp: reference_spread_bp,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id.clone(),
            stub: sched.stub,
            reset_lag_days: 2,
            start,
            end,
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::PayFixed)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("USD basis swap construction should not fail")
    }

    /// Compute PV of fixed leg (helper for value calculation).
    pub(crate) fn pv_fixed_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let mut b = CashFlowSchedule::builder();
        b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id.clone(),
                stub: self.fixed.stub,
            });
        let sched = b.build()?;

        // Sum discounted coupon flows from as_of date
        let mut total = Money::new(0.0, self.notional.currency());
        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        for cf in &sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                // Only include future cashflows
                if cf.date <= as_of {
                    continue;
                }

                // Discount from as_of for correct theta
                let t_cf = disc_dc
                    .year_fraction(
                        disc.base_date(),
                        cf.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df_cf_abs = disc.df(t_cf);
                let df = if df_as_of != 0.0 {
                    df_cf_abs / df_as_of
                } else {
                    1.0
                };
                let disc_amt = cf.amount * df;
                total = (total + disc_amt)?;
            }
        }
        Ok(total)
    }

    /// Compute PV of floating leg (helper for value calculation).
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        fwd: &dyn Forward,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let builder = finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
            .frequency(self.float.freq)
            .stub_rule(self.float.stub);

        let sched_dates: Vec<Date> = {
            let sched = if let Some(id) = &self.float.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    builder.adjust_with(self.float.bdc, cal).build()?
                } else {
                    builder.build()?
                }
            } else {
                builder.build()?
            };
            sched.into_iter().collect()
        };

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let mut prev = sched_dates[0];
        let mut total = Money::new(0.0, self.notional.currency());

        // Pre-compute as_of discount factor
        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        for &d in &sched_dates[1..] {
            // Only include future cashflows
            if d <= as_of {
                prev = d;
                continue;
            }

            let base = disc.base_date();
            let t1 = self
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = self
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = self
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);

            // Only call rate_period if t1 < t2 to avoid date ordering errors
            let f = if t2 > t1 {
                fwd.rate_period(t1, t2)
            } else {
                0.0
            };
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);

            // Discount from as_of for correct theta
            let t_cf = disc_dc
                .year_fraction(
                    disc.base_date(),
                    d,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let df_cf_abs = disc.df(t_cf);
            let df = if df_as_of != 0.0 {
                df_cf_abs / df_as_of
            } else {
                1.0
            };
            let disc_amt = coupon * df;
            total = (total + disc_amt)?;
            prev = d;
        }
        Ok(total)
    }

    /// Calculates the present value of this IRS by composing leg PVs.
    ///
    /// Provides deterministic present value calculation for a vanilla
    /// fixed-for-floating interest rate swap. Uses the instrument
    /// day-counts for accrual and the discount curve's own date helpers for
    /// discounting to ensure policy visibility and currency safety.
    ///
    /// PV = sign × (PV_fixed − PV_float) with sign determined by `PayReceive`.
    pub fn npv(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let disc = context.get_discount_ref(self.fixed.discount_curve_id.as_ref())?;
        // For OIS swaps (same discount curve for both legs), use discount-only pricing
        // even if a forward curve is provided. This ensures correct compounding for
        // daily frequency OIS swaps. The discount-only method properly handles
        // compounded OIS rates: PV_float = N × (DF_start - DF_end).
        let pv_fixed = self.pv_fixed_leg(disc, as_of)?;
        let pv_float = if self.float.discount_curve_id == self.fixed.discount_curve_id {
            // OIS swap: use discount-only method for accurate pricing
            // This handles daily compounded OIS rates correctly
            // Discount from as_of for correct theta
            let disc_dc = disc.day_count();
            let t_as_of = disc_dc
                .year_fraction(
                    disc.base_date(),
                    as_of,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let df_as_of = disc.df(t_as_of);

            let t_start = disc_dc
                .year_fraction(
                    disc.base_date(),
                    self.float.start,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let t_end = disc_dc
                .year_fraction(
                    disc.base_date(),
                    self.float.end,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);

            let df_start_abs = disc.df(t_start);
            let df_end_abs = disc.df(t_end);
            let df_start = if df_as_of != 0.0 {
                df_start_abs / df_as_of
            } else {
                1.0
            };
            let df_end = if df_as_of != 0.0 {
                df_end_abs / df_as_of
            } else {
                1.0
            };
            let mut pv = self.notional.amount() * (df_start - df_end);

            // Add spread contribution if any: N × sum_i( spread × alpha_i × DF(T_i) )
            if self.float.spread_bp != 0.0 {
                // Build coupon schedule using the float leg payment frequency and conventions
                let builder =
                    finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
                        .frequency(self.float.freq)
                        .stub_rule(self.float.stub);
                let sched_dates: Vec<_> = {
                    let sched = if let Some(id) = &self.float.calendar_id {
                        if let Some(cal) = calendar_by_id(id) {
                            builder.adjust_with(self.float.bdc, cal).build()?
                        } else {
                            builder.build()?
                        }
                    } else {
                        builder.build()?
                    };
                    sched.into_iter().collect()
                };

                if sched_dates.len() >= 2 {
                    let mut prev = sched_dates[0];
                    let mut annuity = 0.0;
                    for &d in &sched_dates[1..] {
                        // Only include future cashflows
                        if d <= as_of {
                            prev = d;
                            continue;
                        }

                        let alpha = self
                            .float
                            .dc
                            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                            .unwrap_or(0.0);
                        let t_d = disc_dc
                            .year_fraction(
                                disc.base_date(),
                                d,
                                finstack_core::dates::DayCountCtx::default(),
                            )
                            .unwrap_or(0.0);
                        let df_d_abs = disc.df(t_d);
                        let df = if df_as_of != 0.0 {
                            df_d_abs / df_as_of
                        } else {
                            1.0
                        };
                        annuity += alpha * df;
                        prev = d;
                    }
                    pv += self.notional.amount() * (self.float.spread_bp * 1e-4) * annuity;
                }
            }
            Money::new(pv, self.notional.currency())
        } else {
            // Non-OIS swap: requires forward curve for float leg pricing
            match context.get_forward_ref(self.float.forward_curve_id.as_ref()) {
                Ok(fwd) => self.pv_float_leg(disc, fwd, as_of)?,
                Err(_) => {
                    // Forward curve missing: return error to guide callers
                    return Err(context
                        .get_forward_ref(self.float.forward_curve_id.as_ref())
                        .err()
                        .unwrap_or(finstack_core::error::InputError::Invalid.into()));
                }
            }
        };

        let npv = match self.side {
            PayReceive::PayFixed => (pv_float - pv_fixed)?,
            PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
        };
        Ok(npv)
    }
}

// Explicit trait implementations for modern instrument style
// Attributable implementation is provided by the impl_instrument! macro

impl crate::instruments::common::traits::Instrument for InterestRateSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::IRS
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

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Use builder to generate both legs; then map signs by side
        let mut fixed_b = CashFlowSchedule::builder();
        fixed_b
            .principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id.clone(),
                stub: self.fixed.stub,
            });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = CashFlowSchedule::builder();
        float_b
            .principal(self.notional, self.float.start, self.float.end)
            .floating_cf(crate::cashflow::builder::FloatingCouponSpec {
                rate_spec: crate::cashflow::builder::FloatingRateSpec {
                    index_id: self.float.forward_curve_id.to_owned(),
                    spread_bp: self.float.spread_bp,
                    gearing: 1.0,
                    floor_bp: None,
                    cap_bp: None,
                    reset_freq: self.float.freq,
                    reset_lag_days: self.float.reset_lag_days,
                    dc: self.float.dc,
                    bdc: self.float.bdc,
                    calendar_id: self.float.calendar_id.clone(),
                },
                coupon_type: CouponType::Cash,
                freq: self.float.freq,
                stub: self.float.stub,
            });
        let float_sched = float_b.build()?;

        let mut flows: Vec<(Date, Money)> = Vec::new();
        for cf in fixed_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount,
                    PayReceive::PayFixed => cf.amount * -1.0,
                };
                flows.push((cf.date, amt));
            }
        }
        for cf in float_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount * -1.0,
                    PayReceive::PayFixed => cf.amount,
                };
                flows.push((cf.date, amt));
            }
        }
        Ok(flows)
    }

    /// Build full cashflow schedule with CFKind metadata for precise classification.
    ///
    /// This creates a proper CashFlowSchedule with CFKind information for each leg,
    /// enabling precise classification of fixed vs floating rate payments.
    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        use crate::cashflow::builder::{CashFlowSchedule, FloatingCouponSpec, Notional};
        use finstack_core::cashflow::primitives::{CFKind, CashFlow};

        // Build both legs using the builder to get proper CFKind classification
        let mut fixed_b = CashFlowSchedule::builder();
        fixed_b
            .principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id.clone(),
                stub: self.fixed.stub,
            });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = CashFlowSchedule::builder();
        float_b
            .principal(self.notional, self.float.start, self.float.end)
            .floating_cf(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: self.float.forward_curve_id.to_owned(),
                    spread_bp: self.float.spread_bp,
                    gearing: 1.0,
                    floor_bp: None,
                    cap_bp: None,
                    reset_freq: self.float.freq,
                    reset_lag_days: self.float.reset_lag_days,
                    dc: self.float.dc,
                    bdc: self.float.bdc,
                    calendar_id: self.float.calendar_id.clone(),
                },
                coupon_type: CouponType::Cash,
                freq: self.float.freq,
                stub: self.float.stub,
            });
        let float_sched = float_b.build()?;

        // Combine flows from both legs with proper CFKind classification
        let mut all_flows: Vec<CashFlow> = Vec::new();

        // Add fixed leg flows
        for cf in fixed_sched.flows {
            if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount,
                    PayReceive::PayFixed => cf.amount * -1.0,
                };
                all_flows.push(CashFlow {
                    date: cf.date,
                    reset_date: cf.reset_date,
                    amount: amt,
                    kind: cf.kind, // Preserve precise CFKind
                    accrual_factor: cf.accrual_factor,
                    rate: cf.rate,
                });
            }
        }

        // Add floating leg flows
        for cf in float_sched.flows {
            if cf.kind == CFKind::FloatReset {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount * -1.0,
                    PayReceive::PayFixed => cf.amount,
                };
                all_flows.push(CashFlow {
                    date: cf.date,
                    reset_date: cf.reset_date,
                    amount: amt,
                    kind: cf.kind, // Preserve precise CFKind
                    accrual_factor: cf.accrual_factor,
                    rate: cf.rate,
                });
            }
        }

        // Sort flows by date and CFKind priority
        all_flows.sort_by(|a, b| {
            use core::cmp::Ordering;
            match a.date.cmp(&b.date) {
                Ordering::Equal => {
                    // Use kind ranking logic from cashflow builder
                    let rank_a = match a.kind {
                        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                        CFKind::Fee => 1,
                        CFKind::Amortization => 2,
                        CFKind::PIK => 3,
                        CFKind::Notional => 4,
                        _ => 5,
                    };
                    let rank_b = match b.kind {
                        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                        CFKind::Fee => 1,
                        CFKind::Amortization => 2,
                        CFKind::PIK => 3,
                        CFKind::Notional => 4,
                        _ => 5,
                    };
                    rank_a.cmp(&rank_b)
                }
                other => other,
            }
        });

        // Create notional spec for swap (notional doesn't amortize)
        let notional = Notional::par(self.notional.amount(), self.notional.currency());

        Ok(crate::cashflow::builder::CashFlowSchedule {
            flows: all_flows,
            notional,
            day_count: self.fixed.dc, // Use fixed leg day count as representative
            meta: Default::default(),
        })
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InterestRateSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.fixed.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for InterestRateSwap {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.float.forward_curve_id.clone()]
    }
}
