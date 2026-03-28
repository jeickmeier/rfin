//! Cashflow-related traits and aliases.

use crate::cashflow::builder::schedule::{CashFlowMeta, CashFlowSchedule};
use crate::cashflow::builder::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
pub use crate::cashflow::DatedFlows;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
///
/// Instruments implement this to generate their cashflow schedules
/// given market curves and valuation date.
pub trait CashflowProvider: Send + Sync {
    /// Returns the instrument's notional amount, if applicable.
    ///
    /// Instruments with a defined notional should override this to return
    /// their principal amount. For multi-leg instruments (e.g., swaps),
    /// this typically returns the primary/receive leg notional.
    ///
    /// Default returns `None`, indicating the instrument doesn't have
    /// a simple notional concept or hasn't implemented this method.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, DayCount};
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_cashflows::builder::CashFlowSchedule;
    /// use finstack_cashflows::{CashflowProvider, schedule_from_dated_flows};
    ///
    /// struct MyInstrument {
    ///     notional: Money,
    /// }
    ///
    /// impl CashflowProvider for MyInstrument {
    ///     fn build_full_schedule(
    ///         &self,
    ///         _curves: &MarketContext,
    ///         _as_of: Date,
    ///     ) -> finstack_core::Result<CashFlowSchedule> {
    ///         Ok(schedule_from_dated_flows(vec![], Some(self.notional), DayCount::Act365F))
    ///     }
    ///
    ///     fn notional(&self) -> Option<Money> {
    ///         Some(self.notional)
    ///     }
    /// }
    ///
    /// let inst = MyInstrument { notional: Money::new(1_000_000.0, Currency::USD) };
    /// assert_eq!(inst.notional().unwrap().currency(), Currency::USD);
    /// ```
    fn notional(&self) -> Option<Money> {
        None
    }

    /// Build full cashflow schedule with CFKind metadata and outstanding tracking.
    ///
    /// This enhanced method provides complete cashflow information including:
    /// - Precise cashflow classification via `CFKind` (Interest, Principal, Fees, etc.)
    /// - Outstanding balance tracking over time
    /// - Notional amortization schedules
    ///
    /// Implementers should return their canonical [`CashFlowSchedule`]. Callers that only need
    /// `(Date, Money)` pairs can rely on [`CashflowProvider::build_dated_flows`] which converts
    /// this schedule automatically.
    ///
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule>;

