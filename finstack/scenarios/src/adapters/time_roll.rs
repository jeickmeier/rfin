//! Time roll-forward adapter with carry/theta calculations.

use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::utils::parse_period_to_days;
use finstack_valuations::instruments::common::traits::Instrument;

/// Report from time roll-forward operation.
#[derive(Debug, Clone)]
pub struct RollForwardReport {
    /// Original as-of date.
    pub old_date: finstack_core::dates::Date,

    /// New as-of date after roll.
    pub new_date: finstack_core::dates::Date,

    /// Number of days rolled forward.
    pub days: i64,

    /// Per-instrument carry accrual (if instruments provided).
    pub instrument_carry: Vec<(String, f64)>,

    /// Per-instrument market value change (if instruments provided).
    pub instrument_mv_change: Vec<(String, f64)>,

    /// Total P&L from carry.
    pub total_carry: f64,

    /// Total P&L from market value changes.
    pub total_mv_change: f64,
}

/// Apply time roll-forward operation.
///
/// This advances the valuation date by the specified period and computes carry/theta
/// for each instrument. Theta is calculated as the PV change from rolling the date
/// forward while keeping all market data unchanged (curves, surfaces, FX rates).
///
/// The calculation is consistent with the theta metric definition: it measures
/// the value impact of time passage with no market changes, only rolling down curves.
pub fn apply_time_roll_forward(
    ctx: &mut ExecutionContext,
    period_str: &str,
) -> Result<RollForwardReport> {
    let old_date = ctx.as_of;
    let days = parse_period_to_days(period_str)?;

    // Calculate new date by adding days
    let new_date = old_date + time::Duration::days(days);

    // Note: Proper curve rolling would require:
    // 1. Adjusting all curve base dates
    // 2. Removing expired knot points
    // 3. Shifting remaining knots forward in time
    // For now, we simply update the as_of date and compute carry

    // Update as_of in context
    ctx.as_of = new_date;

    // Calculate carry and market value changes for instruments
    let (instrument_carry, instrument_mv_change, total_carry, total_mv_change) =
        if let Some(instruments) = ctx.instruments.as_ref() {
            calculate_instrument_pnl(instruments, ctx.market, old_date, new_date, days)?
        } else {
            (vec![], vec![], 0.0, 0.0)
        };

    Ok(RollForwardReport {
        old_date,
        new_date,
        days,
        instrument_carry,
        instrument_mv_change,
        total_carry,
        total_mv_change,
    })
}

/// Calculate P&L breakdown for instruments.
///
/// Theta (carry) is calculated as:
///   Carry = PV(end_date) - PV(start_date) + Sum(Cashflows from start to end)
///
/// This accounts for:
/// - Pull-to-par effects (PV change)
/// - Coupon/interest receipts during the period
/// - Principal payments during the period
///
/// This is consistent with the theta metric definition in valuations.
#[allow(clippy::type_complexity)]
fn calculate_instrument_pnl(
    instruments: &[Box<dyn Instrument>],
    market: &finstack_core::market_data::MarketContext,
    old_date: finstack_core::dates::Date,
    new_date: finstack_core::dates::Date,
    _days: i64,
) -> Result<(Vec<(String, f64)>, Vec<(String, f64)>, f64, f64)> {
    let mut instrument_carry = Vec::new();
    let mut instrument_mv_change = Vec::new();
    let mut total_carry = 0.0;
    let mut total_mv_change = 0.0;

    for instrument in instruments {
        let inst_id = instrument.id().to_string();

        // Calculate PV change
        let pv_change = {
            let pv_old = instrument.value(market, old_date).ok();
            let pv_new = instrument.value(market, new_date).ok();

            if let (Some(old), Some(new)) = (pv_old, pv_new) {
                new.amount() - old.amount()
            } else {
                0.0
            }
        };

        // Collect cashflows during the period using downcasting
        let cashflows_during_period =
            collect_instrument_cashflows(instrument.as_ref(), market, old_date, new_date);

        // Carry = PV change + cashflows received
        let carry = pv_change + cashflows_during_period;

        // Market value change is zero in time roll (market data unchanged)
        // All P&L comes from carry/theta
        let mv_change = 0.0;

        instrument_carry.push((inst_id.clone(), carry));
        instrument_mv_change.push((inst_id, mv_change));
        total_carry += carry;
        total_mv_change += mv_change;
    }

    Ok((
        instrument_carry,
        instrument_mv_change,
        total_carry,
        total_mv_change,
    ))
}

/// Collect cashflows for an instrument during a period.
///
/// Uses downcasting to handle instruments that implement CashflowProvider.
fn collect_instrument_cashflows(
    instrument: &dyn Instrument,
    market: &finstack_core::market_data::MarketContext,
    start_date: finstack_core::dates::Date,
    end_date: finstack_core::dates::Date,
) -> f64 {
    use finstack_valuations::cashflow::traits::CashflowProvider;
    use finstack_valuations::instruments::*;

    let instrument_any = instrument.as_any();

    // Try downcasting to instruments that implement CashflowProvider
    let cashflows = if let Some(bond) = instrument_any.downcast_ref::<Bond>() {
        bond.build_schedule(market, start_date).ok()
    } else if let Some(irs) = instrument_any.downcast_ref::<irs::InterestRateSwap>() {
        irs.build_schedule(market, start_date).ok()
    } else if let Some(deposit) = instrument_any.downcast_ref::<deposit::Deposit>() {
        deposit.build_schedule(market, start_date).ok()
    } else if let Some(fra) = instrument_any.downcast_ref::<fra::ForwardRateAgreement>() {
        fra.build_schedule(market, start_date).ok()
    } else if let Some(ir_fut) = instrument_any.downcast_ref::<ir_future::InterestRateFuture>() {
        ir_fut.build_schedule(market, start_date).ok()
    } else if let Some(equity) = instrument_any.downcast_ref::<equity::Equity>() {
        equity.build_schedule(market, start_date).ok()
    } else if let Some(fx_spot) = instrument_any.downcast_ref::<fx_spot::FxSpot>() {
        fx_spot.build_schedule(market, start_date).ok()
    } else if let Some(inf_bond) =
        instrument_any.downcast_ref::<inflation_linked_bond::InflationLinkedBond>()
    {
        inf_bond.build_schedule(market, start_date).ok()
    } else if let Some(repo) = instrument_any.downcast_ref::<repo::Repo>() {
        repo.build_schedule(market, start_date).ok()
    } else if let Some(sc) = instrument_any.downcast_ref::<structured_credit::StructuredCredit>() {
        sc.build_schedule(market, start_date).ok()
    } else if let Some(eq_trs) = instrument_any.downcast_ref::<trs::EquityTotalReturnSwap>() {
        eq_trs.build_schedule(market, start_date).ok()
    } else if let Some(fi_trs) = instrument_any.downcast_ref::<trs::FIIndexTotalReturnSwap>() {
        fi_trs.build_schedule(market, start_date).ok()
    } else if let Some(pmf) =
        instrument_any.downcast_ref::<private_markets_fund::PrivateMarketsFund>()
    {
        pmf.build_schedule(market, start_date).ok()
    } else if let Some(var_swap) = instrument_any.downcast_ref::<variance_swap::VarianceSwap>() {
        var_swap.build_schedule(market, start_date).ok()
    } else {
        None
    };

    // Sum cashflows in (start_date, end_date]
    if let Some(flows) = cashflows {
        flows
            .iter()
            .filter(|(date, _)| *date > start_date && *date <= end_date)
            .map(|(_, money)| money.amount())
            .sum()
    } else {
        0.0
    }
}
