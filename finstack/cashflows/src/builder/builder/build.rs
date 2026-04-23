//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Build the cashflow schedule without market curves.
    ///
    /// Equivalent to `build_with_curves(None)`. For floating-rate instruments
    /// that require projection curves, use [`build_with_curves`](Self::build_with_curves).
    #[deprecated(note = "use build_with_curves(None) or prepared()?.project(None)")]
    pub fn build(&self) -> finstack_core::Result<CashFlowSchedule> {
        self.prepared()?.project(None)
    }

    /// Build the cashflow schedule with optional market curves for floating rate projection.
    ///
    /// When curves are provided, floating rate coupons use forward rates:
    /// `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves, the fallback policy on each floating spec controls behavior
    /// (default: error; `SpreadOnly` uses just margin; `FixedRate(r)` uses a fixed index).
    ///
    /// # Caching pattern
    ///
    /// If you need to reprice the same instrument under many curve bumps
    /// (e.g., delta/vega grids or scenario sweeps), prefer calling
    /// [`prepared`](Self::prepared) once and then [`PreparedCashFlow::project`]
    /// repeatedly — the schedule compilation, date collection, and
    /// amortization setup are done once and reused.
    ///
    /// ```ignore
    /// let prepared = builder.prepared()?;
    /// let base    = prepared.project(Some(&market))?;
    /// let bumped  = prepared.project(Some(&bumped_market))?;
    /// ```
    pub fn build_with_curves(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
        self.prepared()?.project(curves)
    }

    /// Perform the curve-independent preflight work and return a reusable
    /// [`PreparedCashFlow`] that can be projected onto arbitrary market data.
    ///
    /// All the expensive setup (schedule compilation, date collection,
    /// amortization derivation, principal-event normalization) happens here.
    /// The returned value is immutable, cheap to hold, and safe to share
    /// across repeated [`PreparedCashFlow::project`] calls — making it the
    /// right entry point when repricing under many curve bumps.
    ///
    /// Calling this does NOT touch any
    /// [`finstack_core::market_data::context::MarketContext`] — floating-rate
    /// projection is deferred until [`PreparedCashFlow::project`].
    ///
    /// # Errors
    ///
    /// Returns the same validation errors that [`build_with_curves`](Self::build_with_curves)
    /// would surface (missing principal, bad schedules, out-of-range principal
    /// events, currency mismatches, amortization validation failures, etc.).
    /// Curve-lookup failures are deferred to the project step and not
    /// raised here.
    pub fn prepared(&self) -> finstack_core::Result<PreparedCashFlow> {
        if let Some(err) = &self.pending_error {
            return Err(err.clone());
        }
        // 1) Validate core inputs
        let (notional, issue, maturity) = validate_core_inputs(self)?;

        // 2) Compile schedules and fees
        let (
            CompiledSchedules {
                fixed_schedules,
                float_schedules,
            },
            periodic_fees,
            fixed_fees,
        ) = {
            let compiled = compute_coupon_schedules(self, issue, maturity)?;
            let (periodic_fees, fixed_fees) = build_fee_schedules(issue, maturity, &self.fees)?;
            (compiled, periodic_fees, fixed_fees)
        };

        // 2b) Normalize principal events (sorted) and validate currency/date bounds
        let mut principal_events = self.principal_events.clone();
        principal_events.sort_by_key(|ev| ev.date);

        // Reject principal events with currency different from notional.
        let expected_ccy = notional.initial.currency();
        if let Some(ev) = principal_events
            .iter()
            .find(|ev| ev.delta.currency() != expected_ccy)
        {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: expected_ccy,
                actual: ev.delta.currency(),
            });
        }

        // Reject principal events after maturity (would create post-maturity flows
        // after outstanding has been zeroed out, leading to undefined behavior).
        if let Some(ev) = principal_events.iter().find(|ev| ev.date > maturity) {
            return Err(InputError::DateOutOfRange {
                date: ev.date,
                range: (issue, maturity),
            }
            .into());
        }

        // 3) Collect all relevant dates
        let date_inputs = DateCollectionInputs {
            issue,
            maturity,
            fixed_schedules: &fixed_schedules,
            float_schedules: &float_schedules,
            periodic_fees: &periodic_fees,
            fixed_fees: &fixed_fees,
            notional: &notional,
            principal_events: &principal_events,
        };
        let dates = collect_all_dates(&date_inputs)?;
        debug!(dates = dates.len(), %issue, %maturity, "cashflow schedule: dates collected");

        // 4) Derive amortization setup
        let amort_setup = derive_amortization_setup(&notional, &fixed_schedules, &float_schedules)?;

        Ok(PreparedCashFlow {
            notional,
            issue,
            maturity,
            fixed_schedules,
            float_schedules,
            periodic_fees,
            fixed_fees,
            principal_events,
            dates,
            amort_setup,
        })
    }
}