    /// Convenience: build holder-view `(Date, Money)` flows derived from the full schedule.
    ///
    /// Most callers that previously used `build_schedule` should call this
    /// helper, which simply converts the [`CashFlowSchedule`] returned by
    /// [`CashflowProvider::build_full_schedule`] into a `Vec<(Date, Money)>`.
    fn build_dated_flows(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let schedule = self.build_full_schedule(curves, as_of)?;
        Ok(schedule
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }
}

/// Helper to convert holder-view `(Date, Money)` flows into a [`CashFlowSchedule`].
///
/// This mirrors the legacy default implementation and can be used by instruments that
/// naturally produce dated flows but still need to return a `CashFlowSchedule`.
///
/// # Arguments
///
/// * `flows` - List of dated cashflows as `(Date, Money)` pairs
/// * `notional_hint` - Optional notional amount; if `None`, uses 0.0 with currency from first flow
/// * `day_count` - Day count convention for the schedule. **Must be explicitly specified** to avoid
///   incorrect yield/accrual calculations. Common conventions:
///   - `DayCount::Act365F` for most bonds
///   - `DayCount::Thirty360` for US corporate bonds
///   - `DayCount::Act360` for money markets
///
/// # Example
///
/// ```rust
/// use finstack_cashflows::schedule_from_dated_flows;
/// use finstack_core::dates::{Date, DayCount};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use time::Month;
///
/// let flows = vec![
///     (Date::from_calendar_date(2025, Month::June, 15).unwrap(), Money::new(50_000.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::December, 15).unwrap(), Money::new(1_050_000.0, Currency::USD)),
/// ];
/// let schedule = schedule_from_dated_flows(flows, None, DayCount::Thirty360);
/// assert_eq!(schedule.day_count, DayCount::Thirty360);
/// ```
pub fn schedule_from_dated_flows(
    flows: DatedFlows,
    notional_hint: Option<Money>,
    day_count: DayCount,
) -> CashFlowSchedule {
    if flows.is_empty() {
        let ccy = notional_hint.map(|n| n.currency()).unwrap_or(Currency::USD);
        return CashFlowSchedule {
            flows: vec![],
            notional: Notional::par(0.0, ccy),
            day_count,
            meta: Default::default(),
        };
    }

    let first_currency = flows
        .first()
        .map(|(_, m)| m.currency())
        .unwrap_or(Currency::USD);
    let (notional_amount, notional_currency) = match notional_hint {
        Some(m) => (m.amount(), m.currency()),
        None => (0.0, first_currency),
    };

    let cf_flows: Vec<CashFlow> = flows
        .into_iter()
        .map(|(date, amount)| CashFlow {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Fixed,
            accrual_factor: 0.0,
            rate: None,
        })
        .collect();

    schedule_from_classified_flows(
        cf_flows,
        Some(Money::new(notional_amount, notional_currency)),
        day_count,
    )
}

/// Helper to convert holder-view `(Date, Money)` flows into a schedule with an explicit kind.
pub fn schedule_from_dated_flows_with_kind(
    flows: DatedFlows,
    kind: CFKind,
    notional_hint: Option<Money>,
    day_count: DayCount,
) -> CashFlowSchedule {
    if flows.is_empty() {
        return empty_schedule(notional_hint, day_count);
    }

    let inferred_currency = flows
        .first()
        .map(|(_, amount)| amount.currency())
        .or_else(|| notional_hint.map(|money| money.currency()))
        .unwrap_or(Currency::USD);
    let notional = notional_hint
        .map(|money| Notional::par(money.amount(), money.currency()))
        .unwrap_or_else(|| Notional::par(0.0, inferred_currency));
    let cf_flows = flows
        .into_iter()
        .map(|(date, amount)| CashFlow {
            date,
            reset_date: None,
            amount,
            kind,
            accrual_factor: 0.0,
            rate: None,
        })
        .collect();

    CashFlowSchedule::from_parts(cf_flows, notional, day_count, Default::default())
}

/// Helper to convert classified cashflows into a [`CashFlowSchedule`] without losing `CFKind`.
pub fn schedule_from_classified_flows(
    flows: Vec<CashFlow>,
    notional_hint: Option<Money>,
    day_count: DayCount,
) -> CashFlowSchedule {
    let inferred_currency = flows
        .first()
        .map(|cf| cf.amount.currency())
        .unwrap_or(Currency::USD);
    let notional = notional_hint
        .map(|money| Notional::par(money.amount(), money.currency()))
        .unwrap_or_else(|| Notional::par(0.0, inferred_currency));
    CashFlowSchedule::from_parts(flows, notional, day_count, Default::default())
}

/// Canonical root constructor for provider schedules that already have classified flows and metadata.
///
/// Instruments that cannot express their cashflows through `CashFlowSchedule::builder()` should
/// still terminate in the shared `finstack_cashflows` construction layer via this helper rather
/// than assembling `CashFlowSchedule` ad hoc in downstream crates.
pub fn schedule_from_classified_flows_with_meta(
    flows: Vec<CashFlow>,
    notional: Notional,
    day_count: DayCount,
    meta: CashFlowMeta,
) -> CashFlowSchedule {
    CashFlowSchedule::from_parts(flows, notional, day_count, meta)
}

/// Build an empty schedule while preserving any available notional metadata.
pub fn empty_schedule(notional_hint: Option<Money>, day_count: DayCount) -> CashFlowSchedule {
    schedule_from_classified_flows(Vec::new(), notional_hint, day_count)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    struct DummyInstrument;

    impl CashflowProvider for DummyInstrument {
        fn notional(&self) -> Option<Money> {
            Some(Money::new(1_000_000.0, Currency::USD))
        }

        fn build_full_schedule(
            &self,
            _curves: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<CashFlowSchedule> {
            let d1 = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
            let d2 = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
            let flows = vec![
                (d1, Money::new(100.0, Currency::USD)),
                (d2, Money::new(250.0, Currency::USD)),
            ];
            Ok(schedule_from_dated_flows(
                flows,
                self.notional(),
                DayCount::Act365F,
            ))
        }
    }

    #[test]
    fn build_dated_flows_matches_schedule_contents() {
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let dummy = DummyInstrument;
        let holder_flows = dummy
            .build_dated_flows(&curves, as_of)
            .expect("should build flows");
        assert_eq!(holder_flows.len(), 2);
        assert_eq!(holder_flows[0].1.amount(), 100.0);
        assert_eq!(holder_flows[1].1.amount(), 250.0);
    }

    #[test]
    fn schedule_from_dated_flows_uses_notional_hint() {
        let flows = vec![(
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Money::new(100.0, Currency::USD),
        )];
        let notional = Money::new(5_000_000.0, Currency::USD);
        let schedule = schedule_from_dated_flows(flows, Some(notional), DayCount::Act365F);
        assert_eq!(schedule.notional.initial.amount(), 5_000_000.0);
        assert_eq!(schedule.notional.initial.currency(), Currency::USD);
    }

    #[test]
    fn schedule_from_dated_flows_defaults_currency() {
        let flows = vec![(
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Money::new(100.0, Currency::EUR),
        )];
        let schedule = schedule_from_dated_flows(flows, None, DayCount::Thirty360);
        assert_eq!(schedule.notional.initial.amount(), 0.0);
        assert_eq!(schedule.notional.initial.currency(), Currency::EUR);
        assert_eq!(schedule.day_count, DayCount::Thirty360);
    }

    #[test]
    fn schedule_from_classified_flows_preserves_kinds() {
        let date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let flows = vec![
            CashFlow {
                date,
                reset_date: None,
                amount: Money::new(20.0, Currency::USD),
                kind: CFKind::PrePayment,
                accrual_factor: 0.0,
                rate: None,
            },
            CashFlow {
                date,
                reset_date: None,
                amount: Money::new(-5.0, Currency::USD),
                kind: CFKind::DefaultedNotional,
                accrual_factor: 0.0,
                rate: None,
            },
        ];

        let schedule = schedule_from_classified_flows(
            flows,
            Some(Money::new(100.0, Currency::USD)),
            DayCount::Act365F,
        );

        assert_eq!(schedule.flows.len(), 2);
        assert_eq!(schedule.flows[0].kind, CFKind::PrePayment);
        assert_eq!(schedule.flows[1].kind, CFKind::DefaultedNotional);
    }

    #[test]
    fn schedule_from_dated_flows_with_kind_applies_requested_kind() {
        let flows = vec![(
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Money::new(100.0, Currency::USD),
        )];

        let schedule = schedule_from_dated_flows_with_kind(
            flows,
            CFKind::Notional,
            Some(Money::new(100.0, Currency::USD)),
            DayCount::Act365F,
        );

        assert_eq!(schedule.flows.len(), 1);
        assert_eq!(schedule.flows[0].kind, CFKind::Notional);
    }

    #[test]
    fn schedule_from_classified_flows_with_meta_preserves_notional_and_meta() {
        let date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let flows = vec![CashFlow {
            date,
            reset_date: None,
            amount: Money::new(25.0, Currency::USD),
            kind: CFKind::Fee,
            accrual_factor: 0.0,
            rate: None,
        }];
        let notional = Notional::par(250.0, Currency::USD);
        let meta = CashFlowMeta {
            calendar_ids: vec!["weekends_only".to_string()],
            facility_limit: Some(Money::new(500.0, Currency::USD)),
            issue_date: Some(date),
        };

        let schedule = schedule_from_classified_flows_with_meta(
            flows,
            notional.clone(),
            DayCount::Act365F,
            meta.clone(),
        );

        assert_eq!(
            schedule.notional.initial.amount(),
            notional.initial.amount()
        );
        assert_eq!(schedule.meta.issue_date, meta.issue_date);
        assert_eq!(schedule.meta.facility_limit, meta.facility_limit);
        assert_eq!(schedule.meta.calendar_ids, meta.calendar_ids);
    }
}
