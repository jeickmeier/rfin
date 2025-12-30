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
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Compare two amounts using relative epsilon for floating-point tolerance.
///
/// Uses a relative tolerance scaled by the magnitude of the values, with a
/// minimum absolute tolerance of 1e-9 for values near zero.
fn amounts_approx_equal(a: f64, b: f64) -> bool {
    let max_abs = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() < max_abs * 1e-9
}

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
    /// Period start dates
    pub start_dates: Vec<Date>,
    /// Period end dates
    pub end_dates: Vec<Date>,
    /// Payment dates (potentially adjusted for business days)
    pub pay_dates: Vec<Date>,
    /// Reset dates for floating rate coupons (if applicable)
    pub reset_dates: Vec<Option<Date>>,
    /// Cashflow types (coupon, amortization, fee, etc.)
    pub cf_types: Vec<CFKind>,
    /// Currencies for each cashflow
    pub currencies: Vec<Currency>,
    /// Outstanding notional amounts
    pub notionals: Vec<Option<f64>>,
    /// Undrawn notional amounts (for committed facilities)
    pub undrawn_notionals: Option<Vec<Option<f64>>>,
    /// Year fractions for each period (time between dates in years)
    pub yr_fraqs: Vec<f64>,
    /// Accrual factors (day count convention applied)
    pub accrual_factors: Vec<f64>,
    /// Calendar days in each period
    pub days: Vec<i64>,
    /// Cashflow amounts (coupons, principal, fees)
    pub amounts: Vec<f64>,
    /// Interest rates for each period
    pub rates: Vec<f64>,
    /// Discount factors from base date to payment dates
    pub discount_factors: Vec<f64>,
    /// Survival probabilities (if credit risk modeled)
    pub survival_probs: Option<Vec<Option<f64>>>,
    /// Present values of cashflows
    pub pvs: Vec<f64>,
    /// Unfunded amounts (drawn commitment minus outstanding)
    pub unfunded_amounts: Option<Vec<Option<f64>>>,
    /// Total commitment amounts per period
    pub commitment_amounts: Option<Vec<Option<f64>>>,
    /// Base forward rates for floating coupons (if decomposed)
    pub base_rates: Option<Vec<Option<f64>>>,
    /// Spread over base rates for floating coupons (if decomposed)
    pub spreads: Option<Vec<Option<f64>>>,
}

impl PeriodDataFrame {
    /// Create a DataFrame with preallocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            start_dates: Vec::with_capacity(capacity),
            end_dates: Vec::with_capacity(capacity),
            pay_dates: Vec::with_capacity(capacity),
            reset_dates: Vec::with_capacity(capacity),
            cf_types: Vec::with_capacity(capacity),
            currencies: Vec::with_capacity(capacity),
            notionals: Vec::with_capacity(capacity),
            undrawn_notionals: None,
            yr_fraqs: Vec::with_capacity(capacity),
            accrual_factors: Vec::with_capacity(capacity),
            days: Vec::with_capacity(capacity),
            amounts: Vec::with_capacity(capacity),
            rates: Vec::with_capacity(capacity),
            discount_factors: Vec::with_capacity(capacity),
            survival_probs: None,
            pvs: Vec::with_capacity(capacity),
            unfunded_amounts: None,
            commitment_amounts: None,
            base_rates: None,
            spreads: None,
        }
    }

    /// Clear all columns while preserving allocations.
    pub fn clear(&mut self) {
        self.start_dates.clear();
        self.end_dates.clear();
        self.pay_dates.clear();
        self.reset_dates.clear();
        self.cf_types.clear();
        self.currencies.clear();
        self.notionals.clear();
        if let Some(undrawn) = self.undrawn_notionals.as_mut() {
            undrawn.clear();
        }
        self.yr_fraqs.clear();
        self.accrual_factors.clear();
        self.days.clear();
        self.amounts.clear();
        self.rates.clear();
        self.discount_factors.clear();
        if let Some(survival) = self.survival_probs.as_mut() {
            survival.clear();
        }
        self.pvs.clear();
        if let Some(unfunded) = self.unfunded_amounts.as_mut() {
            unfunded.clear();
        }
        if let Some(commitment) = self.commitment_amounts.as_mut() {
            commitment.clear();
        }
        if let Some(base_rates) = self.base_rates.as_mut() {
            base_rates.clear();
        }
        if let Some(spreads) = self.spreads.as_mut() {
            spreads.clear();
        }
    }

}

