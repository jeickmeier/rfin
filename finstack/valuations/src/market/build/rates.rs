//! Builders for interest rate instruments from market quotes.

use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::{FutureContractSpecs, InterestRateFuture, Position};
use crate::instruments::irs::{InterestRateSwap, IrsLegConventions};
use crate::market::build::context::BuildCtx;
use crate::market::conventions::defs::{RateIndexConventions, RateIndexKind};
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::ids::Pillar;
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::{adjust, CalendarRegistry, Date, DateExt, HolidayCalendar, TenorUnit};
use finstack_core::error::Error;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Build an instrument from a RateQuote.
pub fn build_rate_instrument(quote: &RateQuote, ctx: &BuildCtx) -> Result<Box<dyn Instrument>> {
    let registry = ConventionRegistry::global();

    match quote {
        RateQuote::Deposit {
            id,
            index,
            pillar,
            rate,
        } => {
            let conv = registry.require_rate_index(index)?;
            // Important: store the instrument's `start` as the trade/as_of date and let the
            // instrument apply spot lag consistently via `spot_lag_days_opt`.
            //
            // If we set `start` to the already spot-adjusted date AND set `spot_lag_days_opt`,
            // the instrument will apply spot lag twice when generating/pricing cashflows.
            let start = ctx.as_of;
            let spot_start = resolve_spot_date(ctx.as_of, conv)?;

            // Resolve calendar for tenor addition
            let cal_registry = CalendarRegistry::global();
            let cal = resolve_calendar(cal_registry, &conv.market_calendar_id)?;

            let end = match pillar {
                Pillar::Tenor(t) => {
                    // Maturity is SPOT + tenor adjusted by BDC/Calendar.
                    t.add_to_date(spot_start, Some(cal), conv.market_business_day_convention)?
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal)?,
            };

            let discount_id = ctx
                .curve_id("discount")
                .cloned()
                .unwrap_or_else(|| conv.currency.to_string());

            // Use currency string representation
            let currency = conv.currency;

            let deposit = Deposit::builder()
                .id(InstrumentId::new(id.as_str()))
                .notional(Money::new(ctx.notional, currency))
                .start(start)
                .end(end)
                .day_count(conv.day_count)
                .quote_rate_opt(Some(*rate))
                .discount_curve_id(CurveId::new(discount_id))
                // Optional fields
                .spot_lag_days_opt(Some(conv.market_settlement_days))
                .bdc_opt(Some(conv.market_business_day_convention))
                .calendar_id_opt(Some(conv.market_calendar_id.clone()))
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(deposit))
        }
        RateQuote::Fra {
            id,
            index,
            start: start_pillar,
            end: end_pillar,
            rate,
        } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(ctx.as_of, conv)?;

            let cal_registry = CalendarRegistry::global();
            let cal = resolve_calendar(cal_registry, &conv.market_calendar_id)?;

            // Resolve start/end dates from SPOT
            let start_date = match start_pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(spot, Some(cal), conv.market_business_day_convention)?
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal)?,
            };
            let end_date = match end_pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(spot, Some(cal), conv.market_business_day_convention)?
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal)?,
            };

            // Fixings are determined by reset lag from start date
            let reset_lag = conv.default_reset_lag_days;
            let fixing_date = resolve_fixing_date(start_date, conv)?;

            let discount_id = ctx
                .curve_id("discount")
                .cloned()
                .unwrap_or_else(|| conv.currency.to_string());
            let forward_id = ctx
                .curve_id("forward")
                .cloned()
                .unwrap_or_else(|| conv.currency.to_string());

            let fra = ForwardRateAgreement::builder()
                .id(InstrumentId::new(id.as_str()))
                .notional(Money::new(ctx.notional, conv.currency))
                .fixing_date(fixing_date)
                .start_date(start_date)
                .end_date(end_date)
                .fixed_rate(*rate)
                .day_count(conv.day_count)
                .reset_lag(reset_lag)
                .discount_curve_id(CurveId::new(discount_id))
                .forward_id(CurveId::new(forward_id))
                .pay_fixed(true)
                .fixing_calendar_id_opt(Some(conv.market_calendar_id.clone()))
                .fixing_bdc_opt(Some(conv.market_business_day_convention))
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(fra))
        }
        RateQuote::Futures {
            id,
            contract,
            expiry,
            price,
            convexity_adjustment,
        } => {
            let fut_conv = registry.require_ir_future(contract)?;
            let idx_conv = registry.require_rate_index(&fut_conv.index_id)?;

            let cal_registry = CalendarRegistry::global();
            let cal = resolve_calendar(cal_registry, &fut_conv.calendar_id)?;
            let bdc = idx_conv.market_business_day_convention;

            let expiry_date = adjust(*expiry, bdc, cal)?;
            let period_start_unadj = expiry_date.add_business_days(fut_conv.settlement_days, cal)?;
            let period_start = adjust(period_start_unadj, bdc, cal)?;

            let delivery_tenor = finstack_core::dates::Tenor::new(
                fut_conv.delivery_months as u32,
                TenorUnit::Months,
            );
            let period_end =
                delivery_tenor.add_to_date(period_start, Some(cal), bdc)?;

            let fixing_date = resolve_fixing_date(period_start, idx_conv)?;

            let discount_id = ctx
                .curve_id("discount")
                .cloned()
                .unwrap_or_else(|| idx_conv.currency.to_string());
            let forward_id = ctx
                .curve_id("forward")
                .cloned()
                .unwrap_or_else(|| idx_conv.currency.to_string());

            let contract_specs = FutureContractSpecs {
                face_value: fut_conv.face_value,
                tick_size: fut_conv.tick_size,
                tick_value: fut_conv.tick_value,
                delivery_months: fut_conv.delivery_months,
                convexity_adjustment: (*convexity_adjustment).or(fut_conv.convexity_adjustment),
            };

            let future = InterestRateFuture::builder()
                .id(InstrumentId::new(id.as_str()))
                .notional(Money::new(ctx.notional, idx_conv.currency))
                .expiry_date(expiry_date)
                .fixing_date(fixing_date)
                .period_start(period_start)
                .period_end(period_end)
                .quoted_price(*price)
                .day_count(idx_conv.day_count)
                .position(Position::Long)
                .contract_specs(contract_specs)
                .discount_curve_id(CurveId::new(discount_id))
                .forward_id(CurveId::new(forward_id))
                .volatility_id_opt(None)
                .attributes(Default::default())
                .build()?;

            Ok(Box::new(future))
        }
        RateQuote::Swap {
            id,
            index,
            pillar,
            rate,
            spread,
        } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(ctx.as_of, conv)?;

            let cal_registry = CalendarRegistry::global();
            let cal = resolve_calendar(cal_registry, &conv.market_calendar_id)?;

            // Swap start is spot
            let start = spot;
            let maturity = match pillar {
                Pillar::Tenor(t) => {
                    t.add_to_date(start, Some(cal), conv.market_business_day_convention)?
                }
                Pillar::Date(d) => adjust(*d, conv.market_business_day_convention, cal)?,
            };

            let discount_id = ctx
                .curve_id("discount")
                .cloned()
                .unwrap_or_else(|| conv.currency.to_string());
            let forward_id = ctx
                .curve_id("forward")
                .cloned()
                .unwrap_or_else(|| conv.currency.to_string());

            use crate::instruments::common::parameters::legs::PayReceive;

            // Map conventions
            let leg_conv = IrsLegConventions {
                fixed_freq: conv.default_fixed_leg_frequency,
                float_freq: conv.default_payment_frequency,
                fixed_dc: conv.default_fixed_leg_day_count,
                float_dc: conv.day_count,
                bdc: conv.market_business_day_convention,
                payment_calendar_id: Some(conv.market_calendar_id.clone()),
                fixing_calendar_id: Some(conv.market_calendar_id.clone()),
                stub: finstack_core::dates::StubKind::None, // Default
                reset_lag_days: conv.default_reset_lag_days,
                payment_delay_days: conv.default_payment_delay_days,
            };

            // Choose constructor based on index kind
            let mut swap = match conv.kind {
                RateIndexKind::Term => InterestRateSwap::create_term_swap_with_conventions(
                    InstrumentId::new(id.as_str()),
                    Money::new(ctx.notional, conv.currency),
                    *rate,
                    start,
                    maturity,
                    PayReceive::PayFixed,
                    CurveId::new(discount_id),
                    CurveId::new(forward_id),
                    leg_conv,
                )?,
                RateIndexKind::OvernightRfr => {
                    let compounding = conv.ois_compounding.clone().unwrap_or(
                        crate::instruments::irs::FloatingLegCompounding::CompoundedInArrears {
                            lookback_days: 0,
                            observation_shift: None,
                        },
                    );

                    InterestRateSwap::create_ois_swap_with_conventions(
                        InstrumentId::new(id.as_str()),
                        Money::new(ctx.notional, conv.currency),
                        *rate,
                        start,
                        maturity,
                        PayReceive::PayFixed,
                        CurveId::new(discount_id),
                        CurveId::new(forward_id),
                        compounding,
                        leg_conv,
                    )?
                }
            };

            // Apply spread if present
            if let Some(s) = spread {
                swap.float.spread_bp = *s * 10000.0; // spread is decimal (e.g. 0.0010), spread_bp is bps
            }

            Ok(Box::new(swap))
        }
    }
}

