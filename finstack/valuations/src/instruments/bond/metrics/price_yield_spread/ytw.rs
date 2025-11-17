use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::money::Money;

/// Calculates yield-to-worst (YTW) for bonds with call/put schedules.
///
/// YTW is defined here as the minimum yield-to-maturity across all admissible
/// exercise paths (calls, puts, and final maturity), where each candidate
/// yield is solved as an IRR that equates the present value of the **truncated
/// projected cashflows** to the current dirty market price.
///
/// # Applicability
///
/// - **Primary use**: callable / putable **fixed-rate bonds**, where YTW is
///   the standard market risk measure ("worst case" yield to any exercise).
/// - **Other cashflow specs**: for floating-rate, amortizing, or custom
///   structures, this calculator still computes a well-defined "worst-case
///   cashflow-implied yield" across exercise dates, but this is **not** the
///   standard FRN quoting convention and should be interpreted as an internal
///   risk/analytics measure rather than a market quote. For FRNs, **discount
///   margin** is typically the preferred spread metric.
///
/// As with `YtmCalculator`, the `coupon_rate` field passed into the YTM solver
/// is only an **initial guess**; for non-fixed `CashflowSpec` variants it is
/// set to `0.0`, but the solved YTW values are driven entirely by the explicit
/// projected cashflows and the target price along each exercise path.
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // YTW is computed automatically when requesting bond metrics for callable/putable bonds
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn dependencies(&self) -> &[MetricId] {
        // YTW is defined off the market price (quoted clean + accrued), so we
        // require Accrued to be computed first to construct the dirty price.
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        // Build and cache flows and hints if not already present
        let flows = if let Some(ref flows) = context.cashflows {
            flows
        } else {
            let (discount_curve_id, dc, built) = {
                let bond: &Bond = context.instrument_as()?;
                (
                    bond.discount_curve_id.to_owned(),
                    bond.cashflow_spec.day_count(),
                    bond.build_schedule(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(discount_curve_id);
            context.day_count = Some(dc);
            context.cashflows.as_ref().ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "cashflows".to_string(),
                })
            })?
        };

        // Construct current dirty market price from quoted clean price + accrued interest.
        //
        // This mirrors the YTM and DirtyPrice calculators so that YTW is
        // defined relative to the same market price, not the model PV.
        let bond: &Bond = context.instrument_as()?;
        let clean_px = bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "bond.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;

        // Get accrued from computed metrics (dependency ensures this is present).
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Dirty price in currency: quoted clean is % of par.
        let dirty_amt = (clean_px * bond.notional.amount() / 100.0) + accrued;
        let dirty_now = Money::new(dirty_amt, bond.notional.currency());

        // Delegate candidate scanning and YTM solving to shared helper.
        let (best_ytm, _best_flows) =
            crate::instruments::bond::pricing::quote_engine::solve_ytw_from_flows(
                bond,
                flows,
                context.as_of,
                dirty_now,
            )?;

        Ok(best_ytm)
    }
}

impl YtwCalculator {}
