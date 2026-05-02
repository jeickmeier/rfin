//! Vega calculator for interest rate options (caps/floors/caplets/floorlets).

use crate::calibration::hull_white::HullWhiteParams;
use crate::instruments::rates::cap_floor::{CapFloor, CapFloorVolType};
use crate::metrics::{MetricCalculator, MetricContext};
use crate::pricer::ModelKey;
use finstack_core::dates::{Date, DayCountContext};
use finstack_core::Result;

const DEFAULT_HW_VEGA_BUMP: f64 = 0.0001;

/// Vega calculator (model-consistent vega per 1% vol, aggregated for caps/floors).
///
/// Dispatches to the appropriate model based on `vol_type`:
/// - `Lognormal`: Black-76 vega = F·n(d₁)·√T / 100
/// - `ShiftedLognormal`: Black-76 vega on shifted rates
/// - `Normal`: Bachelier vega = n(d)·√T / 100
///
/// For Normal vol, the 1% bump is in absolute rate terms (e.g., 1bp normal vol).
pub(crate) struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CapFloor = context.instrument_as()?;
        if matches!(
            context.clone_pricer_dispatch().0,
            Some(ModelKey::HullWhite1F)
        ) {
            return hull_white_tree_vega_per_pct(option, context);
        }
        let strike = option.strike_f64()?;
        let vol_type = option.vol_type;
        let vol_shift = option.resolved_vol_shift();
        aggregate_vega(option, context, strike, vol_type, vol_shift)
    }
}

fn aggregate_vega(
    option: &CapFloor,
    context: &MetricContext,
    strike: f64,
    vol_type: CapFloorVolType,
    vol_shift: f64,
) -> Result<f64> {
    let disc_curve = context
        .curves
        .get_discount(option.discount_curve_id.as_ref())?;
    let fwd_curve = context
        .curves
        .get_forward(option.forward_curve_id.as_ref())?;
    let vol_surface = context.curves.get_surface(option.vol_surface_id.as_str())?;
    let dc_ctx = DayCountContext::default();
    let use_rfr_observation_time = option.uses_overnight_rfr_index();

    let mut total = 0.0;
    for period in option.pricing_periods()? {
        let fixing_date = option.option_fixing_date(&period);
        if fixing_date < context.as_of {
            continue;
        }

        let price_t_fix = option
            .day_count
            .year_fraction(context.as_of, fixing_date, dc_ctx)?
            .max(1e-6);
        let risk_t_fix = if use_rfr_observation_time {
            rfr_observation_midpoint_time(option, context.as_of, &period, dc_ctx)?
        } else {
            price_t_fix
        };

        let forward = crate::instruments::common_impl::pricing::time::rate_period_on_dates(
            fwd_curve.as_ref(),
            period.accrual_start,
            period.accrual_end,
        )?;
        let df = crate::instruments::common_impl::pricing::time::relative_df_discount_curve(
            disc_curve.as_ref(),
            context.as_of,
            period.payment_date,
        )?;
        let sigma = vol_surface.value_clamped(price_t_fix, strike);
        let per_unit = match vol_type {
            CapFloorVolType::Lognormal => {
                crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                    strike, forward, sigma, risk_t_fix,
                )
            }
            CapFloorVolType::ShiftedLognormal => {
                crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                    strike + vol_shift,
                    forward + vol_shift,
                    sigma,
                    risk_t_fix,
                )
            }
            CapFloorVolType::Normal => {
                crate::instruments::rates::cap_floor::pricing::normal::vega_per_pct(
                    strike, forward, sigma, risk_t_fix,
                )
            }
            CapFloorVolType::Auto => {
                if forward > 0.0 && strike > 0.0 {
                    crate::instruments::rates::cap_floor::pricing::black::vega_per_pct(
                        strike, forward, sigma, risk_t_fix,
                    )
                } else {
                    crate::instruments::rates::cap_floor::pricing::normal::vega_per_pct(
                        strike, forward, sigma, risk_t_fix,
                    )
                }
            }
        };
        total += per_unit * option.notional.amount() * period.accrual_year_fraction * df;
    }
    Ok(total)
}

fn rfr_observation_midpoint_time(
    option: &CapFloor,
    as_of: Date,
    period: &crate::cashflow::builder::periods::SchedulePeriod,
    dc_ctx: DayCountContext,
) -> Result<f64> {
    let observation_start = if period.accrual_start > as_of {
        period.accrual_start
    } else {
        as_of
    };
    let t_start = option
        .day_count
        .year_fraction(as_of, observation_start, dc_ctx)?
        .max(0.0);
    let t_end = option
        .day_count
        .year_fraction(as_of, period.accrual_end, dc_ctx)?
        .max(0.0);
    Ok(((t_start + t_end) * 0.5).max(1e-6))
}

fn hull_white_tree_vega_per_pct(option: &CapFloor, context: &MetricContext) -> Result<f64> {
    let base_vol = option
        .pricing_overrides
        .model_config
        .tree_volatility
        .unwrap_or_else(|| HullWhiteParams::default().sigma);
    if base_vol <= DEFAULT_HW_VEGA_BUMP {
        return Ok(0.0);
    }

    let bump = DEFAULT_HW_VEGA_BUMP;
    let mut up = option.clone();
    up.pricing_overrides.model_config.tree_volatility = Some(base_vol + bump);
    let pv_up = context.reprice_instrument_raw(&up, context.curves.as_ref(), context.as_of)?;

    let mut down = option.clone();
    down.pricing_overrides.model_config.tree_volatility = Some(base_vol - bump);
    let pv_down = context.reprice_instrument_raw(&down, context.curves.as_ref(), context.as_of)?;

    Ok((pv_up - pv_down) / (2.0 * bump) * 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::periods::SchedulePeriod;
    use crate::instruments::rates::cap_floor::RateOptionType;
    use crate::instruments::{ExerciseStyle, SettlementType};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
    use finstack_core::money::Money;
    use rust_decimal::Decimal;
    use time::macros::date;

    #[test]
    fn rfr_vega_time_uses_actual_observation_window_midpoint() {
        let option = CapFloor {
            id: "RFR-VEGA-TIME".into(),
            rate_option_type: RateOptionType::Caplet,
            notional: Money::new(1_000_000.0, Currency::USD),
            strike: Decimal::try_from(0.05).expect("valid decimal"),
            start_date: date!(2024 - 01 - 03),
            maturity: date!(2024 - 04 - 03),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            stub: StubKind::None,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-OIS".into(),
            vol_surface_id: "USD-CAP-VOL".into(),
            vol_type: CapFloorVolType::Lognormal,
            vol_shift: 0.0,
            pricing_overrides: crate::instruments::PricingOverrides::default(),
            attributes: Default::default(),
        };
        let period = SchedulePeriod {
            accrual_start: date!(2024 - 01 - 03),
            accrual_end: date!(2024 - 04 - 03),
            payment_date: date!(2024 - 04 - 05),
            reset_date: None,
            accrual_year_fraction: 91.0 / 360.0,
        };

        let actual = rfr_observation_midpoint_time(
            &option,
            date!(2024 - 01 - 03),
            &period,
            DayCountContext::default(),
        )
        .expect("rfr timing");

        assert!((actual - (91.0 / 360.0) * 0.5).abs() < 1e-12);
    }
}
