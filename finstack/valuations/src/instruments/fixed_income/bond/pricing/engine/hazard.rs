//! Hazard-rate (intensity) bond pricer with fractional recovery of par (FRP).
//!
//! This engine prices defaultable bonds using a reduced-form hazard-rate model
//! with piecewise-constant hazard curve and **fractional recovery of par**.
//!
//! Let:
//! - `D(as_of, t)` be the risk-free discount factor from valuation date to t.
//! - `S(t)` be the survival probability from the hazard curve.
//! - `R` be the recovery rate (fraction of outstanding notional).
//! - `CF_i` be signed canonical schedule cashflows (coupons + principal) at dates `T_i`.
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

use super::super::super::types::Bond;

/// Hazard-rate bond pricing engine using FRP and `HazardCurve`.
///
/// This engine prices defaultable bonds using a reduced-form hazard-rate model
/// with fractional recovery of par (FRP). It gracefully falls back to risk-free
/// pricing if no hazard curve is available in the market context.
///
/// # Examples
///
/// Use `SimpleBondHazardPricer` for public API access to hazard-rate pricing:
///
/// ```rust,ignore
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::pricer::{Pricer, PricerRegistry};
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// let bond = Bond::example().unwrap();
/// let market = MarketContext::new();
/// let as_of = date!(2024-01-15);
///
/// // Register and use hazard pricer via registry
/// let registry = PricerRegistry::default();
/// let result = registry.get_price(&bond, &market, as_of)?;
/// ```
///
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

    /// Build pricing cashflows and the full internal schedule.
    fn build_schedules(
        bond: &Bond,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<(Vec<(Date, Money)>, CashFlowSchedule)> {
        let flows = bond.pricing_dated_cashflows(market, as_of)?;
        let schedule = bond.full_cashflow_schedule(market)?;
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
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::engine::hazard::HazardBondEngine;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example().unwrap();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pv = HazardBondEngine::price(&bond, &market, as_of)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub(crate) fn price(bond: &Bond, market: &MarketContext, as_of: Date) -> Result<Money> {
        Ok(Money::new(
            Self::price_raw(bond, market, as_of)?,
            bond.notional.currency(),
        ))
    }

    /// Price a bond using a hazard curve and return the unrounded PV.
    pub(crate) fn price_raw(bond: &Bond, market: &MarketContext, as_of: Date) -> Result<f64> {
        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        // Resolve discount curve
        let disc = market.get_discount(&bond.discount_curve_id)?;

        // Resolve hazard curve; if not found, fall back to discount-only pricing.
        // We call BondEngine::price directly (not bond.value_raw()) to avoid
        // recursion since Bond::value now routes here when credit_curve_id is set.
        let hazard = match Self::resolve_hazard_curve(bond, market) {
            Some(h) => h,
            None => {
                return super::discount::BondEngine::price(bond, market, as_of).map(|m| m.amount());
            }
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

        // Include dates where the outstanding balance changes (PIK
        // capitalizations, amortization) even when they don't produce
        // signed canonical schedule cashflows, so the recovery leg tracks the correct
        // notional at each interval boundary.
        for cf in &schedule.flows {
            if cf.date > as_of && matches!(cf.kind, CFKind::PIK | CFKind::Amortization) {
                dates.push(cf.date);
            }
        }

        dates.sort();
        dates.dedup();

        // No future cashflows after as_of → PV is zero.
        if dates.is_empty() {
            return Ok(0.0);
        }

        dates.insert(0, as_of);

        // Discount factors relative to as_of for correct PV anchoring.
        let mut dfs = Vec::with_capacity(dates.len());
        for d in &dates {
            let df_rel = disc.df_between_dates(as_of, *d)?;
            dfs.push(df_rel);
        }

        // Survival probabilities from hazard curve at the grid dates.
        // Renormalize to conditional survival Q(as_of, T) = S(T) / S(as_of)
        // so that the PV is correct even when the hazard curve's base date
        // differs from the valuation date (e.g., yesterday's curve reused today).
        let surv_raw = hazard.survival_at_dates(&dates)?;
        if surv_raw.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        // Check if already defaulted by as_of
        let s0 = surv_raw[0].clamp(0.0, 1.0);
        if s0 <= 0.0 {
            // Already defaulted by as_of; no future value.
            return Ok(0.0);
        }
        // Conditional survival: Q(as_of, T_i) = S(T_i) / S(as_of)
        let surv: Vec<f64> = surv_raw.iter().map(|s| (s / s0).clamp(0.0, 1.0)).collect();

        // Alive leg: survival-weighted PV of signed canonical schedule coupons and principal.
        // Use Kahan summation from finstack-core for numerical stability.
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
        let pv_cf = kahan_sum(pv_values);

        // Recovery leg: FRP on outstanding notional.
        // Outstanding tracks amortization (down) and PIK capitalizations (up)
        // so that recovery is computed on the correct accreted balance.
        let mut pv_rec = 0.0;
        if recovery > 0.0 {
            let mut full_flows = schedule.flows.clone();
            full_flows.sort_by_key(|cf| cf.date);

            let mut outstanding = schedule.notional.initial.amount();
            let mut future_balance_delta = std::collections::BTreeMap::<Date, f64>::new();

            for cf in &full_flows {
                let amt = cf.amount.amount();
                if amt <= 0.0 {
                    continue;
                }
                match cf.kind {
                    CFKind::Amortization | CFKind::Notional => {
                        if cf.date <= as_of {
                            outstanding -= amt;
                        } else {
                            *future_balance_delta.entry(cf.date).or_insert(0.0) -= amt;
                        }
                    }
                    CFKind::PIK => {
                        if cf.date <= as_of {
                            outstanding += amt;
                        } else {
                            *future_balance_delta.entry(cf.date).or_insert(0.0) += amt;
                        }
                    }
                    _ => {}
                }
            }

            let mut current_outstanding = outstanding.max(0.0);

            for k in 1..dates.len() {
                let n_prev = current_outstanding.max(0.0);
                if n_prev > 0.0 {
                    let delta_s = (surv[k - 1] - surv[k]).max(0.0);
                    if delta_s > 0.0 {
                        // Use midpoint of interval for recovery timing (better approximation).
                        // Geometric mean of endpoint DFs equals exact midpoint DF under
                        // continuous compounding: sqrt(df(t_start) * df(t_end)) = df((t_start + t_end)/2).
                        let df_midpoint = (dfs[k - 1] * dfs[k]).sqrt();
                        pv_rec += recovery * n_prev * delta_s * df_midpoint;
                    }
                }
                // Apply balance changes: amortization/redemption (negative delta)
                // and PIK capitalizations (positive delta).
                if let Some(delta) = future_balance_delta.remove(&dates[k]) {
                    current_outstanding = (current_outstanding + delta).max(0.0);
                }
            }
        }

        Ok(pv_cf + pv_rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::CouponType;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::instruments::common_impl::traits::Instrument;
    use crate::instruments::fixed_income::bond::pricing::engine::discount::BondEngine;
    use crate::instruments::fixed_income::bond::CashflowSpec;
    use crate::metrics::sensitivities::config::STANDARD_BUCKETS_YEARS;
    use crate::metrics::{standard_registry, MetricContext, MetricId};
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
            .issue_date(issue)
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
            .interp(InterpStyle::LogLinear)
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

    fn build_pik_bond(issue: Date, maturity: Date) -> Bond {
        let mut spec = CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Act365F);
        if let CashflowSpec::Fixed(ref mut inner) = spec {
            inner.coupon_type = CouponType::PIK;
        }
        Bond::builder()
            .id("TEST_PIK_BOND_HAZARD".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue_date(issue)
            .maturity(maturity)
            .cashflow_spec(spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(Some(CurveId::new("USD-CREDIT")))
            .pricing_overrides(crate::instruments::PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("PIK bond builder should succeed in hazard engine test")
    }

    #[test]
    fn hazard_zero_matches_discounting_for_plain_bond() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let disc = build_flat_discount(issue);
        let hazard_zero = build_flat_hazard("USD-CREDIT", issue, 0.0, 0.4);

        let market = MarketContext::new().insert(disc).insert(hazard_zero);

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
            .insert(build_flat_discount(issue))
            .insert(hazard_low);
        let market_high = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(hazard_high);

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
            .insert(build_flat_discount(issue))
            .insert(hazard_low_recovery);
        let market_high_r = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(hazard_high_recovery);

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
            .insert(build_flat_discount(issue))
            .insert(hazard);

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

        // Parallel CS01 should be nonzero since Bond::value now uses the hazard
        // engine when credit_curve_id is set.
        let cs01 = ctx.computed.get(&MetricId::Cs01).copied().unwrap_or(0.0);
        assert!(
            cs01.abs() > 1e-6,
            "CS01 should be nonzero for bond with hazard curve; got {}",
            cs01
        );

        // Bucketed CS01 series should be stored with standard bucket count.
        if let Some(series) = ctx.get_series(&MetricId::BucketedCs01) {
            let buckets = STANDARD_BUCKETS_YEARS;
            assert_eq!(
                series.len(),
                buckets.len(),
                "Bucketed CS01 series length should match standard bucket count"
            );
        }
    }

    #[ignore = "slow"]
    #[test]
    fn quote_engine_works_for_bond_with_hazard_curve() {
        use crate::instruments::fixed_income::bond::pricing::quote_conversions::{
            compute_quotes, BondQuoteInput,
        };
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_test_bond(issue, maturity);
        let market = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(build_flat_hazard("USD-CREDIT", issue, 0.02, 0.4));

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

    #[test]
    fn hazard_zero_matches_discounting_for_pik_bond() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_pik_bond(issue, maturity);
        let disc = build_flat_discount(issue);
        let hazard_zero = build_flat_hazard("USD-CREDIT", issue, 0.0, 0.4);

        let market = MarketContext::new().insert(disc).insert(hazard_zero);

        let pv_rf = BondEngine::price(&bond, &market, issue).expect("RF price should succeed");
        let pv_haz =
            HazardBondEngine::price(&bond, &market, issue).expect("Hazard price should succeed");

        let diff = (pv_rf.amount() - pv_haz.amount()).abs();
        assert!(
            diff < 1e-6,
            "PIK hazard price with zero intensity should match risk-free price; \
             pv_rf={}, pv_haz={}, diff={}",
            pv_rf.amount(),
            pv_haz.amount(),
            diff
        );
    }

    #[test]
    fn pik_recovery_uses_accreted_notional() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let cash_bond = build_test_bond(issue, maturity);
        let pik_bond = build_pik_bond(issue, maturity);

        let lambda = 0.05;
        let recovery = 0.4;
        let market = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(build_flat_hazard("USD-CREDIT", issue, lambda, recovery));

        let pv_cash =
            HazardBondEngine::price(&cash_bond, &market, issue).expect("Cash bond price succeeds");
        let pv_pik =
            HazardBondEngine::price(&pik_bond, &market, issue).expect("PIK bond price succeeds");

        // With zero recovery, isolate the alive-leg difference.
        let market_no_rec = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(build_flat_hazard("USD-CREDIT", issue, lambda, 0.0));

        let pv_cash_norec = HazardBondEngine::price(&cash_bond, &market_no_rec, issue)
            .expect("Cash bond no-recovery price");
        let pv_pik_norec = HazardBondEngine::price(&pik_bond, &market_no_rec, issue)
            .expect("PIK bond no-recovery price");

        // Recovery benefit = PV(with recovery) - PV(without recovery)
        let rec_benefit_cash = pv_cash.amount() - pv_cash_norec.amount();
        let rec_benefit_pik = pv_pik.amount() - pv_pik_norec.amount();

        // PIK accretes notional above par, so its recovery base is larger on
        // average → the PIK recovery benefit must exceed the cash bond's.
        assert!(
            rec_benefit_pik > rec_benefit_cash,
            "PIK recovery benefit ({:.2}) should exceed cash bond recovery benefit ({:.2}) \
             because PIK accretes notional above par",
            rec_benefit_pik,
            rec_benefit_cash
        );
    }

    #[test]
    fn pik_higher_hazard_produces_lower_price() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let bond = build_pik_bond(issue, maturity);

        let market_low = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(build_flat_hazard("USD-CREDIT", issue, 0.01, 0.4));
        let market_high = MarketContext::new()
            .insert(build_flat_discount(issue))
            .insert(build_flat_hazard("USD-CREDIT", issue, 0.05, 0.4));

        let pv_low =
            HazardBondEngine::price(&bond, &market_low, issue).expect("Low hazard price succeeds");
        let pv_high = HazardBondEngine::price(&bond, &market_high, issue)
            .expect("High hazard price succeeds");

        assert!(
            pv_high.amount() < pv_low.amount(),
            "PIK price with higher hazard should be lower (pv_high={}, pv_low={})",
            pv_high.amount(),
            pv_low.amount()
        );
    }
}