impl PreparedCashFlow {
    /// Project the prepared schedule onto the supplied market curves to
    /// produce the fully-materialized [`CashFlowSchedule`].
    ///
    /// When `curves` is `Some(market)`, floating-rate coupons are computed
    /// from the forward curves registered under each float spec's
    /// `index_id`. When `None`, each float spec's fallback policy controls
    /// behavior (default: error; `SpreadOnly` uses only the margin;
    /// `FixedRate(r)` uses a fixed index).
    ///
    /// This method is safe to call repeatedly on the same
    /// `PreparedCashFlow` — no shared mutable state is retained between
    /// calls, so concurrent projection is supported as long as the
    /// caller synchronizes access to the underlying `MarketContext`.
    pub fn project(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
        // 5) Initialize fold state and build context (processing issue-date principal events)
        let mut state = initialize_build_state(
            self.issue,
            &self.notional,
            self.dates.len(),
            &self.principal_events,
        );
        let ccy = self.notional.initial.currency();
        for (fee_date, amount) in &self.fixed_fees {
            if *fee_date == self.issue && amount.amount() != 0.0 {
                state.flows.push(CashFlow {
                    date: *fee_date,
                    reset_date: None,
                    amount: *amount,
                    kind: CFKind::Fee,
                    accrual_factor: 0.0,
                    rate: None,
                });
            }
        }
        let ctx = BuildContext {
            ccy,
            maturity: self.maturity,
            notional: &self.notional,
            fixed_schedules: &self.fixed_schedules,
            float_schedules: &self.float_schedules,
            periodic_fees: &self.periodic_fees,
            fixed_fees: &self.fixed_fees,
            principal_events: &self.principal_events,
        };

        // Resolve curves upfront and reuse across all payment dates.
        let resolved_curves: Vec<Option<Arc<ForwardCurve>>> = if let Some(mkt) = curves {
            self.float_schedules
                .iter()
                .map(|(spec, _, _)| mkt.get_forward(spec.rate_spec.index_id.as_str()).ok())
                .collect()
        } else {
            vec![None; self.float_schedules.len()]
        };

        // 6) Fold over dates producing flows deterministically
        let processor = DateProcessor::new(&ctx, &self.amort_setup, &resolved_curves);
        for &d in self.dates.iter().skip(1) {
            state = processor.process(d, state)?;
        }

        // 6.5) Sanity-check final outstanding against initial notional.
        //
        // After processing every scheduled date (including maturity's principal
        // repayment), a well-formed schedule for a standard bullet/amortizing
        // instrument should leave `state.outstanding ≈ 0`. A non-trivial
        // residual indicates amortization schedule misconfiguration
        // (percentages that don't sum to 100%). With Decimal tracking,
        // accumulated drift is eliminated; any residual is a genuine
        // configuration issue. Warn above 1bp relative so production
        // misconfigurations surface in logs instead of silently biasing
        // downstream PV and duration calculations.
        //
        // This is a warning, not an error: revolving facilities with draws
        // past maturity or user-defined terminal events may legitimately
        // end with a non-zero balance, and we don't want to reject those.
        let threshold = Decimal::new(1, 4); // 1e-4 = 1 bp relative
        let initial_amount = self.notional.initial.amount();
        if initial_amount.abs() > 0.0 {
            let initial_dec = f64_to_decimal_saturating(initial_amount);
            // Guard against zero initial (already checked above, but defensive)
            if initial_dec != Decimal::ZERO {
                let abs_outstanding = if state.outstanding < Decimal::ZERO {
                    -state.outstanding
                } else {
                    state.outstanding
                };
                let abs_initial = if initial_dec < Decimal::ZERO {
                    -initial_dec
                } else {
                    initial_dec
                };
                let relative_residual = abs_outstanding / abs_initial;
                if relative_residual > threshold {
                    let final_outstanding = state.outstanding.to_f64().unwrap_or(0.0);
                    let relative_residual_f64 = relative_residual.to_f64().unwrap_or(0.0);
                    tracing::warn!(
                        initial = initial_amount,
                        final_outstanding,
                        relative_residual = relative_residual_f64,
                        threshold_bps = 1.0,
                        "PreparedCashFlow: final outstanding balance deviates from zero; \
                         check amortization schedule or instrument terminal flow"
                    );
                }
            }
        }

        // 7) Finalize flows and produce meta/day count from compiled schedules.
        let (flows, meta, out_dc) = finalize_flows(
            state.flows,
            &self.fixed_schedules,
            &self.float_schedules,
            Some(self.issue),
        );
        debug!(flows = flows.len(), "cashflow schedule: project complete");
        Ok(CashFlowSchedule {
            flows,
            notional: self.notional.clone(),
            day_count: out_dc,
            meta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::FloatingRateSpec;
    use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
    use finstack_core::types::CurveId;
    use rust_decimal_macros::dec;
    use time::Month;

    fn test_dates() -> (Date, Date, Date) {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid issue date");
        let switch = Date::from_calendar_date(2026, Month::January, 1).expect("valid switch date");
        let maturity =
            Date::from_calendar_date(2027, Month::January, 1).expect("valid maturity date");
        (issue, switch, maturity)
    }

    #[test]
    fn add_fixed_coupon_window_records_decimal_conversion_error() {
        let (issue, _, maturity) = test_dates();
        let mut builder = CashFlowBuilder::default();
        let _ = builder
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .add_fixed_coupon_window(
                issue,
                maturity,
                1e100,
                ScheduleParams::annual_actact(),
                CouponType::Cash,
            );

        let err = builder
            .build_with_curves(None)
            .expect_err("oversized fixed rate should surface as a builder error");
        assert!(
            err.to_string().contains("add_fixed_coupon_window"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn fixed_to_float_records_decimal_to_f64_conversion_error() {
        let (issue, switch, maturity) = test_dates();
        let mut builder = CashFlowBuilder::default();
        let _ = builder.principal(Money::new(1_000_000.0, Currency::USD), issue, maturity);
        let fixed_win = FixedWindow {
            rate: Decimal::MAX,
            schedule: ScheduleParams::annual_actact(),
        };
        let float_spec = FloatingCouponSpec {
            coupon_type: CouponType::Cash,
            rate_spec: FloatingRateSpec {
                index_id: CurveId::new("USD-SOFR"),
                spread_bp: dec!(150),
                gearing: dec!(1),
                gearing_includes_spread: true,
                index_floor_bp: None,
                all_in_cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: Tenor::annual(),
                reset_lag_days: 2,
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::Following,
                calendar_id: "weekends_only".to_string(),
                fixing_calendar_id: None,
                end_of_month: false,
                payment_lag_days: 0,
                overnight_compounding: None,
                overnight_basis: None,
                fallback: Default::default(),
            },
            freq: Tenor::annual(),
            stub: StubKind::None,
        };
        let _ = builder.fixed_to_float(switch, fixed_win, float_spec, CouponType::Cash);

        let err = builder
            .build_with_curves(None)
            .expect_err("oversized fixed-to-float rate should surface as a builder error");
        assert!(
            err.to_string().contains("could not convert rate"),
            "unexpected error: {err}"
        );
    }
}
