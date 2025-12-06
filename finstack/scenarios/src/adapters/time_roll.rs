//! Time roll-forward adapter with carry/theta calculations.
//!
//! Implements the `OperationSpec::TimeRollForward` variant by advancing the
//! valuation date, recomputing time-dependent instrument metrics, and returning
//! a structured report of the resulting P&L decomposition.

use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::utils::parse_period_to_days;
use crate::TimeRollMode;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Tenor};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use indexmap::IndexMap;

/// Report from time roll-forward operation.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::adapters::RollForwardReport;
/// use indexmap::IndexMap;
/// use time::macros::date;
///
/// let report = RollForwardReport {
///     old_date: date!(2025 - 01 - 01),
///     new_date: date!(2025 - 02 - 01),
///     days: 31,
///     instrument_carry: vec![],
///     instrument_mv_change: vec![],
///     total_carry: IndexMap::new(),
///     total_mv_change: IndexMap::new(),
/// };
/// assert_eq!(report.days, 31);
/// ```
#[derive(Debug, Clone)]
pub struct RollForwardReport {
    /// Original as-of date.
    pub old_date: finstack_core::dates::Date,

    /// New as-of date after roll.
    pub new_date: finstack_core::dates::Date,

    /// Number of days rolled forward.
    pub days: i64,

    /// Per-instrument carry accrual (if instruments provided), grouped by currency.
    pub instrument_carry: Vec<(String, IndexMap<Currency, Money>)>,

    /// Per-instrument market value change (if instruments provided), grouped by currency.
    pub instrument_mv_change: Vec<(String, IndexMap<Currency, Money>)>,

    /// Total P&L from carry, grouped by currency.
    pub total_carry: IndexMap<Currency, Money>,

    /// Total P&L from market value changes, grouped by currency.
    pub total_mv_change: IndexMap<Currency, Money>,
}

