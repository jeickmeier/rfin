use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};

/// Calculates modified duration for bonds.
///
/// Modified duration measures interest rate sensitivity and is computed as:
/// ```text
/// D_mod = D_mac / (1 + y/m)
/// ```
/// where `D_mac` is Macaulay duration, `y` is yield to maturity, and `m` is
/// the number of compounding periods per year.
///
/// # Dependencies
///
/// Requires `DurationMac` and `Ytm` metrics to be computed first.
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
/// // Modified duration is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMac]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Ytm".to_string(),
                })
            })?;

        let d_mac = context
            .computed
            .get(&MetricId::DurationMac)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:DurationMac".to_string(),
                })
            })?;

        // Modified duration depends on compounding; default to Street (periodic with bond freq)
        let m = crate::instruments::bond::pricing::quote_engine::periods_per_year(
            bond.cashflow_spec.frequency(),
        )?
        .max(1.0);
        Ok(d_mac / (1.0 + ytm / m))
    }
}
