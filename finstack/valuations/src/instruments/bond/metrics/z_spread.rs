use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
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
        // Build or reuse holder cashflows (avoid holding `bond` borrow across mutations)
        let flows: Vec<(Date, finstack_core::money::Money)> = if let Some(f) = &context.cashflows {
            f.clone()
        } else {
            let bond: &Bond = context.instrument_as()?;
            let disc_id = bond.disc_id.clone();
            let dc = bond.dc;
            let built = bond.build_schedule(&context.curves, context.as_of)?;
            context.cashflows = Some(built.clone());
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
            built
        };

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

        // Fetch base discount curve
        let disc_curve = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                bond.disc_id.as_ref(),
            )?;
        let base_date = disc_curve.base_date();

        // Precompute (t, amount) for cashflows strictly after as_of
        let mut times_and_amounts: Vec<(F, F)> = Vec::with_capacity(flows.len());
        for (date, amt) in &flows {
            if *date <= context.as_of {
                continue;
            }
            let t = bond
                .dc
                .year_fraction(
                    base_date,
                    *date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            if t > 0.0 {
                times_and_amounts.push((t, amt.amount()));
            }
        }

        if times_and_amounts.is_empty() {
            return Ok(0.0);
        }

        // Objective: PV_z(z) - target_value_ccy = 0
        let objective = |z: F| -> F {
            let mut pv = 0.0;
            for (t, amount) in &times_and_amounts {
                let df = disc_curve.df(*t);
                let df_z = df * (-z * *t).exp();
                pv += *amount * df_z;
            }
            pv - target_value_ccy
        };

        // Solve using Brent with a reasonable bracket
        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(0.5)); // ±50% spread range is ample
        let z = solver.solve(objective, 0.0)?;
        Ok(z)
    }
}


