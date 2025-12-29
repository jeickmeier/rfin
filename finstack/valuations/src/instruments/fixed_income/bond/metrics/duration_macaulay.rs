use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Calculates Macaulay duration for bonds.
///
/// Macaulay duration is the weighted average time to receive cashflows, where
/// weights are the present values of each cashflow:
/// ```text
/// D_mac = Σ (t_i * PV(CF_i)) / Price
/// ```
///
/// # Dependencies
///
/// Requires `Ytm` metric to be computed first.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // Macaulay duration is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct MacaulayDurationCalculator;

impl MetricCalculator for MacaulayDurationCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "metric:Ytm".to_string(),
                })
            })?;

        // YTM dependency ensures cashflows are already built and cached
        let flows: &Vec<(Date, Money)> = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Calculate price from flows to ensure consistency
        let price = {
            let bond: &Bond = context.instrument_as()?;
            crate::instruments::bond::pricing::quote_engine::price_from_ytm(
                bond,
                flows,
                context.as_of,
                ytm,
            )?
        };
        if price == 0.0 {
            return Ok(0.0);
        }

        // Calculate Macaulay duration
        let mut weighted_time = 0.0;

        {
            let bond: &Bond = context.instrument_as()?;
            for &(date, amount) in flows {
                if date <= context.as_of {
                    continue;
                }
                let t = bond
                    .cashflow_spec
                    .day_count()
                    .year_fraction(
                        context.as_of,
                        date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?
                    .max(0.0);
                let df = crate::instruments::bond::pricing::quote_engine::df_from_yield(
                    ytm,
                    t,
                    crate::instruments::bond::pricing::quote_engine::YieldCompounding::Street,
                    bond.cashflow_spec.frequency(),
                )?;
                weighted_time += t * amount.amount() * df;
            }
        }

        Ok(weighted_time / price)
    }
}
