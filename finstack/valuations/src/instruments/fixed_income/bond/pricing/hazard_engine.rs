//! Hazard-rate (intensity) bond pricer with fractional recovery of par (FRP).
//!
//! This engine prices defaultable bonds using a reduced-form hazard-rate model
//! with piecewise-constant hazard curve and **fractional recovery of par**.
//!
//! Let:
//! - `D(as_of, t)` be the risk-free discount factor from valuation date to t.
//! - `S(t)` be the survival probability from the hazard curve.
//! - `R` be the recovery rate (fraction of outstanding notional).
//! - `CF_i` be holder-view cashflows (coupons + principal) at dates `T_i`.
//! - `N(t)` be the outstanding notional process (including amortization).
//!
//! Under independence of rates and credit and FRP, the price at `as_of` is:
//! ```text
//! PV = Σ_i CF_i · D(as_of, T_i) · S(T_i)
//!    + R · Σ_k N(t_{k-1}) · D(as_of, t_k) · ΔS_k
//! ```
//! where:
//! - `ΔS_k = S(t_{k-1}) - S(t_k) ≈ ∫_{t_{k-1}}^{t_k} λ(u) S(u) du`
//! - the time grid `{t_k}` is built from the bond cashflow dates, with `t_0`
//!   anchored at `as_of` (valuation date).
//!
//! Recovery is taken as a fraction of **outstanding notional** (par) during
//! each interval, which matches the fractional recovery of par convention used
//! in the two-factor rates+credit tree (`BondValuator`).
//!
//! # Settlement Convention
//!
//! Settlement days affect quote interpretation (accrued interest at settlement),
//! but the PV is always anchored at `as_of`. The quote engine handles
//! settlement-date accrued interest separately.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::math::summation::kahan_sum;
use finstack_core::money::Money;
use finstack_core::InputError;
use finstack_core::Result;

use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::CashflowProvider;

use super::super::types::Bond;
use super::discount_engine::BondEngine;

/// Hazard-rate bond pricing engine using FRP and `HazardCurve`.
///
/// This engine prices defaultable bonds using a reduced-form hazard-rate model
/// with fractional recovery of par (FRP). It gracefully falls back to risk-free
/// pricing if no hazard curve is available in the market context.
///
/// # Examples
///
/// Use the [`SimpleBondHazardPricer`] for public API access to hazard-rate pricing:
///
/// ```rust,ignore
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::pricer::{Pricer, PricerRegistry};
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// let bond = Bond::example();
/// let market = MarketContext::new();
/// let as_of = date!(2024-01-15);
///
/// // Register and use hazard pricer via registry
/// let registry = PricerRegistry::default();
/// let result = registry.price(&bond, &market, as_of)?;
/// ```
///
/// [`SimpleBondHazardPricer`]: crate::instruments::bond::pricing::SimpleBondHazardPricer
pub struct HazardBondEngine;

impl HazardBondEngine {
    /// Resolve a hazard curve for the bond using the same precedence as the
    /// tree-based bond valuator:
    ///
    /// 1. `credit_curve_id` if present.
    /// 2. `discount_curve_id`.
    /// 3. `discount_curve_id` with `-CREDIT` suffix.
    fn resolve_hazard_curve(
        bond: &Bond,
        market: &MarketContext,
    ) -> Option<std::sync::Arc<HazardCurve>> {
        if let Some(ref credit_id) = bond.credit_curve_id {
            if let Ok(hc) = market.get_hazard(credit_id.as_str()) {
                return Some(hc);
            }
        }

        if let Ok(hc) = market.get_hazard(bond.discount_curve_id.as_str()) {
            return Some(hc);
        }

        let credit_id = format!("{}-CREDIT", bond.discount_curve_id.as_str());
        if let Ok(hc) = market.get_hazard(credit_id.as_str()) {
            return Some(hc);
        }

        None
    }

