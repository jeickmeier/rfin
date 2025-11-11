//! Period-aligned DataFrame exports for cashflow schedules.
//!
//! This module provides DataFrame-like representations of cashflow schedules aligned
//! to period boundaries. It computes all derived columns (discount factors, survival
//! probabilities, base rates, spreads, unfunded amounts) in Rust for consistency
//! across language bindings.
//!
//! ## Design
//!
//! - All computations happen in Rust to ensure deterministic results across Python/WASM bindings
//! - Historical cashflows (`date <= as_of/base`) are included for auditability but contribute zero PV
//! - Optional columns (survival_probs, base_rates, spreads, etc.) are conditionally computed
//! - Facility limits enable undrawn balance calculations for revolving credit facilities

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::primitives::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Period};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

/// Options for period-aligned DataFrame exports.
///
/// Controls which optional columns are computed and provides configuration
/// for market data lookups and discounting conventions.
#[derive(Debug, Clone, Default)]
pub struct PeriodDataFrameOptions<'a> {
    /// Optional hazard curve ID for credit-adjusted discounting
    pub hazard_curve_id: Option<&'a str>,
    /// Optional forward curve ID for floating rate decomposition
    pub forward_curve_id: Option<&'a str>,
    /// Valuation date (defaults to discount curve base date if not provided)
    pub as_of: Option<Date>,
    /// Day count convention for year fraction calculations
    pub day_count: Option<DayCount>,
    /// Optional override for discounting time calculation basis.
    ///
    /// When provided, the discounting time 't' will be computed using this
    /// day-count instead of `day_count`/schedule DC.
    pub discount_day_count: Option<DayCount>,
    /// Facility limit/commitment for undrawn balance calculations
    pub facility_limit: Option<Money>,
    /// Whether to include floating rate decomposition (base_rates, spreads)
    pub include_floating_decomposition: bool,
}

/// Period-aligned DataFrame-like result.
///
/// Contains row-oriented vectors representing cashflows aligned to period boundaries.
/// All vectors have the same length corresponding to the number of cashflows that
/// fall within the provided periods.
///
/// ## Columns
///
/// ### Required columns
/// - `start_dates`: Period start dates
/// - `end_dates`: Period end dates (payment dates)
/// - `pay_dates`: Actual cashflow payment dates
/// - `reset_dates`: Reset dates for floating rate fixings
/// - `cf_types`: Cashflow kinds (Fixed, FloatReset, Amortization, etc.)
/// - `currencies`: Currency for each cashflow
/// - `amounts`: Cashflow amounts
/// - `accrual_factors`: Year fractions from cashflow
/// - `rates`: Effective rates from cashflow
/// - `discount_factors`: Discount factors from base date
/// - `pvs`: Present values (amount * DF * survival probability)
///
/// Optional columns (Some if requested, None otherwise)
/// - `notionals`: Outstanding balance for accruing flows
/// - `undrawn_notionals`: Undrawn balance (facility_limit - outstanding)
/// - `survival_probs`: Survival probabilities if hazard curve provided
/// - `unfunded_amounts`: Undrawn amounts if facility_limit provided
/// - `commitment_amounts`: Facility limit amounts
/// - `base_rates`: Forward rates if floating decomposition enabled
/// - `spreads`: Margin over forward rate if floating decomposition enabled
#[derive(Clone)]
pub struct PeriodDataFrame {
    pub start_dates: Vec<Date>,
    pub end_dates: Vec<Date>,
    pub pay_dates: Vec<Date>,
    pub reset_dates: Vec<Option<Date>>,
    pub cf_types: Vec<CFKind>,
    pub currencies: Vec<Currency>,
    pub notionals: Vec<Option<f64>>,
    pub undrawn_notionals: Option<Vec<Option<f64>>>,
    pub yr_fraqs: Vec<f64>,
    pub accrual_factors: Vec<f64>,
    pub days: Vec<i64>,
    pub amounts: Vec<f64>,
    pub rates: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub survival_probs: Option<Vec<Option<f64>>>,
    pub pvs: Vec<f64>,
    pub unfunded_amounts: Option<Vec<Option<f64>>>,
    pub commitment_amounts: Option<Vec<Option<f64>>>,
    pub base_rates: Option<Vec<Option<f64>>>,
    pub spreads: Option<Vec<Option<f64>>>,
}