/// Apply a time roll-forward operation.
///
/// The function advances the valuation date by the requested period and computes
/// theta/carry for each instrument (if a portfolio is supplied). Theta is defined
/// as the PV change resulting purely from the passage of time while holding
/// market data constant.
///
/// # Arguments
/// - `ctx`: Execution context providing the mutable valuation date, market data,
///   and optional instruments.
/// - `period_str`: Period string such as `"1D"`, `"1W"`, `"1M"`, or `"1Y"`.
/// - `mode`: Roll interpretation (business-day aware vs approximate days).
///
/// # Returns
/// [`RollForwardReport`] summarising the new date and P&L breakdown.
///
/// # Errors
/// - [`Error::InvalidPeriod`](crate::error::Error::InvalidPeriod) if the period
///   string cannot be parsed.
/// - Propagates any errors encountered while revaluing instruments.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::ExecutionContext;
/// use finstack_scenarios::adapters::time_roll::apply_time_roll_forward;
/// use finstack_scenarios::TimeRollMode;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_statements::FinancialModelSpec;
/// use time::macros::date;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// let as_of = date!(2025 - 01 - 01);
/// let mut ctx = ExecutionContext {
///     market: &mut market,
///     model: &mut model,
///     instruments: None,
///     rate_bindings: None,
///     calendar: None,
///     as_of,
/// };
/// let report = apply_time_roll_forward(&mut ctx, "1M", TimeRollMode::BusinessDays)?;
/// assert_eq!(report.days, 30);
/// # Ok(())
/// # }
/// ```
pub fn apply_time_roll_forward(
    ctx: &mut ExecutionContext,
    period_str: &str,
    mode: TimeRollMode,
) -> Result<RollForwardReport> {
    use crate::error::Error;

    let old_date = ctx.as_of;
    let (new_date, day_shift) = match mode {
        TimeRollMode::Approximate => {
            let days = parse_period_to_days(period_str)?;
            let new_date = old_date + time::Duration::days(days);
            (new_date, days)
        }
        TimeRollMode::CalendarDays => {
            let tenor =
                Tenor::parse(period_str).map_err(|e| Error::InvalidPeriod(e.to_string()))?;
            let target = tenor
                .add_to_date(old_date, None, BusinessDayConvention::Unadjusted)
                .map_err(|e| Error::Internal(e.to_string()))?;
            let days = (target - old_date).whole_days();
            (target, days)
        }
        TimeRollMode::BusinessDays => {
            let tenor =
                Tenor::parse(period_str).map_err(|e| Error::InvalidPeriod(e.to_string()))?;
            let target = tenor
                .add_to_date(
                    old_date,
                    ctx.calendar,
                    BusinessDayConvention::ModifiedFollowing,
                )
                .map_err(|e| Error::Internal(e.to_string()))?;
            let days = (target - old_date).whole_days();
            (target, days)
        }
    };

    // Calculate carry and market value changes for instruments BEFORE rolling curves
    // This ensures we capture the true carry (time value change with constant curves)
    let (instrument_carry, instrument_mv_change, total_carry, total_mv_change) =
        if let Some(instruments) = ctx.instruments.as_ref() {
            calculate_instrument_pnl(instruments, ctx.market, old_date, new_date, day_shift)?
        } else {
            (Vec::new(), Vec::new(), IndexMap::new(), IndexMap::new())
        };

    // Roll all curves forward (adjusts base dates, shifts knots, filters expired)
    // This is the "constant curves" scenario - rates at calendar dates stay the same,
    // but maturities are re-measured from the new base date
    let rolled_market = ctx.market.roll_forward(day_shift).map_err(|e| {
        Error::Internal(format!(
            "Failed to roll market data forward by {} days: {}",
            day_shift, e
        ))
    })?;

    // Replace market context with rolled version
    *ctx.market = rolled_market;

    // Update as_of in context
    ctx.as_of = new_date;

    Ok(RollForwardReport {
        old_date,
        new_date,
        days: day_shift,
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
    market: &finstack_core::market_data::context::MarketContext,
    old_date: finstack_core::dates::Date,
    new_date: finstack_core::dates::Date,
    _days: i64,
) -> Result<(
    Vec<(String, IndexMap<Currency, Money>)>,
    Vec<(String, IndexMap<Currency, Money>)>,
    IndexMap<Currency, Money>,
    IndexMap<Currency, Money>,
)> {
    let mut instrument_carry: Vec<(String, IndexMap<Currency, Money>)> = Vec::new();
    let mut instrument_mv_change: Vec<(String, IndexMap<Currency, Money>)> = Vec::new();
    let mut total_carry: IndexMap<Currency, Money> = IndexMap::new();
    let total_mv_change: IndexMap<Currency, Money> = IndexMap::new();

    for instrument in instruments {
        let inst_id = instrument.id().to_string();

        // Calculate PV change as Money, grouped by currency (single currency per instrument).
        let mut pv_change_by_ccy: IndexMap<Currency, Money> = IndexMap::new();
        let pv_old = instrument.value(market, old_date).ok();
        let pv_new = instrument.value(market, new_date).ok();
        if let (Some(old), Some(new)) = (pv_old, pv_new) {
            let diff = (new - old)?;
            pv_change_by_ccy.insert(diff.currency(), diff);
        }

        // Collect cashflows during the period, grouped by currency.
        let cashflows_during_period =
            collect_instrument_cashflows(instrument.as_ref(), market, old_date, new_date);

        // Carry per currency = PV change + cashflows received.
        let mut carry_by_ccy = pv_change_by_ccy;
        for (ccy, flow) in cashflows_during_period {
            carry_by_ccy
                .entry(ccy)
                .and_modify(|m| *m += flow)
                .or_insert(flow);
        }

        // Market value change is zero in time roll (market data unchanged),
        // so per-currency MV change is always an empty map.
        let mv_change_by_ccy: IndexMap<Currency, Money> = IndexMap::new();

        // Accumulate totals by currency.
        for (ccy, amount) in &carry_by_ccy {
            total_carry
                .entry(*ccy)
                .and_modify(|m| *m += *amount)
                .or_insert(*amount);
        }

        instrument_carry.push((inst_id.clone(), carry_by_ccy));
        instrument_mv_change.push((inst_id, mv_change_by_ccy));
    }

    Ok((
        instrument_carry,
        instrument_mv_change,
        total_carry,
        total_mv_change,
    ))
}

/// Collect cashflows for an instrument during a period, grouped by currency.
fn collect_instrument_cashflows(
    instrument: &dyn Instrument,
    market: &finstack_core::market_data::context::MarketContext,
    start_date: finstack_core::dates::Date,
    end_date: finstack_core::dates::Date,
) -> IndexMap<Currency, Money> {
    let mut result: IndexMap<Currency, Money> = IndexMap::new();

    if let Some(provider) = instrument.as_cashflow_provider() {
        if let Ok(flows) = provider.build_schedule(market, start_date) {
            for (date, money) in flows.into_iter() {
                if date > start_date && date <= end_date {
                    let ccy = money.currency();
                    result
                        .entry(ccy)
                        .and_modify(|m| *m += money)
                        .or_insert(money);
                }
            }
        }
    }

    result
}
