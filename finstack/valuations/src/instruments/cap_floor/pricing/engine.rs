//! Interest rate option pricer (Black model) for caps/floors/caplets/floorlets.
//!
//! This pricer mirrors the structure used by `cds::pricing::engine` and
//! centralizes pricing logic away from the instrument struct to keep
//! public APIs stable and enable reuse by metrics.

use crate::instruments::cap_floor::{InterestRateOption, RateOptionType};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

/// Pricing engine for interest rate options (Black model)
#[derive(Clone, Debug, Default)]
pub struct IrOptionPricer;

impl IrOptionPricer {
    /// Create a new interest rate option pricer
    pub fn new() -> Self {
        Self
    }

    /// Price an `InterestRateOption` to present value using market curves.
    pub fn price(
        &self,
        s: &InterestRateOption,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use crate::cashflow::builder::schedule_utils::build_dates;
        use crate::instruments::cap_floor::pricing::black as black_ir;

        // Get market curves
        let disc_curve = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.disc_id.as_ref(),
            )?;
        let fwd_curve = curves
            .get_ref::<finstack_core::market_data::term_structures::forward_curve::ForwardCurve>(
                s.forward_id.as_ref(),
            )?;
        let vol_surface = if s.pricing_overrides.implied_volatility.is_none() {
            Some(curves.surface_ref(s.vol_id)?)
        } else {
            None
        };

        let mut total_pv = Money::new(0.0, s.notional.currency());

        // Single caplet/floorlet
        if matches!(
            s.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Floorlet
        ) {
            let t_fix = s.day_count.year_fraction(
                as_of,
                s.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let t_pay = s.day_count.year_fraction(
                as_of,
                s.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let tau = s.day_count.year_fraction(
                s.start_date,
                s.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            let forward = fwd_curve.rate_period(t_fix.max(0.0), t_pay);
            let df = disc_curve.df(t_pay);
            let sigma = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
                impl_vol
            } else if let Some(vol_surf) = &vol_surface {
                vol_surf.value_clamped(t_fix.max(0.0), s.strike_rate)
            } else {
                return Err(finstack_core::error::InputError::NotFound {
                    id: "cap_floor_vol_surface".to_string(),
                }
                .into());
            };

            let is_cap = matches!(
                s.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            return black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                is_cap,
                notional: s.notional.amount(),
                strike: s.strike_rate,
                forward,
                discount_factor: df,
                volatility: sigma,
                time_to_fixing: t_fix,
                accrual_year_fraction: tau,
                currency: s.notional.currency(),
            });
        }

        // Cap/floor portfolio of caplets/floorlets
        let schedule = build_dates(
            s.start_date,
            s.end_date,
            s.frequency,
            s.stub_kind,
            s.bdc,
            s.calendar_id,
        );

        if schedule.dates.len() < 2 {
            return Ok(total_pv);
        }

        let is_cap = matches!(
            s.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Cap
        );
        let mut prev = schedule.dates[0];
        for &pay in &schedule.dates[1..] {
            let t_fix = s.day_count.year_fraction(
                as_of,
                prev,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let t_pay = s.day_count.year_fraction(
                as_of,
                pay,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let tau = s.day_count.year_fraction(
                prev,
                pay,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            if t_fix > 0.0 {
                let forward = fwd_curve.rate_period(t_fix, t_pay);
                let df = disc_curve.df(t_pay);
                let sigma = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
                    impl_vol
                } else if let Some(vol_surf) = &vol_surface {
                    vol_surf.value_clamped(t_fix, s.strike_rate)
                } else {
                    return Err(finstack_core::error::InputError::NotFound {
                        id: "cap_floor_vol_surface".to_string(),
                    }
                    .into());
                };

                let leg_pv = black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                    is_cap,
                    notional: s.notional.amount(),
                    strike: s.strike_rate,
                    forward,
                    discount_factor: df,
                    volatility: sigma,
                    time_to_fixing: t_fix,
                    accrual_year_fraction: tau,
                    currency: s.notional.currency(),
                })?;
                total_pv = (total_pv + leg_pv)?;
            }
            prev = pay;
        }

        Ok(total_pv)
    }
}