// Helpers

fn resolve_calendar<'a>(
    registry: &'a CalendarRegistry,
    id: &str,
) -> Result<&'a dyn HolidayCalendar> {
    registry
        .resolve_str(id)
        .ok_or_else(|| Error::calendar_not_found_with_suggestions(id, &[]))
}

fn resolve_spot_date(as_of: Date, conv: &RateIndexConventions) -> Result<Date> {
    let cal = CalendarRegistry::global().resolve_str(&conv.market_calendar_id);
    if let Some(c) = cal {
        let spot = as_of.add_business_days(conv.market_settlement_days, c)?;
        adjust(spot, conv.market_business_day_convention, c)
    } else {
        Err(Error::calendar_not_found_with_suggestions(
            &conv.market_calendar_id,
            &[],
        ))
    }
}

// Kept for other date adjustments if needed, but not used directly for Tenor anymore
#[allow(dead_code)]
fn adjust_date(date: Date, conv: &RateIndexConventions) -> Result<Date> {
    let cal = CalendarRegistry::global().resolve_str(&conv.market_calendar_id);
    if let Some(c) = cal {
        adjust(date, conv.market_business_day_convention, c)
    } else {
        Err(Error::calendar_not_found_with_suggestions(
            &conv.market_calendar_id,
            &[],
        ))
    }
}

fn resolve_fixing_date(start: Date, conv: &RateIndexConventions) -> Result<Date> {
    let cal = CalendarRegistry::global().resolve_str(&conv.market_calendar_id);
    let lag = conv.default_reset_lag_days;

    if let Some(c) = cal {
        start.add_business_days(-lag, c)
    } else {
        Err(Error::calendar_not_found_with_suggestions(
            &conv.market_calendar_id,
            &[],
        ))
    }
}
