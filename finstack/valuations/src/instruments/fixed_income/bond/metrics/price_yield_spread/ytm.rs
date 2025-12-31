use crate::cashflow::traits::CashflowProvider;
use crate::instruments::bond::CashflowSpec;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::money::Money;
use rust_decimal::prelude::ToPrimitive;

/// Calculates yield to maturity (YTM) for bonds.
///
/// YTM is defined here as the internal rate of return that equates the present
/// value of **all projected future cashflows** to the current dirty market
/// price (quoted clean price plus accrued interest).
///
/// # Applicability
///
/// - **Primary use**: plain-vanilla **fixed-rate bullet bonds**, where YTM has
///   the usual market interpretation (coupon-like yield for comparison).
/// - **Other cashflow specs**: for floating-rate, amortizing, or custom
///   cashflow structures, this calculator still solves a well-defined IRR off
///   the full discounted cashflow schedule. The resulting YTM is a
///   **cashflow-implied yield**, but it is **not** the market-standard quote
///   for FRNs (where **discount margin** is preferred) and may have less direct
///   interpretation for exotic structures.
///
/// Implementation detail: the `coupon_rate` field in `YtmPricingSpec` is used
/// only as a **solver hint / initial guess**. For non-fixed `CashflowSpec`
/// variants this is set to `0.0`, but the solved YTM is fully determined by
/// the explicit projected cashflows and the target price, not by this hint.
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // YTM is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        // Extract fields we need from the bond
        let (maybe_clean_px, notional, dc, discount_curve_id, coupon, freq) = {
            let bond: &Bond = context.instrument_as()?;
            (
                bond.pricing_overrides.quoted_clean_price,
                bond.notional,
                bond.cashflow_spec.day_count(),
                bond.discount_curve_id.to_owned(),
                match &bond.cashflow_spec {
                    // Rate overflow is extremely unlikely for interest rates,
                    // but use 0.0 as initial guess hint (solver will find correct YTM)
                    CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
                    _ => 0.0,
                },
                bond.cashflow_spec.frequency(),
            )
        };

        // Determine dirty price in currency.
        //
        // Preferred path: use quoted clean price (market quote) plus accrued
        // interest to build the dirty market price. When no quoted clean price
        // is available, fall back to the model PV from `context.base_value`,
        // which provides a well-defined cashflow-implied yield consistent with
        // the discount curve.
        let dirty: Money = if let Some(clean_px) = maybe_clean_px {
            // Get accrued from computed metrics
            let ai = context
                .computed
                .get(&MetricId::Accrued)
                .copied()
                .ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::InputError::NotFound {
                        id: "metric:Accrued".to_string(),
                    })
                })?;

            // Compute dirty price in currency: clean is quoted % of par
            let dirty_amt = (clean_px * notional.amount() / 100.0) + ai;
            Money::new(dirty_amt, notional.currency())
        } else {
            // Fallback: use model PV as dirty price. This preserves the semantic
            // that YTM is the IRR of the full projected cashflows, and avoids
            // hard failures when no explicit market quote is present.
            context.base_value
        };

        // Build and cache flows and hints if not already present
        if context.cashflows.is_none() {
            let bond: &Bond = context.instrument_as()?;
            let flows = bond.build_dated_flows(&context.curves, context.as_of)?;
            context.cashflows = Some(flows);
            context.discount_curve_id = Some(discount_curve_id);
            context.day_count = Some(dc);
        }
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "cashflows".to_string(),
            })
        })?;

        // Solve for YTM using shared solver with Street compounding (default)
        let ytm = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            flows,
            context.as_of,
            dirty,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: dc,
                notional,
                coupon_rate: coupon,
                compounding:
                    crate::instruments::bond::pricing::quote_engine::YieldCompounding::Street,
                frequency: freq,
            },
        )?;

        Ok(ytm)
    }
}
