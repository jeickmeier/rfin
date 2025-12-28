use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCount, DayCountCtx, StubKind, Tenor};

/// Configuration for I-Spread fixed-leg conventions.
///
/// Controls the proxy swap fixed leg used to derive the par rate that is
/// subtracted from the bond's YTM.
#[derive(Clone, Debug)]
pub struct ISpreadConfig {
    /// Day-count convention for the proxy fixed leg used in the par rate.
    pub fixed_leg_day_count: DayCount,
    /// Payment frequency for the proxy fixed leg.
    pub fixed_leg_frequency: Tenor,
}

impl Default for ISpreadConfig {
    fn default() -> Self {
        Self {
            // Preserve previous behaviour: annual Act/Act proxy fixed leg.
            fixed_leg_day_count: DayCount::ActAct,
            fixed_leg_frequency: Tenor::annual(),
        }
    }
}

/// I-Spread: bond YTM minus interpolated swap par rate at same maturity.
///
/// Uses the bond's discount curve to approximate a par swap fixed leg with
/// configurable day-count and frequency (defaults to annual Act/Act).
///
/// The I-spread is computed as:
/// ```text
/// I-Spread = YTM - par_swap_rate
/// ```
/// where `par_swap_rate` is derived from the discount curve using a proxy
/// fixed-leg schedule.
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
/// // I-spread is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct ISpreadCalculator {
    config: ISpreadConfig,
}

impl ISpreadCalculator {
    /// Create an I-Spread calculator with default (annual Act/Act) fixed-leg
    /// conventions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an I-Spread calculator with explicit fixed-leg conventions.
    pub fn with_config(config: ISpreadConfig) -> Self {
        Self { config }
    }
}

impl MetricCalculator for ISpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Bond YTM from dependencies
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Ytm".to_string(),
                })
            })?;

        // Use the bond's discount curve as proxy for swap discounting (OIS collateral)
        let disc = context.curves.get_discount_ref(&bond.discount_curve_id)?;

        // Build proxy fixed-leg schedule using configured frequency and standard
        // business-day / stub rules. This approximates a plain-vanilla par swap
        // fixed leg at the bond maturity.
        let dates: Vec<Date> =
            finstack_core::dates::ScheduleBuilder::new(context.as_of, bond.maturity)
                .frequency(self.config.fixed_leg_frequency)
                .stub_rule(StubKind::ShortFront)
                .build()?
                .into_iter()
                .collect();

        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Par rate approx: (P(0,T0) - P(0,Tn)) / Sum alpha_i P(0,Ti)
        let p0 = disc.df_on_date_curve(dates[0])?;
        // Safe: we checked dates.len() >= 2 above
        let pn = disc.df_on_date_curve(dates[dates.len() - 1])?;
        let num = p0 - pn;
        let mut den = 0.0;
        for w in dates.windows(2) {
            let (a, b) = (w[0], w[1]);
            // Use configured fixed-leg day-count for accrual factors.
            let alpha =
                self.config
                    .fixed_leg_day_count
                    .year_fraction(a, b, DayCountCtx::default())?;
            let p = disc.df_on_date_curve(b)?;
            den += alpha * p;
        }
        if den == 0.0 {
            return Ok(0.0);
        }
        let par_swap_rate = num / den;

        Ok(ytm - par_swap_rate)
    }
}