impl PeriodDataFrame {
    /// Create an empty DataFrame.
    fn empty() -> Self {
        Self {
            start_dates: Vec::new(),
            end_dates: Vec::new(),
            pay_dates: Vec::new(),
            reset_dates: Vec::new(),
            cf_types: Vec::new(),
            currencies: Vec::new(),
            notionals: Vec::new(),
            undrawn_notionals: None,
            yr_fraqs: Vec::new(),
            accrual_factors: Vec::new(),
            days: Vec::new(),
            amounts: Vec::new(),
            rates: Vec::new(),
            discount_factors: Vec::new(),
            survival_probs: None,
            pvs: Vec::new(),
            unfunded_amounts: None,
            commitment_amounts: None,
            base_rates: None,
            spreads: None,
        }
    }
}

impl CashFlowSchedule {
    /// Export all cashflows as DataFrame without period filtering.
    ///
    /// Convenience wrapper that creates a single period spanning all cashflows.
    /// Useful for debugging and full schedule inspection.
    ///
    /// # Arguments
    ///
    /// * `market` - Market context containing discount curves
    /// * `discount_curve_id` - ID of the discount curve to use
    /// * `options` - Additional configuration (hazard/forward IDs, overrides, facility limits)
    ///
    /// # Returns
    ///
    /// A `PeriodDataFrame` with all cashflows included.
    pub fn to_dataframe(
        &self,
        market: &MarketContext,
        discount_curve_id: &str,
        options: PeriodDataFrameOptions<'_>,
    ) -> finstack_core::Result<PeriodDataFrame> {
        if self.flows.is_empty() {
            return Ok(PeriodDataFrame::empty());
        }
        
        // Create single period spanning all flows
        let first = self.flows.first().unwrap().date;
        let last = self.flows.last().unwrap().date;
        let period = Period {
            id: finstack_core::dates::PeriodId::annual(first.year()),
            start: first,
            end: last,
            is_actual: true,
        };
        
        self.to_period_dataframe(&[period], market, discount_curve_id, options)
    }

