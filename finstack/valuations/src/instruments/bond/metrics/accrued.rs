use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates accrued interest for bonds.
///
/// Computes the accrued interest since the last coupon payment up to the
/// valuation date. This is essential for determining the dirty price and
/// other bond metrics that depend on accrued interest.
///
/// See unit tests and `examples/` for usage.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Scope the bond borrow to avoid conflicts with &mut context later
        let (last, next, period_coupon_amount, disc_id, dc, maybe_flows) = {
            let bond: &Bond = context.instrument_as()?;

            // Determine coupon periods from actual schedule when available
            let (last, next, period_coupon_amount) = if let Some(ref custom) = bond.custom_cashflows
            {
                // Use coupon flows (Fixed/Stub) from custom schedule
                let mut coupon_dates: Vec<(
                    finstack_core::dates::Date,
                    finstack_core::money::Money,
                )> = Vec::new();
                for cf in &custom.flows {
                    if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
                        coupon_dates.push((cf.date, cf.amount));
                    }
                }
                if coupon_dates.len() < 2 {
                    return Ok(0.0);
                }
                // Find window around as_of and the coupon amount on the next date
                let mut found = None;
                for w in coupon_dates.windows(2) {
                    let (a, _a_amt) = w[0];
                    let (b, b_amt) = w[1];
                    if a <= context.as_of && context.as_of < b {
                        found = Some((a, b, b_amt));
                        break;
                    }
                }
                match found {
                    Some((a, b, amt)) => (a, b, amt),
                    None => return Ok(0.0),
                }
            } else {
                // Fallback to canonical schedule using bond fields
                let sched = crate::cashflow::builder::build_dates(
                    bond.issue,
                    bond.maturity,
                    bond.freq,
                    finstack_core::dates::StubKind::None,
                    finstack_core::dates::BusinessDayConvention::Following,
                    None,
                );
                let dates = sched.dates;
                if dates.len() < 2 {
                    return Ok(0.0);
                }
                let mut last = dates[0];
                let mut next = dates[1];
                let mut found = false;
                for w in dates.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if a <= context.as_of && context.as_of < b {
                        last = a;
                        next = b;
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(0.0);
                }
                // Period coupon amount based on notional × rate × yf
                let yf = bond
                    .dc
                    .year_fraction(last, next, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0);
                let coupon_amt = bond.notional * (bond.coupon * yf);
                (last, next, coupon_amt)
            };

            // Prepare potential flows for caching (build now, assign later)
            let maybe_flows = if context.cashflows.is_none() {
                Some(bond.build_schedule(&context.curves, context.as_of)?)
            } else {
                None
            };

            (
                last,
                next,
                period_coupon_amount,
                bond.disc_id.clone(),
                bond.dc,
                maybe_flows,
            )
        };

        // Calculate accrued interest linearly within the coupon period
        let yf_total = dc
            .year_fraction(last, next, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        if yf_total <= 0.0 {
            return Ok(0.0);
        }
        let elapsed = dc
            .year_fraction(
                last,
                context.as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0)
            .max(0.0);
        let accrued = period_coupon_amount * (elapsed / yf_total);

        // Cache basic context hints for downstream metrics
        context.discount_curve_id = Some(disc_id.clone());
        context.day_count = Some(dc);
        // Also cache full holder cashflows for downstream risk metrics
        if context.cashflows.is_none() {
            if let Some(flows) = maybe_flows {
                context.cashflows = Some(flows);
            }
        }

        Ok(accrued.amount())
    }
}


