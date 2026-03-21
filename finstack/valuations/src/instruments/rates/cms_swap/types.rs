//! CMS swap instrument definition.
//!
//! A CMS (Constant Maturity Swap) swap has one leg paying a CMS rate
//! (the par swap rate for a reference tenor, e.g., 10Y) and the other
//! leg paying a fixed or floating rate.
//!
//! The CMS rate requires a convexity adjustment because the forward
//! swap rate is a martingale under the annuity measure, not the payment
//! measure. The adjustment depends on volatility and the rate level.
//!
//! # Reference
//!
//! Hagan, P. S. (2003). "Convexity Conundrums: Pricing CMS Swaps, Caps,
//! and Floors." *Wilmott Magazine*, March, 38-44.

use crate::impl_instrument_base;
use crate::instruments::common_impl::parameters::IRSConvention;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// CMS (Constant Maturity Swap) swap instrument.
///
/// One leg pays a CMS rate (the par swap rate for a reference tenor, e.g., 10Y)
/// observed on each fixing date, and the other leg pays a fixed or floating rate.
///
/// The CMS rate requires a convexity adjustment because the CMS rate is not a
/// martingale under the payment measure. The adjustment depends on the correlation
/// between the CMS rate and the numeraire (annuity).
///
/// # Reference
///
/// Hagan, P. S. (2003). "Convexity Conundrums: Pricing CMS Swaps, Caps, and Floors."
/// *Wilmott Magazine*, March, 38-44.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct CmsSwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional amount.
    pub notional: Money,
    /// Pay direction: `Pay` means pay CMS leg, receive funding leg.
    pub side: crate::instruments::common_impl::parameters::legs::PayReceive,

    // ── CMS Leg ──────────────────────────────────────────────────────────
    /// CMS tenor in years (e.g., 10.0 for 10Y swap rate).
    pub cms_tenor: f64,
    /// Fixing dates for CMS rate observations.
    pub cms_fixing_dates: Vec<Date>,
    /// Payment dates for the CMS leg.
    pub cms_payment_dates: Vec<Date>,
    /// Accrual fractions for each CMS period.
    pub cms_accrual_fractions: Vec<f64>,
    /// Day count convention for CMS leg accrual.
    pub cms_day_count: DayCount,
    /// Spread over the CMS rate (decimal, e.g., 0.001 = 10bp).
    #[serde(default)]
    #[builder(default)]
    pub cms_spread: f64,
    /// Optional cap on the CMS rate (decimal).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cms_cap: Option<f64>,
    /// Optional floor on the CMS rate (decimal).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cms_floor: Option<f64>,

    // ── Underlying Swap Conventions ──────────────────────────────────────
    /// IRS convention for the underlying swap (e.g., `USDStandard`).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_convention: Option<IRSConvention>,
    /// Fixed leg frequency of the underlying swap (overrides convention).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_fixed_freq: Option<Tenor>,
    /// Floating leg frequency of the underlying swap (overrides convention).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_float_freq: Option<Tenor>,
    /// Day count of the underlying swap fixed leg (overrides convention).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_day_count: Option<DayCount>,
    /// Day count of the underlying swap floating leg (overrides convention).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_float_day_count: Option<DayCount>,

    // ── Funding Leg ──────────────────────────────────────────────────────
    /// Funding leg definition (fixed or floating).
    pub funding_leg: FundingLeg,

    // ── Market References ────────────────────────────────────────────────
    /// Discount curve ID for present value calculations.
    pub discount_curve_id: CurveId,
    /// Forward/projection curve ID for CMS rate projection.
    pub forward_curve_id: CurveId,
    /// Volatility surface ID for CMS convexity adjustment.
    pub vol_surface_id: CurveId,

    /// Pricing overrides (manual price, yield, spread).
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping.
    #[serde(default)]
    #[builder(default)]
    pub attributes: Attributes,
}

/// Funding leg specification for a CMS swap.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum FundingLeg {
    /// Fixed rate funding leg.
    Fixed {
        /// Fixed coupon rate (decimal, e.g., 0.03 = 3%).
        rate: f64,
        /// Payment dates for each period.
        payment_dates: Vec<Date>,
        /// Accrual fractions for each period.
        accrual_fractions: Vec<f64>,
        /// Day count convention.
        day_count: DayCount,
    },
    /// Floating rate funding leg.
    Floating {
        /// Spread over the floating index (decimal, e.g., 0.001 = 10bp).
        spread: f64,
        /// Payment dates for each period.
        payment_dates: Vec<Date>,
        /// Accrual fractions for each period.
        accrual_fractions: Vec<f64>,
        /// Day count convention.
        day_count: DayCount,
        /// Forward curve for floating rate projection.
        forward_curve_id: CurveId,
    },
}

