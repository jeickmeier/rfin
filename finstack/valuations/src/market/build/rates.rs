//! Builders for interest rate instruments from market quotes.

use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::{FutureContractSpecs, InterestRateFuture, Position};
use crate::instruments::irs::{InterestRateSwap, IrsLegConventions};
use crate::market::build::context::BuildCtx;
use crate::market::build::helpers::{resolve_calendar, resolve_spot_date};
use crate::market::conventions::defs::{RateIndexConventions, RateIndexKind};
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::ids::Pillar;
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::{adjust, Date, DateExt, TenorUnit};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Build an interest rate instrument from a [`RateQuote`].
///
/// This function resolves conventions, calculates accrual dates, and constructs a concrete
/// instrument instance based on the quote type. Supported quote types include deposits, FRAs,
/// interest rate futures, and swaps (both term and overnight).
///
/// # Arguments
///
/// * `quote` - The market quote containing rate/price and pillar information
/// * `ctx` - Build context with valuation date, notional, and curve mappings
///
/// # Returns
///
/// `Ok(Box<dyn Instrument>)` with the constructed instrument, or `Err` if:
/// - Convention lookup fails (missing index or future contract)
/// - Calendar resolution fails
/// - Date calculations fail (invalid tenor, business day adjustment)
/// - Instrument construction fails (invalid parameters)
///
/// # Examples
///
/// Building a deposit:
/// ```rust
/// use finstack_valuations::market::build::context::BuildCtx;
/// use finstack_valuations::market::build::rates::build_rate_instrument;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::conventions::ids::IndexId;
/// use finstack_core::dates::Date;
/// use std::collections::HashMap;
///
/// # fn example() -> finstack_core::Result<()> {
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     1_000_000.0,
///     HashMap::new(),
/// );
///
/// let quote = RateQuote::Deposit {
///     id: QuoteId::new("USD-SOFR-DEP-1M"),
///     index: IndexId::new("USD-SOFR-1M"),
///     pillar: Pillar::Tenor("1M".parse().unwrap()),
///     rate: 0.0525,
/// };
///
/// let instrument = build_rate_instrument(&quote, &ctx)?;
/// # Ok(())
/// # }
/// ```
///
/// Building a swap:
/// ```rust
/// use finstack_valuations::market::build::context::BuildCtx;
/// use finstack_valuations::market::build::rates::build_rate_instrument;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::conventions::ids::IndexId;
/// use finstack_core::dates::Date;
/// use std::collections::HashMap;
///
/// # fn example() -> finstack_core::Result<()> {
/// let ctx = BuildCtx::new(
///     Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
///     1_000_000.0,
///     HashMap::new(),
/// );
///
/// let quote = RateQuote::Swap {
///     id: QuoteId::new("USD-OIS-SWAP-5Y"),
///     index: IndexId::new("USD-SOFR-OIS"),
///     pillar: Pillar::Tenor("5Y".parse().unwrap()),
///     rate: 0.0450,
///     spread_decimal: None,
/// };
///
/// let instrument = build_rate_instrument(&quote, &ctx)?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`RateQuote`](crate::market::quotes::rates::RateQuote) for supported quote types
/// - [`BuildCtx`](crate::market::build::context::BuildCtx) for build context configuration
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
            let spot_start = resolve_spot_date(
                ctx.as_of,
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;
            // Resolve concrete accrual start/end dates here so the built instrument
            // remains fixed even if as_of changes later.
            let start = spot_start;

            // Resolve calendar for tenor addition
            let cal = resolve_calendar(&conv.market_calendar_id)?;

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
            let spot = resolve_spot_date(
                ctx.as_of,
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;

            let cal = resolve_calendar(&conv.market_calendar_id)?;

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

            let cal = resolve_calendar(&fut_conv.calendar_id)?;
            let bdc = idx_conv.market_business_day_convention;

            let expiry_date = adjust(*expiry, bdc, cal)?;
            let period_start_unadj =
                expiry_date.add_business_days(fut_conv.settlement_days, cal)?;
            let period_start = adjust(period_start_unadj, bdc, cal)?;

            let delivery_tenor = finstack_core::dates::Tenor::new(
                fut_conv.delivery_months as u32,
                TenorUnit::Months,
            );
            let period_end = delivery_tenor.add_to_date(period_start, Some(cal), bdc)?;

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
            spread_decimal,
        } => {
            let conv = registry.require_rate_index(index)?;
            let spot = resolve_spot_date(
                ctx.as_of,
                &conv.market_calendar_id,
                conv.market_settlement_days,
                conv.market_business_day_convention,
            )?;

            let cal = resolve_calendar(&conv.market_calendar_id)?;

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
            // spread_decimal is in decimal format (e.g., 0.0010 for 10bp)
            // Convert to basis points by multiplying by 10000
            if let Some(spread_decimal) = spread_decimal {
                swap.float.spread_bp = *spread_decimal * 10000.0;
            }

            Ok(Box::new(swap))
        }
    }
}

