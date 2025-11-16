use crate::instruments::bond::CashflowSpec;
use crate::instruments::bond::pricing::helpers as price_helpers;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::math::solver::{BrentSolver, Solver};
use std::cell::RefCell;

/// Discount Margin (DM) for floating-rate bonds.
///
/// Definition: constant additive spread (decimal, e.g., 0.01 = 100bp) over the
/// reference forward index such that the discounted PV of the bond's projected
/// cashflows equals the observed dirty market price.
///
/// Notes:
/// - Requires quoted clean price or falls back to base PV as target.
/// - Uses the FRN path: coupons are projected off the forward curve at reset
///   with margin and gearing from `FloatingCouponSpec`, then discounted with the
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
        if !matches!(&bond.cashflow_spec, CashflowSpec::Floating(_)) {
            return Ok(0.0);
        }

        // Root-find DM such that PV(dm) - dirty = 0
        let pricing_error: RefCell<Option<finstack_core::Error>> = RefCell::new(None);

        let objective = |dm: f64| -> f64 {
            match Self::pv_given_dm(bond, &context.curves, context.as_of, dm) {
                Ok(pv) => pv - dirty_ccy,
                Err(e) => {
                    // Capture the first pricing error and map to a large non-zero residual
                    let mut slot = pricing_error.borrow_mut();
                    if slot.is_none() {
                        *slot = Some(e);
                    }
                    drop(slot);
                    // Use a large residual with deterministic sign so the solver never sees a
                    // spurious "perfect fit" at the initial guess (0.0 DM).
                    1e12 * if dm >= 0.0 { 1.0 } else { -1.0 }
                }
            }
        };

        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(0.05));
        // Initial guess 0.0 (0 bp). DM returned in decimal (e.g., 0.01 = 100bp)
        let dm = solver.solve(objective, 0.0)?;

        // If any pricing error occurred during objective evaluation, surface it instead of
        // returning a potentially meaningless DM.
        if let Some(err) = pricing_error.into_inner() {
            return Err(err);
        }

        Ok(dm)
    }
}