impl CmsSwap {
    /// Resolved fixed leg frequency (explicit > convention > default semi-annual).
    pub fn resolved_swap_fixed_freq(&self) -> Tenor {
        self.swap_fixed_freq
            .or_else(|| self.swap_convention.map(|c| c.fixed_frequency()))
            .unwrap_or_else(Tenor::semi_annual)
    }

    /// Resolved float leg frequency (explicit > convention > default quarterly).
    pub fn resolved_swap_float_freq(&self) -> Tenor {
        self.swap_float_freq
            .or_else(|| self.swap_convention.map(|c| c.float_frequency()))
            .unwrap_or_else(Tenor::quarterly)
    }

    /// Resolved fixed leg day count (explicit > convention > default 30/360).
    pub fn resolved_swap_day_count(&self) -> DayCount {
        self.swap_day_count
            .or_else(|| self.swap_convention.map(|c| c.fixed_day_count()))
            .unwrap_or(DayCount::Thirty360)
    }

    /// Resolved float leg day count (explicit > convention > swap day count).
    pub fn resolved_swap_float_day_count(&self) -> DayCount {
        self.swap_float_day_count
            .or_else(|| self.swap_convention.map(|c| c.float_day_count()))
            .unwrap_or_else(|| self.resolved_swap_day_count())
    }

    /// Create a CMS swap from schedule parameters.
    ///
    /// Generates fixing/payment dates for both legs from start, end, and
    /// frequency. Uses standard market conventions (Modified Following BDC,
    /// weekends-only calendar).
    #[allow(clippy::too_many_arguments)]
    pub fn from_schedule(
        id: impl Into<InstrumentId>,
        start_date: Date,
        maturity: Date,
        cms_frequency: Tenor,
        cms_tenor: f64,
        cms_spread: f64,
        funding_leg: FundingLegSpec,
        notional: Money,
        cms_day_count: DayCount,
        swap_convention: IRSConvention,
        side: crate::instruments::common_impl::parameters::legs::PayReceive,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        use crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID;
        use crate::cashflow::builder::periods::{build_periods, BuildPeriodsParams};
        use finstack_core::dates::{BusinessDayConvention, StubKind};

        let cms_periods = build_periods(BuildPeriodsParams {
            start: start_date,
            end: maturity,
            frequency: cms_frequency,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: WEEKENDS_ONLY_ID,
            end_of_month: false,
            day_count: cms_day_count,
            payment_lag_days: 0,
            reset_lag_days: None,
        })?;

        if cms_periods.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }

        let cms_fixing_dates: Vec<Date> = cms_periods.iter().map(|p| p.accrual_start).collect();
        let cms_payment_dates: Vec<Date> = cms_periods.iter().map(|p| p.payment_date).collect();
        let cms_accrual_fractions: Vec<f64> = cms_periods
            .iter()
            .map(|p| p.accrual_year_fraction)
            .collect();

        let funding_leg = match funding_leg {
            FundingLegSpec::Fixed { rate, day_count } => {
                let fund_periods = build_periods(BuildPeriodsParams {
                    start: start_date,
                    end: maturity,
                    frequency: cms_frequency,
                    stub: StubKind::ShortFront,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: WEEKENDS_ONLY_ID,
                    end_of_month: false,
                    day_count,
                    payment_lag_days: 0,
                    reset_lag_days: None,
                })?;
                FundingLeg::Fixed {
                    rate,
                    payment_dates: fund_periods.iter().map(|p| p.payment_date).collect(),
                    accrual_fractions: fund_periods
                        .iter()
                        .map(|p| p.accrual_year_fraction)
                        .collect(),
                    day_count,
                }
            }
            FundingLegSpec::Floating {
                spread,
                day_count,
                forward_curve_id,
            } => {
                let fund_periods = build_periods(BuildPeriodsParams {
                    start: start_date,
                    end: maturity,
                    frequency: cms_frequency,
                    stub: StubKind::ShortFront,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: WEEKENDS_ONLY_ID,
                    end_of_month: false,
                    day_count,
                    payment_lag_days: 0,
                    reset_lag_days: None,
                })?;
                FundingLeg::Floating {
                    spread,
                    payment_dates: fund_periods.iter().map(|p| p.payment_date).collect(),
                    accrual_fractions: fund_periods
                        .iter()
                        .map(|p| p.accrual_year_fraction)
                        .collect(),
                    day_count,
                    forward_curve_id,
                }
            }
        };