// Helpers

// Kept for other date adjustments if needed, but not used directly for Tenor anymore
#[allow(dead_code)]
fn adjust_date(date: Date, conv: &RateIndexConventions) -> Result<Date> {
    let cal = resolve_calendar(&conv.market_calendar_id)?;
    adjust(date, conv.market_business_day_convention, cal)
}

fn resolve_fixing_date(start: Date, conv: &RateIndexConventions) -> Result<Date> {
    let cal = resolve_calendar(&conv.market_calendar_id)?;
    let lag = conv.default_reset_lag_days;

    start.add_business_days(-lag, cal)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::market::conventions::ids::IndexId;
    use crate::market::quotes::ids::{Pillar, QuoteId};
    use std::collections::HashMap;

    /// Test that spread_decimal is correctly converted to basis points
    #[test]
    fn test_swap_spread_decimal_conversion() -> Result<()> {
        let ctx = BuildCtx::new(
            Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
            1_000_000.0,
            HashMap::new(),
        );

        // Create a swap quote with spread_decimal = 0.0010 (10bp)
        let quote = RateQuote::Swap {
            id: QuoteId::new("USD-SOFR-OIS-SWAP-5Y"),
            index: IndexId::new("USD-SOFR-OIS"),
            pillar: Pillar::Tenor(finstack_core::dates::Tenor::new(
                5,
                finstack_core::dates::TenorUnit::Years,
            )),
            rate: 0.0450,
            spread_decimal: Some(0.0010), // 10bp in decimal
        };

        let instrument = build_rate_instrument(&quote, &ctx)?;

        // Downcast to InterestRateSwap to access spread_bp
        use crate::instruments::irs::InterestRateSwap;
        let swap = instrument
            .as_any()
            .downcast_ref::<InterestRateSwap>()
            .expect("Expected InterestRateSwap");

        // Verify spread_decimal (0.0010) was converted to spread_bp (10.0)
        assert_eq!(
            swap.float.spread_bp, 10.0,
            "Expected spread_decimal of 0.0010 to convert to 10.0 basis points"
        );

        Ok(())
    }

    /// Test that swap with no spread works correctly
    #[test]
    fn test_swap_no_spread() -> Result<()> {
        let ctx = BuildCtx::new(
            Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
            1_000_000.0,
            HashMap::new(),
        );

        let quote = RateQuote::Swap {
            id: QuoteId::new("USD-SOFR-OIS-SWAP-5Y"),
            index: IndexId::new("USD-SOFR-OIS"),
            pillar: Pillar::Tenor(finstack_core::dates::Tenor::new(
                5,
                finstack_core::dates::TenorUnit::Years,
            )),
            rate: 0.0450,
            spread_decimal: None,
        };

        let instrument = build_rate_instrument(&quote, &ctx)?;

        // Should build successfully
        use crate::instruments::irs::InterestRateSwap;
        let swap = instrument
            .as_any()
            .downcast_ref::<InterestRateSwap>()
            .expect("Expected InterestRateSwap");

        // Default spread_bp should be 0.0
        assert_eq!(
            swap.float.spread_bp, 0.0,
            "Expected default spread_bp to be 0.0"
        );

        Ok(())
    }

    /// Test spread conversion with various values
    #[test]
    fn test_swap_spread_various_values() -> Result<()> {
        let test_cases = vec![
            (0.0001, 1.0),    // 1bp
            (0.0010, 10.0),   // 10bp
            (0.0050, 50.0),   // 50bp
            (0.0100, 100.0),  // 100bp (1%)
            (-0.0010, -10.0), // -10bp (negative spread)
        ];

        for (spread_decimal, expected_bp) in test_cases {
            let ctx = BuildCtx::new(
                Date::from_calendar_date(2024, time::Month::January, 2).unwrap(),
                1_000_000.0,
                HashMap::new(),
            );

            let quote = RateQuote::Swap {
                id: QuoteId::new("USD-SOFR-OIS-SWAP-5Y"),
                index: IndexId::new("USD-SOFR-OIS"),
                pillar: Pillar::Tenor(finstack_core::dates::Tenor::new(
                    5,
                    finstack_core::dates::TenorUnit::Years,
                )),
                rate: 0.0450,
                spread_decimal: Some(spread_decimal),
            };

            let instrument = build_rate_instrument(&quote, &ctx)?;

            use crate::instruments::irs::InterestRateSwap;
            let swap = instrument
                .as_any()
                .downcast_ref::<InterestRateSwap>()
                .expect("Expected InterestRateSwap");

            assert_eq!(
                swap.float.spread_bp, expected_bp,
                "spread_decimal {} should convert to {} basis points",
                spread_decimal, expected_bp
            );
        }

        Ok(())
    }
}
