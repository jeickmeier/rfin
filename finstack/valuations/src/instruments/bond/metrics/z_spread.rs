use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::F;

/// Calculates Z-Spread (zero-volatility spread) for fixed-rate bonds.
///
/// Market-standard definition: constant additive spread `z` to the base
/// discount curve such that the discounted value of future cashflows equals
/// the bond's dirty market price. We apply the spread as an exponential
/// shift on discount factors: `df_z(t) = df_base(t) * exp(-z * t)`.
///
/// Returns `z` in decimal units (e.g., 0.01 = 100 bps).
pub struct ZSpreadCalculator;

impl MetricCalculator for ZSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        // Need accrued to form dirty market price when using quoted clean price
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Determine dirty market value in currency
        let bond: &Bond = context.instrument_as()?;
        let target_value_ccy: F = if let Some(clean_px) = bond.pricing_overrides.quoted_clean_price {
            // Accrued from computed metrics (currency amount)
            let accrued_ccy = context
                .computed
                .get(&MetricId::Accrued)
                .copied()
                .ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "metric:Accrued".to_string(),
                    })
                })?;
            // Convert clean price (quote, pct of par) to currency and add accrued currency
            clean_px * bond.notional.amount() / 100.0 + accrued_ccy
        } else {
            // Fallback to base PV if no market quote
            context.base_value.amount()
        };

        // Objective: PV_z(z) - target_value_ccy = 0
        let curves = context.curves.as_ref().clone();
        let as_of = context.as_of;
        let objective = |z: F| -> F {
            match crate::instruments::bond::pricing::helpers::price_from_z_spread(
                bond,
                &curves,
                as_of,
                z,
            ) {
                Ok(pv) => pv - target_value_ccy,
                Err(_) => 1e12 * z.signum(),
            }
        };

        // Solve using Brent with a reasonable bracket
        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(0.5)); // ±50% spread range is ample
        let z = solver.solve(objective, 0.0)?;
        Ok(z)
    }
}