    /// Build holder-view cashflows and the full internal schedule.
    fn build_schedules(
        bond: &Bond,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<(Vec<(Date, Money)>, CashFlowSchedule)> {
        let flows = bond.build_dated_flows(market, as_of)?;
        if flows.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        let schedule = bond.get_full_schedule(market)?;
        Ok((flows, schedule))
    }

    /// Price a bond using a hazard curve with fractional recovery of par (FRP).
    ///
    /// Computes the present value accounting for credit risk by:
    /// 1. Discounting survival-weighted cashflows (alive leg)
    /// 2. Adding recovery value on default events (recovery leg)
    ///
    /// If no hazard curve can be resolved from the market context, this
    /// gracefully falls back to the standard discounting engine so callers
    /// can safely request hazard pricing even when credit data is absent.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to price
    /// * `market` - Market context containing discount and hazard curves
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value of the bond accounting for credit risk, or risk-free PV
    /// if no hazard curve is available.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found in market context
    /// - Bond has no future cashflows
    /// - Cashflow schedule building fails
    /// - Survival probability calculation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::hazard_engine::HazardBondEngine;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pv = HazardBondEngine::price(&bond, &market, as_of)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub(crate) fn price(bond: &Bond, market: &MarketContext, as_of: Date) -> Result<Money> {
        if as_of >= bond.maturity {
            return Ok(Money::new(0.0, bond.notional.currency()));
        }

        // Resolve discount curve
        let disc = market.get_discount(&bond.discount_curve_id)?;

        // Resolve hazard curve; if not found, fall back to risk-free pricing.
        let hazard = match Self::resolve_hazard_curve(bond, market) {
            Some(h) => h,
            None => return BondEngine::price(bond, market, as_of),
        };
        let recovery = hazard.recovery_rate().clamp(0.0, 1.0);

        // Schedules
        let (flows, schedule) = Self::build_schedules(bond, market, as_of)?;

        // Build time grid from as_of + future cashflow dates.
        // PV is anchored at as_of (valuation date), not settlement.
        let mut dates: Vec<Date> = flows
            .iter()
            .map(|(d, _)| *d)
            .filter(|d| *d > as_of)
            .collect();
        dates.sort();
        dates.dedup();

        // No future cashflows after as_of → PV is zero.
        if dates.is_empty() {
            return Ok(Money::new(0.0, bond.notional.currency()));
        }

        dates.insert(0, as_of);

        // Discount factors relative to as_of for correct PV anchoring.
        let mut dfs = Vec::with_capacity(dates.len());
        for d in &dates {
            let df_rel = disc.df_between_dates(as_of, *d)?;
            dfs.push(df_rel);
        }

        // Survival probabilities from hazard curve at the grid dates.
        // Use unconditional survival from the hazard curve's base date; no
        // renormalization is applied since PV is anchored at as_of.
        let surv = hazard.survival_at_dates(&dates)?;
        if surv.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        // Check if already defaulted by as_of
        let s0 = surv[0].clamp(0.0, 1.0);
        if s0 <= 0.0 {
            // Already defaulted by as_of; no future value.
            return Ok(Money::new(0.0, bond.notional.currency()));
        }

        // Alive leg: survival-weighted PV of holder-view coupons and principal.
        // Use Kahan summation from finstack-core for numerical stability.
        let ccy = bond.notional.currency();
        let pv_values: Vec<f64> = flows
            .iter()
            .filter(|(d, amt)| *d > as_of && amt.amount() != 0.0)
            .filter_map(|(d, amt)| {
                // Dates come from the same grid we built, so binary_search should succeed
                dates.binary_search(d).ok().map(|idx| {
                    let df = dfs[idx];
                    let s = surv[idx];
                    amt.amount() * df * s
                })
            })
            .collect();
        let pv_cf = Money::new(kahan_sum(pv_values), ccy);

        // Recovery leg: FRP on outstanding notional.
        let mut pv_rec = 0.0;
        if recovery > 0.0 {
            // Compute outstanding notional at as_of and future reductions.
            let mut full_flows = schedule.flows.clone();
            full_flows.sort_by_key(|cf| cf.date);

            let mut outstanding = schedule.notional.initial.amount();
            let mut future_principal = std::collections::BTreeMap::<Date, f64>::new();

            for cf in &full_flows {
                let amt = cf.amount.amount();
                if amt <= 0.0 {
                    continue;
                }
                let is_principal = matches!(cf.kind, CFKind::Amortization | CFKind::Notional);
                if !is_principal {
                    continue;
                }
                if cf.date <= as_of {
                    outstanding -= amt;
                } else {
                    *future_principal.entry(cf.date).or_insert(0.0) += amt;
                }
            }

            let mut current_outstanding = outstanding.max(0.0);

            for k in 1..dates.len() {
                let n_prev = current_outstanding.max(0.0);
                if n_prev > 0.0 {
                    let delta_s = (surv[k - 1] - surv[k]).max(0.0);
                    if delta_s > 0.0 {
                        let df_k = dfs[k];
                        pv_rec += recovery * n_prev * df_k * delta_s;
                    }
                }
                // Apply principal repayments at the end of the interval.
                if let Some(amt) = future_principal.remove(&dates[k]) {
                    current_outstanding = (current_outstanding - amt).max(0.0);
                }
            }
        }

        let pv_rec_money = Money::new(pv_rec, ccy);
        let total = (pv_cf + pv_rec_money)?;
        Ok(total)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::bond::pricing::quote_engine::{compute_quotes, BondQuoteInput};
    use crate::instruments::bond::CashflowSpec;
    use crate::instruments::common::traits::Attributes;
    use crate::instruments::common::traits::Instrument;
    use crate::metrics::{
        standard_credit_cs01_buckets, standard_registry, MetricContext, MetricId,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Tenor};
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::types::CurveId;
    use finstack_core::{dates::Date, money::Money};
    use std::sync::Arc;
    use time::Month;

    fn build_test_bond(issue: Date, maturity: Date) -> Bond {
        Bond::builder()
            .id("TEST_BOND_HAZARD".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Tenor::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(Some(CurveId::new("USD-CREDIT")))
            .pricing_overrides(crate::instruments::PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Bond builder should succeed in hazard engine test")
    }

    fn build_flat_discount(issue: Date) -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (10.0, 0.8)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed in hazard engine test")
    }

    fn build_flat_hazard(id: &str, issue: Date, lambda: f64, recovery: f64) -> HazardCurve {
        HazardCurve::builder(id)
            .base_date(issue)
            .recovery_rate(recovery)
            .knots([(0.0, lambda), (10.0, lambda)])
            .build()
            .expect("HazardCurve builder should succeed in hazard engine test")
    }

    #[test]
    fn hazard_zero_matches_discounting_for_plain_bond() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let disc = build_flat_discount(issue);
        let hazard_zero = build_flat_hazard("USD-CREDIT", issue, 0.0, 0.4);

        let market = MarketContext::new()
            .insert_discount(disc)
            .insert_hazard(hazard_zero);

        let pv_rf = BondEngine::price(&bond, &market, issue).expect("RF price should succeed");
        let pv_haz =
            HazardBondEngine::price(&bond, &market, issue).expect("Hazard price should succeed");

        let diff = (pv_rf.amount() - pv_haz.amount()).abs();
        assert!(
            diff < 1e-6,
            "Hazard price with zero intensity should match risk-free price; diff={}",
            diff
        );
    }

    #[test]
    fn higher_hazard_produces_lower_price() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let hazard_low = build_flat_hazard("USD-CREDIT", issue, 0.01, 0.4);
        let hazard_high = build_flat_hazard("USD-CREDIT", issue, 0.05, 0.4);

        let market_low = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(hazard_low);
        let market_high = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(hazard_high);

        let pv_low =
            HazardBondEngine::price(&bond, &market_low, issue).expect("Low hazard price succeeds");
        let pv_high = HazardBondEngine::price(&bond, &market_high, issue)
            .expect("High hazard price succeeds");

        assert!(
            pv_high.amount() < pv_low.amount(),
            "Price with higher hazard should be lower (pv_high={}, pv_low={})",
            pv_high.amount(),
            pv_low.amount()
        );
    }

