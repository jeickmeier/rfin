//! Inflation cap/floor instrument and pricing logic.
//!
//! Prices YoY inflation caps/floors using Black-76 (lognormal) or
//! Bachelier (normal) on the forward YoY inflation rate.

use crate::instruments::cap_floor::pricing::black as black_ir;
use crate::instruments::common::models::volatility::normal::bachelier_price;
use crate::instruments::common::parameters::OptionType;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::pricer::ModelKey;
use finstack_core::dates::{
    BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, StubKind, Tenor,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};
use time::Duration;

/// Inflation option type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InflationCapFloorType {
    /// Cap (portfolio of caplets).
    Cap,
    /// Floor (portfolio of floorlets).
    Floor,
    /// Single-period caplet.
    Caplet,
    /// Single-period floorlet.
    Floorlet,
}

impl InflationCapFloorType {
    fn is_cap(self) -> bool {
        matches!(
            self,
            InflationCapFloorType::Cap | InflationCapFloorType::Caplet
        )
    }

    fn option_type(self) -> OptionType {
        if self.is_cap() {
            OptionType::Call
        } else {
            OptionType::Put
        }
    }
}

impl std::fmt::Display for InflationCapFloorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InflationCapFloorType::Cap => write!(f, "cap"),
            InflationCapFloorType::Floor => write!(f, "floor"),
            InflationCapFloorType::Caplet => write!(f, "caplet"),
            InflationCapFloorType::Floorlet => write!(f, "floorlet"),
        }
    }
}

impl std::str::FromStr for InflationCapFloorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cap" => Ok(InflationCapFloorType::Cap),
            "floor" => Ok(InflationCapFloorType::Floor),
            "caplet" => Ok(InflationCapFloorType::Caplet),
            "floorlet" => Ok(InflationCapFloorType::Floorlet),
            other => Err(format!("Unknown inflation option type: {}", other)),
        }
    }
}

