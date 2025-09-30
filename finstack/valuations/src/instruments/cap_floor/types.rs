//! Interest rate option instrument types and Black model greeks.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, SettlementType};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::InterestRateOptionParams;

/// Type of interest rate option
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateOptionType {
    /// Cap (series of caplets)
    Cap,
    /// Floor (series of floorlets)
    Floor,
    /// Caplet (single period cap)
    Caplet,
    /// Floorlet (single period floor)
    Floorlet,
}

/// Interest rate option instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
pub struct InterestRateOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate (as decimal, e.g., 0.05 for 5%)
    pub strike_rate: f64,
    /// Start date of underlying period
    pub start_date: Date,
    /// End date of underlying period
    pub end_date: Date,
    /// Payment frequency for caps/floors
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Schedule stub convention
    pub stub_kind: StubKind,
    /// Schedule business day convention
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier for schedule and roll conventions
    pub calendar_id: Option<&'static str>,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub disc_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InterestRateOption {
    /// Create a new interest rate option using parameter structs
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &InterestRateOptionParams,
        start_date: Date,
        end_date: Date,
        disc_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            rate_option_type: option_params.rate_option_type,
            notional: option_params.notional,
            strike_rate: option_params.strike_rate,
            start_date,
            end_date,
            frequency: option_params.frequency,
            day_count: option_params.day_count,
            stub_kind: option_params.stub_kind,
            bdc: option_params.bdc,
            calendar_id: option_params.calendar_id,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            disc_id: disc_id.into(),
            forward_id: forward_id.into(),
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a cap instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_cap(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_id: &'static str,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::cap(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            disc_id.into(),
            forward_id.into(),
            vol_id,
        )
    }

    /// Create a floor instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_floor(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        start_date: Date,
        end_date: Date,
        frequency: Frequency,
        day_count: DayCount,
        disc_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_id: &'static str,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::floor(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            disc_id.into(),
            forward_id.into(),
            vol_id,
        )
    }
}

impl_instrument!(
    InterestRateOption,
    crate::pricer::InstrumentType::CapFloor,
    "InterestRateOption",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
    }
);

impl InterestRateOption {
    /// Calculate the net present value of this interest rate option
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::cashflow::builder::schedule_utils::build_dates;
        use crate::instruments::cap_floor::pricing::black as black_ir;

        // Get market curves
        let disc_curve = curves.get_discount_ref(self.disc_id.as_ref())?;
        let fwd_curve = curves.get_forward_ref(self.forward_id.as_ref())?;
        let vol_surface = if self.pricing_overrides.implied_volatility.is_none() {
            Some(curves.surface_ref(self.vol_id)?)
        } else {
            None
        };

        let mut total_pv = Money::new(0.0, self.notional.currency());

        // Single caplet/floorlet
        if matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Floorlet
        ) {
            let t_fix = self.day_count.year_fraction(
                as_of,
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let t_pay = self.day_count.year_fraction(
                as_of,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let tau = self.day_count.year_fraction(
                self.start_date,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            let forward = fwd_curve.rate_period(t_fix.max(0.0), t_pay);
            let df = disc_curve.df(t_pay);
            let sigma = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
                impl_vol
            } else if let Some(vol_surf) = &vol_surface {
                vol_surf.value_clamped(t_fix.max(0.0), self.strike_rate)
            } else {
                return Err(finstack_core::error::InputError::NotFound {
                    id: "cap_floor_vol_surface".to_string(),
                }
                .into());
            };

            let is_cap = matches!(
                self.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            return black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                is_cap,
                notional: self.notional.amount(),
                strike: self.strike_rate,
                forward,
                discount_factor: df,
                volatility: sigma,
                time_to_fixing: t_fix,
                accrual_year_fraction: tau,
                currency: self.notional.currency(),
            });
        }

        // Cap/floor portfolio of caplets/floorlets
        let schedule = build_dates(
            self.start_date,
            self.end_date,
            self.frequency,
            self.stub_kind,
            self.bdc,
            self.calendar_id,
        );

        if schedule.dates.len() < 2 {
            return Ok(total_pv);
        }

        let is_cap = matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Cap
        );
        let mut prev = schedule.dates[0];
        for &pay in &schedule.dates[1..] {
            let t_fix = self.day_count.year_fraction(
                as_of,
                prev,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let t_pay = self.day_count.year_fraction(
                as_of,
                pay,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let tau = self.day_count.year_fraction(
                prev,
                pay,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            if t_fix > 0.0 {
                let forward = fwd_curve.rate_period(t_fix, t_pay);
                let df = disc_curve.df(t_pay);
                let sigma = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
                    impl_vol
                } else if let Some(vol_surf) = &vol_surface {
                    vol_surf.value_clamped(t_fix, self.strike_rate)
                } else {
                    return Err(finstack_core::error::InputError::NotFound {
                        id: "cap_floor_vol_surface".to_string(),
                    }
                    .into());
                };

                let leg_pv = black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                    is_cap,
                    notional: self.notional.amount(),
                    strike: self.strike_rate,
                    forward,
                    discount_factor: df,
                    volatility: sigma,
                    time_to_fixing: t_fix,
                    accrual_year_fraction: tau,
                    currency: self.notional.currency(),
                })?;
                total_pv = (total_pv + leg_pv)?;
            }
            prev = pay;
        }

        Ok(total_pv)
    }
}

impl crate::instruments::common::HasDiscountCurve for InterestRateOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}
