use crate::instruments::bond::pricing::helpers as price_helpers;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::math::solver::{BrentSolver, Solver};


/// Discount Margin (DM) for floating-rate bonds.
///
/// Definition: constant additive spread (decimal, e.g., 0.01 = 100bp) over the
/// reference forward index such that the discounted PV of the bond's projected
/// cashflows equals the observed dirty market price.
///
/// Notes:
/// - Requires quoted clean price or falls back to base PV as target.
/// - Uses the FRN path: coupons are projected off the forward curve at reset
///   with margin and gearing from `BondFloatSpec`, then discounted with the
///   discount curve. The DM is added to the projected index rate.
pub struct DiscountMarginCalculator;

impl DiscountMarginCalculator {
    fn pv_given_dm(
        bond: &Bond,
        curves: &finstack_core::market_data::MarketContext,
        as_of: Date,
        dm: f64,
    ) -> finstack_core::Result<f64> {
        price_helpers::price_from_dm(bond, curves, as_of, dm)
    }
}

impl MetricCalculator for DiscountMarginCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Determine dirty market price in currency
        let dirty_ccy = if let Some(clean_px) = bond.pricing_overrides.quoted_clean_price {
            let accrued = context
                .computed
                .get(&MetricId::Accrued)
                .copied()
                .unwrap_or(0.0);
            clean_px * bond.notional.amount() / 100.0 + accrued
        } else {
            context.base_value.amount()
        };

        // If no floating spec, DM is zero by definition here
        if bond.float.is_none() {
            return Ok(0.0);
        }

        // Root-find DM such that PV(dm) - dirty = 0
        let objective = |dm: f64| -> f64 {
            match Self::pv_given_dm(bond, &context.curves, context.as_of, dm) {
                Ok(pv) => pv - dirty_ccy,
                Err(_) => 1e12 * dm.signum(),
            }
        };

        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(0.05));
        // Initial guess 0.0 (0 bp). DM returned in decimal (e.g., 0.01 = 100bp)
        let dm = solver.solve(objective, 0.0)?;
        Ok(dm)
    }
}