        CmsSwap::builder()
            .id(id.into())
            .notional(notional)
            .side(side)
            .cms_tenor(cms_tenor)
            .cms_fixing_dates(cms_fixing_dates)
            .cms_payment_dates(cms_payment_dates)
            .cms_accrual_fractions(cms_accrual_fractions)
            .cms_day_count(cms_day_count)
            .cms_spread(cms_spread)
            .swap_convention_opt(Some(swap_convention))
            .funding_leg(funding_leg)
            .discount_curve_id(discount_curve_id.into())
            .forward_curve_id(forward_curve_id.into())
            .vol_surface_id(vol_surface_id.into())
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))
    }

    /// Create a canonical example CMS swap (pay 10Y CMS, receive fixed).
    #[allow(clippy::expect_used)]
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;

        let fixing_dates = vec![
            Date::from_calendar_date(2025, Month::March, 20).expect("valid"),
            Date::from_calendar_date(2025, Month::June, 20).expect("valid"),
            Date::from_calendar_date(2025, Month::September, 22).expect("valid"),
            Date::from_calendar_date(2025, Month::December, 22).expect("valid"),
        ];
        let payment_dates = vec![
            Date::from_calendar_date(2025, Month::June, 20).expect("valid"),
            Date::from_calendar_date(2025, Month::September, 22).expect("valid"),
            Date::from_calendar_date(2025, Month::December, 22).expect("valid"),
            Date::from_calendar_date(2026, Month::March, 20).expect("valid"),
        ];
        let accrual_fractions = vec![0.25, 0.25, 0.25, 0.25];

        CmsSwap::builder()
            .id(InstrumentId::new("CMSSWAP-10Y-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(crate::instruments::common_impl::parameters::legs::PayReceive::Pay)
            .cms_tenor(10.0)
            .cms_fixing_dates(fixing_dates)
            .cms_payment_dates(payment_dates.clone())
            .cms_accrual_fractions(accrual_fractions.clone())
            .cms_day_count(DayCount::Act365F)
            .cms_spread(0.0)
            .swap_convention_opt(Some(IRSConvention::USDStandard))
            .swap_float_day_count_opt(Some(DayCount::Act360))
            .funding_leg(FundingLeg::Fixed {
                rate: 0.03,
                payment_dates,
                accrual_fractions,
                day_count: DayCount::Thirty360,
            })
            .discount_curve_id(CurveId::new("USD-OIS"))
            .forward_curve_id(CurveId::new("USD-LIBOR-3M"))
            .vol_surface_id(CurveId::new("USD-CMS10Y-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example CmsSwap construction should not fail")
    }
}

/// Simplified funding leg specification for [`CmsSwap::from_schedule`].
pub enum FundingLegSpec {
    /// Fixed rate funding leg.
    Fixed {
        /// Fixed coupon rate (decimal).
        rate: f64,
        /// Day count convention.
        day_count: DayCount,
    },
    /// Floating rate funding leg.
    Floating {
        /// Spread over the floating index (decimal).
        spread: f64,
        /// Day count convention.
        day_count: DayCount,
        /// Forward curve for floating rate projection.
        forward_curve_id: CurveId,
    },
}

impl crate::instruments::common_impl::traits::Instrument for CmsSwap {
    impl_instrument_base!(crate::pricer::InstrumentType::CmsSwap);

    fn default_model(&self) -> crate::pricer::ModelKey {
        crate::pricer::ModelKey::Black76
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        if self.cms_fixing_dates.len() != self.cms_payment_dates.len()
            || self.cms_fixing_dates.len() != self.cms_accrual_fractions.len()
        {
            return Err(finstack_core::Error::Validation(format!(
                "CMS swap vectors must have equal length: fixing_dates={}, payment_dates={}, accrual_fractions={}",
                self.cms_fixing_dates.len(),
                self.cms_payment_dates.len(),
                self.cms_accrual_fractions.len(),
            )));
        }
        crate::instruments::rates::cms_swap::pricer::compute_pv(self, market, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
        self.cms_fixing_dates.first().copied()
    }

    fn pricing_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn pricing_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for CmsSwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        let mut builder = crate::instruments::common_impl::traits::InstrumentCurves::builder();
        builder = builder.discount(self.discount_curve_id.clone());
        builder = builder.forward(self.forward_curve_id.clone());
        if let FundingLeg::Floating {
            forward_curve_id, ..
        } = &self.funding_leg
        {
            builder = builder.forward(forward_curve_id.clone());
        }
        builder.build()
    }
}