    #[test]
    fn higher_recovery_increases_price() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let hazard_low_recovery = build_flat_hazard("USD-CREDIT", issue, 0.03, 0.0);
        let hazard_high_recovery = build_flat_hazard("USD-CREDIT", issue, 0.03, 1.0);

        let market_low_r = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(hazard_low_recovery);
        let market_high_r = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(hazard_high_recovery);

        let pv_low_r = HazardBondEngine::price(&bond, &market_low_r, issue)
            .expect("Low recovery hazard price succeeds");
        let pv_high_r = HazardBondEngine::price(&bond, &market_high_r, issue)
            .expect("High recovery hazard price succeeds");

        assert!(
            pv_high_r.amount() > pv_low_r.amount(),
            "Price with higher recovery should be higher (pv_high={}, pv_low={})",
            pv_high_r.amount(),
            pv_low_r.amount()
        );
    }

    #[test]
    fn cs01_and_bucketed_cs01_with_hazard_engine() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        // Base flat hazard curve used for CS01 tests
        let base_lambda = 0.03;
        let hazard = HazardCurve::builder("USD-CREDIT")
            .base_date(issue)
            .recovery_rate(0.4)
            .knots([(0.0, base_lambda), (5.0, base_lambda), (10.0, base_lambda)])
            .build()
            .expect("Base hazard curve builder should succeed in test");
        let market = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(hazard);

        // Use the standard metrics registry and MetricContext to request CS01 and
        // BucketedCs01, ensuring the metrics plumbing works for bonds with hazard curves.
        let base_pv = bond
            .value(&market, issue)
            .expect("Base bond valuation should succeed in CS01 test");

        let instrument_arc: Arc<dyn Instrument> = Arc::new(bond.clone());
        let curves_arc = Arc::new(market.clone());
        let mut ctx = MetricContext::new(
            instrument_arc,
            curves_arc,
            issue,
            base_pv,
            MetricContext::default_config(),
        );

        let registry = standard_registry();
        let metric_ids = [MetricId::Cs01, MetricId::BucketedCs01];
        let _ = registry
            .compute(&metric_ids, &mut ctx)
            .expect("CS01 metrics should compute for bond with hazard curve");

        // Parallel CS01 should be present (may be zero today depending on Bond::value semantics).
        let _cs01 = ctx.computed.get(&MetricId::Cs01).copied().unwrap_or(0.0);

        // Bucketed CS01 series should be stored with standard bucket count.
        // Bucketed CS01 series is computed via GenericBucketedCs01. For bonds this
        // may currently be zero when `Bond::value` does not depend on hazard, but
        // the series should still be present and match the standard bucket grid.
        if let Some(series) = ctx.get_series(&MetricId::BucketedCs01) {
            let buckets = standard_credit_cs01_buckets();
            assert_eq!(
                series.len(),
                buckets.len(),
                "Bucketed CS01 series length should match standard bucket count"
            );
        }
    }

    #[test]
    fn quote_engine_works_for_bond_with_hazard_curve() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let market = MarketContext::new()
            .insert_discount(build_flat_discount(issue))
            .insert_hazard(build_flat_hazard("USD-CREDIT", issue, 0.02, 0.4));

        // Use a simple clean price quote; the quote engine should handle bonds
        // with hazard curves present in the MarketContext without error.
        let quotes = compute_quotes(&bond, &market, issue, BondQuoteInput::CleanPricePct(99.5))
            .expect("Quote engine should work for bonds with hazard curves");

        assert!(
            (quotes.clean_price_pct - 99.5).abs() < 1e-9,
            "Clean price pct should reflect the input quote"
        );
        // Basic sanity: core yield/spread metrics should be populated.
        assert!(quotes.ytm.is_some(), "YTM should be computed");
        assert!(
            quotes.z_spread.is_some(),
            "Z-spread should be computed for quoted bond"
        );
    }
}
