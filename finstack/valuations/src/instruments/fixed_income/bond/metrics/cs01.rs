//! Bond-specific CS01 calculators with z-spread fallback.
//!
//! When a bond has a credit (hazard) curve, CS01 is computed by bumping the
//! hazard curve par spreads (standard credit-model approach via
//! [`GenericParallelCs01`] / [`GenericBucketedCs01`]).
//!
//! When no credit curve is configured, CS01 is computed by bumping the bond's
//! z-spread by 1 bp and measuring the PV change:
//!
//! ```text
//! CS01 = -(PV(z + 1bp) - PV(z))
//! ```
//!
//! where `PV(z) = Σ CF_i · DF_i · exp(-z · t_i)`. This is the market-standard
//! approach for vanilla bonds without an explicit credit model.

use crate::cashflow::traits::CashflowProvider;
use crate::constants::ONE_BASIS_POINT;
use crate::instruments::common_impl::traits::{CurveDependencies, Instrument};
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::DayCountCtx;
use finstack_core::math::summation::NeumaierAccumulator;

/// Bond parallel CS01 with z-spread fallback.
///
/// Delegates to [`GenericParallelCs01`] when the bond references a credit
/// curve; otherwise computes CS01 by bumping the z-spread by 1 bp.
/// The result is keyed by credit curve ID or instrument ID.
pub struct BondCs01Calculator;

impl MetricCalculator for BondCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let curves = bond.curve_dependencies()?;

        if !curves.credit_curves.is_empty() {
            return crate::metrics::GenericParallelCs01::<Bond>::default().calculate(context);
        }

        let inst_id = bond.id();

        let base_spread = context
            .computed
            .get(&MetricId::ZSpread)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "metric:ZSpread".to_string(),
                })
            })?;

        let bumped_spread = base_spread + ONE_BASIS_POINT;

        let flows = bond.dated_cashflows(&context.curves, context.as_of)?;
        let disc = context.curves.get_discount(&bond.discount_curve_id)?;
        let dc = disc.day_count();
        let base_date = disc.base_date();

        let mut base_npv = NeumaierAccumulator::new();
        let mut bumped_npv = NeumaierAccumulator::new();

        for (date, amount) in &flows {
            if *date <= context.as_of {
                continue;
            }
            let t = dc.year_fraction(base_date, *date, DayCountCtx::default())?;
            let df = disc.df_on_date_curve(*date)?;
            let amt = amount.amount();

            base_npv.add(amt * df * (-base_spread * t).exp());
            bumped_npv.add(amt * df * (-bumped_spread * t).exp());
        }

        let cs01 = -(bumped_npv.total() - base_npv.total());

        context
            .computed
            .insert(MetricId::custom(format!("cs01::{}", inst_id)), cs01);

        Ok(cs01)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::ZSpread]
    }
}

/// Bond bucketed CS01 with z-spread fallback.
///
/// Delegates to [`GenericBucketedCs01`] when the bond references a credit
/// curve; otherwise returns the parallel z-spread CS01 keyed by instrument ID.
pub struct BondBucketedCs01Calculator;

impl MetricCalculator for BondBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let curves = bond.curve_dependencies()?;

        if !curves.credit_curves.is_empty() {
            return crate::metrics::GenericBucketedCs01::<Bond>::default().calculate(context);
        }

        let cs01 = context
            .computed
            .get(&MetricId::Cs01)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "metric:Cs01".to_string(),
                })
            })?;

        Ok(cs01)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Cs01]
    }
}