impl CashFlowSchedule {
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
        let mut out = PeriodDataFrame::with_capacity(self.flows.len());
        self.to_period_dataframe_into(periods, market, discount_curve_id, options, &mut out)?;
        Ok(out)
    }

    /// Period-aligned DataFrame-like export into an existing buffer.
    ///
    /// This reuses allocations in `out` when possible and preserves the
    /// input ordering of cashflows.
    pub(crate) fn to_period_dataframe_into(
        &self,
        periods: &[Period],
        market: &MarketContext,
        discount_curve_id: &str,
        options: PeriodDataFrameOptions<'_>,
        out: &mut PeriodDataFrame,
    ) -> finstack_core::Result<()> {
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
        let facility_limit = options.facility_limit.or(self.meta.facility_limit);
        let capacity = self.flows.len();
        let include_floating = options.include_floating_decomposition;
        let has_facility = facility_limit.is_some();

        out.clear();
        out.start_dates.reserve(capacity);
        out.end_dates.reserve(capacity);
        out.pay_dates.reserve(capacity);
        out.reset_dates.reserve(capacity);
        out.cf_types.reserve(capacity);
        out.currencies.reserve(capacity);
        out.notionals.reserve(capacity);
        out.yr_fraqs.reserve(capacity);
        out.accrual_factors.reserve(capacity);
        out.days.reserve(capacity);
        out.amounts.reserve(capacity);
        out.rates.reserve(capacity);
        out.discount_factors.reserve(capacity);
        out.pvs.reserve(capacity);

        out.undrawn_notionals = if has_facility {
            let mut vec = out.undrawn_notionals.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };
        out.survival_probs = if has_hazard {
            let mut vec = out.survival_probs.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };
        out.unfunded_amounts = if has_facility {
            let mut vec = out.unfunded_amounts.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };
        out.commitment_amounts = if has_facility {
            let mut vec = out.commitment_amounts.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };
        out.base_rates = if include_floating {
            let mut vec = out.base_rates.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };
        out.spreads = if include_floating {
            let mut vec = out.spreads.take().unwrap_or_default();
            vec.clear();
            vec.reserve(capacity);
            Some(vec)
        } else {
            None
        };

        // Track outstanding drawn balance for Notional column
        let mut outstanding = self.notional.initial;

        // Identify the first date in the schedule (issue date) for initial funding detection
        let first_date = self.flows.first().map(|cf| cf.date);

        for cf in &self.flows {
            // Find containing period (inclusive end)
            let period_opt = periods
                .iter()
                .find(|p| cf.date >= p.start && cf.date <= p.end);
            let Some(period) = period_opt else {
                continue;
            };

            // Outstanding before this cashflow
            let outstanding_pre = outstanding;

            // Detect initial funding notional flow (negative, equal to -notional.initial on first date)
            // This is already accounted for in notional.initial, so we skip it to avoid double-counting.
            let is_initial_funding = cf.kind == CFKind::Notional
                && first_date == Some(cf.date)
                && cf.amount.amount() < 0.0
                && amounts_approx_equal(cf.amount.amount().abs(), self.notional.initial.amount());

            match cf.kind {
                CFKind::Amortization => {
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                CFKind::PIK => {
                    outstanding = outstanding.checked_add(cf.amount)?;
                }
                CFKind::Notional if !is_initial_funding => {
                    // Draws are negative, repays are positive from lender perspective
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                _ => {}
            }

            // Basic columns
            out.start_dates.push(period.start);
            out.end_dates.push(period.end);
            out.pay_dates.push(cf.date);
            out.reset_dates.push(cf.reset_date);
            out.cf_types.push(cf.kind);
            out.currencies.push(cf.amount.currency());
            out.amounts.push(cf.amount.amount());
            out.accrual_factors.push(cf.accrual_factor);
            out.rates.push(cf.rate.unwrap_or(0.0));

            // Notional balances for interest/fee-like rows
            let (notional_drawn, notional_undrawn) = if matches!(
                cf.kind,
                CFKind::Fixed
                    | CFKind::Stub
                    | CFKind::FloatReset
                    | CFKind::CommitmentFee
                    | CFKind::UsageFee
                    | CFKind::FacilityFee
            ) || cf.accrual_factor > 0.0
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
            out.notionals.push(notional_drawn);
            if let Some(ref mut undrawn) = out.undrawn_notionals {
                undrawn.push(notional_undrawn);
            }

            // YrFraq and Days
            let yr_fraq = dc
                .year_fraction(period.start, cf.date, DayCountCtx::default())
                .unwrap_or(0.0);
            out.yr_fraqs.push(yr_fraq);
            out.days.push((cf.date - period.start).whole_days());

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
            out.discount_factors.push(df);

            // Survival probability
            if let (Some(h), Some(spv)) = (hazard_arc_opt.as_ref(), out.survival_probs.as_mut()) {
                spv.push(Some(h.sp(t)));
            }

            // PV
            let sp_mult = if let Some(ref spv) = out.survival_probs {
                spv.last().copied().flatten().unwrap_or(1.0)
            } else {
                1.0
            };
            let pv_amt = if cf.date > base {
                cf.amount.amount() * df * sp_mult
            } else {
                0.0
            };
            out.pvs.push(pv_amt);

            // Unfunded and commitment amounts
            if let Some(limit) = facility_limit.as_ref() {
                if let Some(ref mut unfunded_vec) = out.unfunded_amounts {
                    if limit.currency() == cf.amount.currency() {
                        let val = (limit.amount() - outstanding_pre.amount()).max(0.0);
                        unfunded_vec.push(Some(val));
                    } else {
                        unfunded_vec.push(None);
                    }
                }
                if let Some(ref mut commit_vec) = out.commitment_amounts {
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
            if let Some(ref mut br) = out.base_rates {
                br.push(base_rate_opt);
            }
            if let Some(ref mut sp) = out.spreads {
                sp.push(spread_opt);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::schedule::{CashFlowMeta, CashFlowSchedule};
    use crate::cashflow::builder::Notional;
    use finstack_core::cashflow::CashFlow;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Period, PeriodId};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), day)
            .expect("Valid test date")
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
            meta: CashFlowMeta::default(),
        };

        // Market context with flat discount curve (df = 1.0)
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (30.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");
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
            .expect("PeriodDataFrame creation should succeed in test");
        // Find PVs aligned with input cashflows
        // Historical row should be 0.0 PV; future row should be amount * DF
        assert_eq!(df.pvs.len(), 2);
        assert!((df.pvs[0] - 0.0).abs() < 1e-12);
        assert!((df.pvs[1] - 200.0 * df.discount_factors[1]).abs() < 1e-12);
    }

    #[test]
    fn dataframe_does_not_double_count_initial_funding() {
        // Build a schedule that includes the initial funding notional flow
        // (like what CashFlowBuilder produces).
        // The initial funding is a NEGATIVE Notional flow on the first date.
        let issue = d(2025, 1, 15);
        let initial_amount = 1_000_000.0;
        let flows = vec![
            // Initial funding (negative from lender perspective - money out)
            CashFlow {
                date: issue,
                reset_date: None,
                amount: Money::new(-initial_amount, Currency::USD),
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
            },
            // First coupon
            CashFlow {
                date: d(2025, 4, 15),
                reset_date: None,
                amount: Money::new(12_500.0, Currency::USD), // 5% quarterly
                kind: CFKind::Fixed,
                accrual_factor: 0.25,
                rate: Some(0.05),
            },
        ];
        let schedule = CashFlowSchedule {
            flows,
            notional: Notional::par(initial_amount, Currency::USD),
            day_count: DayCount::Act365F,
            meta: CashFlowMeta::default(),
        };

        // Market context
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed");
        let market = MarketContext::new().insert_discount(curve);

        let periods = vec![
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
        ];
        let options = PeriodDataFrameOptions {
            as_of: Some(issue),
            day_count: Some(DayCount::Act365F),
            ..Default::default()
        };

        let df = schedule
            .to_period_dataframe(&periods, &market, "USD-OIS", options)
            .expect("PeriodDataFrame creation should succeed");

        // The coupon flow's notional should be the original notional (1M),
        // NOT double-counted (2M) due to the initial funding flow.
        // The coupon is the second row (index 1).
        assert_eq!(df.cf_types.len(), 2);
        assert_eq!(df.cf_types[0], CFKind::Notional);
        assert_eq!(df.cf_types[1], CFKind::Fixed);

        // The notional for the coupon row should be 1M, not 2M
        let coupon_notional = df.notionals[1].expect("Coupon should have notional");
        assert!(
            (coupon_notional - initial_amount).abs() < 1e-6,
            "Expected notional {} but got {} (double-counting bug if ~2M)",
            initial_amount,
            coupon_notional
        );
    }

    #[test]
    fn dataframe_omits_undrawn_notionals_without_facility_limit() {
        let base = d(2025, 4, 1);
        let flows = vec![CashFlow {
            date: d(2025, 5, 15),
            reset_date: None,
            amount: Money::new(200.0, Currency::USD),
            kind: CFKind::Fixed,
            accrual_factor: 0.25,
            rate: None,
        }];
        let schedule = CashFlowSchedule {
            flows,
            notional: Notional::par(1_000.0, Currency::USD),
            day_count: DayCount::Act365F,
            meta: CashFlowMeta::default(),
        };

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (30.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");
        let market = MarketContext::new().insert_discount(curve);

        let options = PeriodDataFrameOptions {
            as_of: Some(base),
            day_count: Some(DayCount::Act365F),
            ..Default::default()
        };

        let df = schedule
            .to_period_dataframe(&quarters_2025(), &market, "USD-OIS", options)
            .expect("PeriodDataFrame creation should succeed in test");

        assert!(df.undrawn_notionals.is_none());
    }
}
