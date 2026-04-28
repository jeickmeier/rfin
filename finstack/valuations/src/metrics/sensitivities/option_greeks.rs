//! Generic option greeks metric adapters.
//!
//! These calculators eliminate the per-instrument explosion of:
//! `metrics/{delta,gamma,vega,rho,theta,vanna,volga}.rs`
//! by delegating to the consolidated [`OptionGreeksProvider`] trait.

use std::marker::PhantomData;

use crate::instruments::common_impl::traits::{
    Instrument, OptionGreekKind, OptionGreeks, OptionGreeksProvider, OptionGreeksRequest,
};
use crate::metrics::{metric_not_found, MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

fn requested_value<I>(
    context: &mut MetricContext,
    request: OptionGreeksRequest,
    metric_id: MetricId,
    extract: impl FnOnce(OptionGreeks) -> Option<f64>,
) -> Result<f64>
where
    I: Instrument + OptionGreeksProvider + 'static,
{
    if let Some(value) = context.computed.get(&metric_id) {
        return Ok(*value);
    }

    let inst: &I = context.instrument_as()?;
    let greeks = inst.option_greeks(&context.curves, context.as_of, &request)?;
    store_available_greeks(context, greeks);
    extract(greeks).ok_or_else(|| metric_not_found(metric_id))
}

fn store_available_greeks(context: &mut MetricContext, greeks: OptionGreeks) {
    for (metric, value) in [
        (MetricId::Delta, greeks.delta),
        (MetricId::Gamma, greeks.gamma),
        (MetricId::Vega, greeks.vega),
        (MetricId::Theta, greeks.theta),
        (MetricId::Rho, greeks.rho_bp),
        (MetricId::ForeignRho, greeks.foreign_rho_bp),
        (MetricId::Vanna, greeks.vanna),
        (MetricId::Volga, greeks.volga),
    ] {
        if let Some(value) = value {
            context.computed.entry(metric).or_insert(value);
        }
    }
}

macro_rules! option_greek_calculator {
    ($name:ident, $kind:expr, $metric:expr, $base_pv:expr, $extract:expr) => {
        pub(crate) struct $name<I> {
            _phantom: PhantomData<I>,
        }

        impl<I> Default for $name<I> {
            fn default() -> Self {
                Self {
                    _phantom: PhantomData,
                }
            }
        }

        impl<I> MetricCalculator for $name<I>
        where
            I: Instrument + OptionGreeksProvider + 'static,
        {
            fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
                let base_pv = $base_pv(&*context);
                requested_value::<I>(
                    context,
                    OptionGreeksRequest {
                        greek: $kind,
                        base_pv,
                    },
                    $metric,
                    $extract,
                )
            }
        }
    };
}

option_greek_calculator!(
    OptionDeltaCalculator,
    OptionGreekKind::Delta,
    MetricId::Delta,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.delta
);
option_greek_calculator!(
    OptionGammaCalculator,
    OptionGreekKind::Gamma,
    MetricId::Gamma,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.gamma
);
option_greek_calculator!(
    OptionVegaCalculator,
    OptionGreekKind::Vega,
    MetricId::Vega,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.vega
);
option_greek_calculator!(
    OptionThetaCalculator,
    OptionGreekKind::Theta,
    MetricId::Theta,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.theta
);
option_greek_calculator!(
    OptionRhoCalculator,
    OptionGreekKind::Rho,
    MetricId::Rho,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.rho_bp
);
option_greek_calculator!(
    OptionForeignRhoCalculator,
    OptionGreekKind::ForeignRho,
    MetricId::ForeignRho,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.foreign_rho_bp
);
option_greek_calculator!(
    OptionVannaCalculator,
    OptionGreekKind::Vanna,
    MetricId::Vanna,
    |_: &MetricContext| None,
    |greeks: OptionGreeks| greeks.vanna
);
option_greek_calculator!(
    OptionVolgaCalculator,
    OptionGreekKind::Volga,
    MetricId::Volga,
    |context: &MetricContext| Some(context.base_value.amount()),
    |greeks: OptionGreeks| greeks.volga
);