    /// Period-aligned DataFrame-like export with optional credit and floating decomposition.
    ///
    /// This computes all derived columns (discount factors, survival probabilities,
    /// base rate, spread, all-in rate, unfunded amounts) in Rust for consistency
    /// across language bindings. Bindings should only perform type conversion.
    ///
    /// Historical cashflows (`date <= as_of/base`) are included in the table for
    /// auditability but contribute zero PV by convention.
    ///
    /// # Arguments
    ///
    /// * `periods` - Period definitions with start/end boundaries
    /// * `market` - Market context containing discount and optional curves
    /// * `discount_curve_id` - ID of the discount curve to use
    /// * `options` - Additional configuration (hazard/forward IDs, overrides, facility limits)
    ///
    /// # Returns
    ///
    /// A `PeriodDataFrame` with all computed columns.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The discount curve is not found in the market context
    /// - Hazard curve is specified but not found
    /// - Currency mismatches occur in facility limit calculations
    pub fn to_period_dataframe(
        &self,
        periods: &[Period],
        market: &MarketContext,
        discount_curve_id: &str,
        options: PeriodDataFrameOptions<'_>,
    ) -> finstack_core::Result<PeriodDataFrame> {
        let dc = options.day_count.unwrap_or(self.day_count);

        let disc_arc = market.get_discount(discount_curve_id)?;
        let base = options.as_of.unwrap_or_else(|| disc_arc.base_date());

        let has_hazard = options.hazard_curve_id.is_some();
        let hazard_arc_opt = if let Some(hz) = options.hazard_curve_id {
            Some(market.get_hazard(hz)?)
        } else {
            None
        };
        let forward_arc_opt = if options.include_floating_decomposition {
            options
                .forward_curve_id
                .and_then(|fid| market.get_forward(fid).ok())
        } else {
            None
        };

        // Prefer explicit facility_limit; fallback to schedule meta (e.g., RCF commitment)
        let facility_limit = options
            .facility_limit
            .or(self.meta.facility_limit);

        // Columns
        let mut start_dates: Vec<Date> = Vec::new();
        let mut end_dates: Vec<Date> = Vec::new();
        let mut pay_dates: Vec<Date> = Vec::new();
        let mut reset_dates: Vec<Option<Date>> = Vec::new();
        let mut cf_types: Vec<CFKind> = Vec::new();
        let mut currencies: Vec<Currency> = Vec::new();
        let mut notionals: Vec<Option<f64>> = Vec::new();
        let mut undrawn_notionals: Vec<Option<f64>> = Vec::new();
        let mut yr_fraqs: Vec<f64> = Vec::new();
        let mut accrual_factors: Vec<f64> = Vec::new();
        let mut days: Vec<i64> = Vec::new();
        let mut amounts: Vec<f64> = Vec::new();
        let mut rates: Vec<f64> = Vec::new();
        let mut discount_factors: Vec<f64> = Vec::new();
        let mut survival_probs: Option<Vec<Option<f64>>> =
            if has_hazard { Some(Vec::new()) } else { None };
        let mut pvs: Vec<f64> = Vec::new();
        let mut unfunded_amounts: Option<Vec<Option<f64>>> =
            facility_limit.as_ref().map(|_| Vec::new());
        let mut commitment_amounts: Option<Vec<Option<f64>>> =
            facility_limit.as_ref().map(|_| Vec::new());
        let mut base_rates: Option<Vec<Option<f64>>> = if options.include_floating_decomposition {
            Some(Vec::new())
        } else {
            None
        };
        let mut spreads: Option<Vec<Option<f64>>> = if options.include_floating_decomposition {
            Some(Vec::new())
        } else {
            None
        };

        // Track outstanding drawn balance for Notional column
        let mut outstanding = self.notional.initial;

        for cf in &self.flows {
            // Find containing period (inclusive end)
            let period_opt = periods
                .iter()
                .find(|p| cf.date >= p.start && cf.date <= p.end);
            if period_opt.is_none() {
                continue;
            }
            let period = period_opt.unwrap();

            // Outstanding before this cashflow
            let outstanding_pre = outstanding;
            match cf.kind {
                CFKind::Amortization => {
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                CFKind::PIK => {
                    outstanding = outstanding.checked_add(cf.amount)?;
                }
                CFKind::Notional => {
                    // Draws are negative, repays are positive from lender perspective
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                _ => {}
            }

            // Basic columns
            start_dates.push(period.start);
            end_dates.push(period.end);
            pay_dates.push(cf.date);
            reset_dates.push(cf.reset_date);
            cf_types.push(cf.kind);
            currencies.push(cf.amount.currency());
            amounts.push(cf.amount.amount());
            accrual_factors.push(cf.accrual_factor);
            rates.push(cf.rate.unwrap_or(0.0));

            // Notional balances for interest/fee-like rows
            let (notional_drawn, notional_undrawn) =
                if matches!(cf.kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset
                    | CFKind::CommitmentFee | CFKind::UsageFee | CFKind::FacilityFee)
                    || cf.accrual_factor > 0.0
                {
                    let drawn = Some(outstanding_pre.amount());
                    // Undrawn only available when facility_limit (commitment) is provided
                    let undrawn = if let Some(limit) = facility_limit.as_ref() {
                        if limit.currency() == cf.amount.currency() {
                            Some((limit.amount() - outstanding_pre.amount()).max(0.0))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    (drawn, undrawn)
                } else {
                    (None, None)
                };
            notionals.push(notional_drawn);
            undrawn_notionals.push(notional_undrawn);

            // YrFraq and Days
            let yr_fraq = dc
                .year_fraction(period.start, cf.date, DayCountCtx::default())
                .unwrap_or(0.0);
            yr_fraqs.push(yr_fraq);
            days.push((cf.date - period.start).whole_days());

            // Discount factor using configured discounting basis
            let dc_for_discounting = options.discount_day_count.unwrap_or(dc);
            let t = if cf.date == base {
                0.0
            } else if cf.date > base {
                dc_for_discounting
                    .year_fraction(base, cf.date, DayCountCtx::default())
                    .unwrap_or(0.0)
            } else {
                -dc_for_discounting
                    .year_fraction(cf.date, base, DayCountCtx::default())
                    .unwrap_or(0.0)
            };
            let df = disc_arc.df(t);
            discount_factors.push(df);

            // Survival probability
            if let (Some(h), Some(spv)) = (hazard_arc_opt.as_ref(), survival_probs.as_mut()) {
                spv.push(Some(h.sp(t)));
            }

            // PV
            let sp_mult = if let Some(ref spv) = survival_probs {
                spv.last().copied().flatten().unwrap_or(1.0)
            } else {
                1.0
            };
            let pv_amt = if cf.date > base {
                cf.amount.amount() * df * sp_mult
            } else {
                0.0
            };
            pvs.push(pv_amt);

            // Unfunded and commitment amounts
            if let Some(limit) = facility_limit.as_ref() {
                if let Some(ref mut unfunded_vec) = unfunded_amounts {
                    if limit.currency() == cf.amount.currency() {
                        let val = (limit.amount() - outstanding_pre.amount()).max(0.0);
                        unfunded_vec.push(Some(val));
                    } else {
                        unfunded_vec.push(None);
                    }
                }
                if let Some(ref mut commit_vec) = commitment_amounts {
                    if limit.currency() == cf.amount.currency() {
                        commit_vec.push(Some(limit.amount()));
                    } else {
                        commit_vec.push(None);
                    }
                }
            }

            // Floating decomposition
            let mut base_rate_opt: Option<f64> = None;
            let mut spread_opt: Option<f64> = None;
            if options.include_floating_decomposition && matches!(cf.kind, CFKind::FloatReset) {
                if let Some(ref fwd) = forward_arc_opt {
                    let reset_t = if let Some(reset_date) = cf.reset_date {
                        if reset_date == base {
                            0.0
                        } else if reset_date > base {
                            fwd.day_count()
                                .year_fraction(base, reset_date, DayCountCtx::default())
                                .unwrap_or(0.0)
                        } else {
                            -fwd.day_count()
                                .year_fraction(reset_date, base, DayCountCtx::default())
                                .unwrap_or(0.0)
                        }
                    } else {
                        fwd.day_count()
                            .year_fraction(base, period.start, DayCountCtx::default())
                            .unwrap_or(0.0)
                    };
                    let b = fwd.rate(reset_t);
                    base_rate_opt = Some(b);
                    // Spread = rate - base_rate for floating cashflows
                    if let Some(rate) = cf.rate {
                        spread_opt = Some(rate - b);
                    }
                }
            }
            if let Some(ref mut br) = base_rates {
                br.push(base_rate_opt);
            }
            if let Some(ref mut sp) = spreads {
                sp.push(spread_opt);
            }
        }

        Ok(PeriodDataFrame {
            start_dates,
            end_dates,
            pay_dates,
            reset_dates,
            cf_types,
            currencies,
            notionals,
            undrawn_notionals: Some(undrawn_notionals),
            yr_fraqs,
            accrual_factors,
            days,
            amounts,
            rates,
            discount_factors,
            survival_probs,
            pvs,
            unfunded_amounts,
            commitment_amounts,
            base_rates,
            spreads,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::schedule::{CashFlowSchedule, CashflowMeta};
    use crate::cashflow::primitives::{CashFlow, Notional};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Period, PeriodId};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
    }

    fn quarters_2025() -> Vec<Period> {
        vec![
            Period {
                id: PeriodId::quarter(2025, 1),
                start: d(2025, 1, 1),
                end: d(2025, 4, 1),
                is_actual: true,
            },
            Period {
                id: PeriodId::quarter(2025, 2),
                start: d(2025, 4, 1),
                end: d(2025, 7, 1),
                is_actual: false,
            },
        ]
    }

    #[test]
    fn dataframe_sets_zero_pv_for_historical_rows() {
        // Build a simple schedule with one historical and one future cashflow
        let base = d(2025, 4, 1);
        let flows = vec![
            CashFlow {
                date: d(2025, 3, 15), // historical
                reset_date: None,
                amount: Money::new(100.0, Currency::USD),
                kind: CFKind::Fixed,
                accrual_factor: 0.25,
                rate: None,
            },
            CashFlow {
                date: d(2025, 5, 15), // future
                reset_date: None,
                amount: Money::new(200.0, Currency::USD),
                kind: CFKind::Fixed,
                accrual_factor: 0.25,
                rate: None,
            },
        ];
        let schedule = CashFlowSchedule {
            flows,
            notional: Notional::par(1_000.0, Currency::USD),
            day_count: DayCount::Act365F,
            meta: CashflowMeta::default(),
        };

        // Market context with flat discount curve (df = 1.0)
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (30.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        let periods = quarters_2025();
        let options = PeriodDataFrameOptions {
            hazard_curve_id: None,
            forward_curve_id: None,
            as_of: Some(base),
            day_count: Some(DayCount::Act365F),
            discount_day_count: None,
            facility_limit: None,
            include_floating_decomposition: false,
        };

        let df = schedule
            .to_period_dataframe(&periods, &market, "USD-OIS", options)
            .unwrap();
        // Find PVs aligned with input cashflows
        // Historical row should be 0.0 PV; future row should be amount * DF
        assert_eq!(df.pvs.len(), 2);
        assert!((df.pvs[0] - 0.0).abs() < 1e-12);
        assert!((df.pvs[1] - 200.0 * df.discount_factors[1]).abs() < 1e-12);
    }
}