/// YoY inflation cap/floor instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InflationCapFloor {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Cap/floor type (cap, floor, caplet, floorlet).
    pub option_type: InflationCapFloorType,
    /// Notional amount in quote currency.
    pub notional: Money,
    /// Strike rate (annualized, decimal).
    pub strike_rate: f64,
    /// Start date of the first inflation period.
    pub start_date: Date,
    /// End date of the final inflation period.
    pub end_date: Date,
    /// Payment frequency (ignored for caplet/floorlet).
    pub frequency: Tenor,
    /// Day count convention for accrual and option time.
    pub day_count: DayCount,
    /// Schedule stub convention.
    pub stub_kind: StubKind,
    /// Business day convention for schedule and payments.
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier.
    #[builder(optional)]
    pub calendar_id: Option<String>,
    /// Inflation index/curve identifier (e.g., US-CPI-U).
    pub inflation_index_id: CurveId,
    /// Discount curve identifier.
    pub discount_curve_id: CurveId,
    /// Volatility surface identifier.
    pub vol_surface_id: CurveId,
    /// Pricing overrides (implied volatility, surface extrapolation).
    pub pricing_overrides: PricingOverrides,
    /// Optional contract-level lag override.
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl InflationCapFloor {
    /// Validate structural invariants.
    pub fn validate(&self) -> finstack_core::Result<()> {
        if self.start_date >= self.end_date {
            return Err(finstack_core::error::InputError::InvalidDateRange.into());
        }
        if self.notional.amount() <= 0.0 {
            return Err(finstack_core::error::InputError::NonPositiveValue.into());
        }
        if self.frequency.count == 0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        Ok(())
    }

    fn effective_lag(&self, curves: &MarketContext) -> InflationLag {
        if let Some(lag) = self.lag_override {
            return lag;
        }
        if let Some(index) = curves.inflation_index_ref(self.inflation_index_id.as_str()) {
            return index.lag();
        }
        InflationLag::None
    }

    fn apply_lag(date: Date, lag: InflationLag) -> Date {
        match lag {
            InflationLag::None => date,
            InflationLag::Months(m) => date.add_months(-(m as i32)),
            InflationLag::Days(d) => date - Duration::days(d as i64),
            _ => date,
        }
    }

    fn signed_year_fraction(start: Date, end: Date) -> f64 {
        if end >= start {
            DayCount::Act365F
                .year_fraction(start, end, DayCountCtx::default())
                .unwrap_or(0.0)
        } else {
            -DayCount::Act365F
                .year_fraction(end, start, DayCountCtx::default())
                .unwrap_or(0.0)
        }
    }

    fn adjusted_payment_date(&self, date: Date) -> Date {
        if let Some(ref cal_id) = self.calendar_id {
            use finstack_core::dates::calendar::registry::CalendarRegistry;
            if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                return finstack_core::dates::adjust(date, self.bdc, cal).unwrap_or(date);
            }
        }
        date
    }

    fn cpi_value(
        &self,
        curves: &MarketContext,
        as_of: Date,
        date: Date,
    ) -> finstack_core::Result<f64> {
        if let Some(index) = curves.inflation_index_ref(self.inflation_index_id.as_str()) {
            if let Ok(value) = index.value_on(date) {
                return Ok(value);
            }
        }

        let lag = self.effective_lag(curves);
        let lagged_date = Self::apply_lag(date, lag);
        let curve = curves.get_inflation_ref(self.inflation_index_id.as_str())?;
        let t = Self::signed_year_fraction(as_of, lagged_date);
        Ok(curve.cpi(t))
    }

    fn schedule(&self) -> finstack_core::Result<Vec<(Date, Date, Date)>> {
        if matches!(
            self.option_type,
            InflationCapFloorType::Caplet | InflationCapFloorType::Floorlet
        ) {
            let pay = self.adjusted_payment_date(self.end_date);
            return Ok(vec![(self.start_date, self.end_date, pay)]);
        }

        let schedule = crate::cashflow::builder::date_generation::build_dates_checked(
            self.start_date,
            self.end_date,
            self.frequency,
            self.stub_kind,
            self.bdc,
            self.calendar_id.as_deref(),
        )?;

        if schedule.dates.len() < 2 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        let mut periods = Vec::with_capacity(schedule.dates.len().saturating_sub(1));
        for window in schedule.dates.windows(2) {
            let start = window[0];
            let end = window[1];
            let pay = self.adjusted_payment_date(end);
            periods.push((start, end, pay));
        }
        Ok(periods)
    }

    /// Price using an explicit model key (Black-76 or Normal).
    pub fn npv_with_model(
        &self,
        curves: &MarketContext,
        as_of: Date,
        model: ModelKey,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_str())?;
        let vol_surface = if self.pricing_overrides.implied_volatility.is_none() {
            Some(curves.surface_ref(self.vol_surface_id.as_str())?)
        } else {
            None
        };

        let mut total_pv = Money::new(0.0, self.notional.currency());
        let lag = self.effective_lag(curves);

        for (start, end, pay) in self.schedule()? {
            if pay <= as_of {
                continue;
            }

            let accrual = self
                .day_count
                .year_fraction(start, end, DayCountCtx::default())?;
            if accrual <= 0.0 {
                continue;
            }

            let cpi_start = self.cpi_value(curves, as_of, start)?;
            let cpi_end = self.cpi_value(curves, as_of, end)?;
            if cpi_start <= 0.0 || cpi_end <= 0.0 {
                return Err(finstack_core::error::InputError::NonPositiveValue.into());
            }

            let forward_rate = (cpi_end / cpi_start - 1.0) / accrual;
            let fixing_date = Self::apply_lag(end, lag);
            let t_fix =
                self.day_count
                    .signed_year_fraction(as_of, fixing_date, DayCountCtx::default())?;

            let t_pay = disc
                .day_count()
                .year_fraction(as_of, pay, DayCountCtx::default())?;
            let df = disc.df(t_pay);

            let sigma = if t_fix > 0.0 {
                if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
                    impl_vol
                } else if let Some(vol) = &vol_surface {
                    vol.value_clamped(t_fix, self.strike_rate)
                } else {
                    return Err(finstack_core::error::InputError::NotFound {
                        id: "inflation_cap_floor_vol_surface".to_string(),
                    }
                    .into());
                }
            } else {
                0.0
            };

            let leg_pv = match model {
                ModelKey::Normal => {
                    let annuity = df * self.notional.amount() * accrual;
                    let premium = bachelier_price(
                        self.option_type.option_type(),
                        forward_rate,
                        self.strike_rate,
                        sigma,
                        t_fix,
                        annuity,
                    );
                    Money::new(premium, self.notional.currency())
                }
                _ => {
                    if t_fix > 0.0 && (forward_rate <= 0.0 || self.strike_rate <= 0.0) {
                        return Err(finstack_core::error::InputError::Invalid.into());
                    }
                    black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                        is_cap: self.option_type.is_cap(),
                        notional: self.notional.amount(),
                        strike: self.strike_rate,
                        forward: forward_rate,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: accrual,
                        currency: self.notional.currency(),
                    })?
                }
            };

            total_pv = (total_pv + leg_pv)?;
        }

        Ok(total_pv)
    }

    /// Default NPV using Black-76 (lognormal) model.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        self.npv_with_model(curves, as_of, ModelKey::Black76)
    }
}

impl InflationCapFloorBuilder {
    /// Set the strike rate using a typed rate.
    pub fn strike_rate_rate(mut self, rate: Rate) -> Self {
        self.strike_rate = Some(rate.as_decimal());
        self
    }
}

impl crate::instruments::common::traits::Instrument for InflationCapFloor {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::InflationCapFloor
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

    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InflationCapFloor {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for InflationCapFloor {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        vec![self.inflation_index_id.clone()]
    }
}

impl crate::instruments::common::traits::CurveDependencies for InflationCapFloor {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.inflation_index_id.clone())
            .build()
    }
}
