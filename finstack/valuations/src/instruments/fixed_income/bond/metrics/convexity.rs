use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Calculates convexity for bonds.
///
/// Convexity measures the curvature of the price/yield relationship and is
/// computed using the closed-form second derivative of price with respect to yield:
/// ```text
/// Convexity = (1 / P) * d²P/dy²
/// ```
/// where `P` is the yield-implied price and `y` uses the bond's yield compounding
/// convention (street/periodic by default).
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
/// // Convexity is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
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

        let bond: &Bond = context.instrument_as()?;
        let comp = crate::instruments::bond::pricing::quote_engine::YieldCompounding::Street;
        let freq = bond.cashflow_spec.frequency();

        let price = crate::instruments::bond::pricing::quote_engine::price_from_ytm(
            bond,
            flows,
            context.as_of,
            ytm,
        )?;
        if price == 0.0 {
            return Ok(0.0);
        }

        let mut d2_price = 0.0;
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
            let df_second = df_second_derivative(ytm, t, comp, freq)?;
            d2_price += amount.amount() * df_second;
        }

        Ok(d2_price / price)
    }
}

fn df_second_derivative(
    ytm: f64,
    t: f64,
    comp: crate::instruments::bond::pricing::quote_engine::YieldCompounding,
    freq: finstack_core::dates::Tenor,
) -> finstack_core::Result<f64> {
    use crate::instruments::bond::pricing::quote_engine::{
        df_from_yield, periods_per_year, YieldCompounding,
    };

    if t <= 0.0 {
        return Ok(0.0);
    }

    let df = df_from_yield(ytm, t, comp, freq)?;
    Ok(match comp {
        YieldCompounding::Simple => {
            let denom = 1.0 + ytm * t;
            2.0 * t * t / (denom * denom * denom)
        }
        YieldCompounding::Annual => {
            let denom = 1.0 + ytm;
            t * (t + 1.0) / (denom * denom) * df
        }
        YieldCompounding::Periodic(m) => {
            let m = m as f64;
            let c = m * t;
            let denom = m + ytm;
            c * (c + 1.0) / (denom * denom) * df
        }
        YieldCompounding::Continuous => t * t * df,
        YieldCompounding::Street => {
            let m = periods_per_year(freq)?.max(1.0);
            let c = m * t;
            let denom = m + ytm;
            c * (c + 1.0) / (denom * denom) * df
        }
        YieldCompounding::TreasuryActual => {
            let m = periods_per_year(freq)?.max(1.0);
            let period_length = 1.0 / m;

            if t <= period_length {
                let denom = 1.0 + ytm * t;
                2.0 * t * t / (denom * denom * denom)
            } else {
                let n_full = (t * m).floor();
                let stub_time = t - n_full / m;
                if stub_time <= 1e-10 {
                    let c = m * t;
                    let denom = m + ytm;
                    c * (c + 1.0) / (denom * denom) * df
                } else {
                    let df_stub = 1.0 / (1.0 + ytm * stub_time);
                    let df_periodic = (1.0 + ytm / m).powf(-n_full);

                    let df_stub_prime = -stub_time / (1.0 + ytm * stub_time).powi(2);
                    let df_stub_second =
                        2.0 * stub_time * stub_time / (1.0 + ytm * stub_time).powi(3);

                    let denom = m + ytm;
                    let df_periodic_prime = -(n_full / denom) * df_periodic;
                    let df_periodic_second =
                        n_full * (n_full + 1.0) / (denom * denom) * df_periodic;

                    df_stub_second * df_periodic
                        + 2.0 * df_stub_prime * df_periodic_prime
                        + df_stub * df_periodic_second
                }
            }
        }
    })
}
