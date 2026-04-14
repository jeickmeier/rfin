#[cfg(feature = "scenarios")]
mod replay_tests {
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_portfolio::{
        Entity, Portfolio, Position, PositionUnit, ReplayConfig, ReplayMode, ReplayTimeline,
    };
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    fn empty_market() -> MarketContext {
        MarketContext::new()
    }

    #[test]
    fn timeline_rejects_empty() {
        let result = ReplayTimeline::new(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_accepts_single_snapshot() {
        let result = ReplayTimeline::new(vec![(date!(2024 - 01 - 01), empty_market())]);
        assert!(result.is_ok());
        let tl = result.unwrap();
        assert_eq!(tl.len(), 1);
        assert!(!tl.is_empty());
        let (start, end) = tl.date_range();
        assert_eq!(start, date!(2024 - 01 - 01));
        assert_eq!(end, date!(2024 - 01 - 01));
    }

    #[test]
    fn timeline_accepts_sorted_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 02), empty_market()),
            (date!(2024 - 01 - 03), empty_market()),
        ]);
        assert!(result.is_ok());
        let tl = result.unwrap();
        assert_eq!(tl.len(), 3);
        let (start, end) = tl.date_range();
        assert_eq!(start, date!(2024 - 01 - 01));
        assert_eq!(end, date!(2024 - 01 - 03));
    }

    #[test]
    fn timeline_rejects_unsorted_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 02), empty_market()),
            (date!(2024 - 01 - 01), empty_market()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_rejects_duplicate_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 01), empty_market()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_iter_yields_all_snapshots() {
        let tl = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 02), empty_market()),
        ])
        .unwrap();
        let dates: Vec<_> = tl.iter().map(|(d, _)| *d).collect();
        assert_eq!(dates, vec![date!(2024 - 01 - 01), date!(2024 - 01 - 02)]);
    }

    fn build_test_portfolio() -> Portfolio {
        let as_of = date!(2024 - 01 - 01);
        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start_date(as_of)
            .maturity(date!(2024 - 02 - 01))
            .day_count(DayCount::Act360)
            .quote_rate_opt(Some(rust_decimal::Decimal::try_from(0.045).unwrap()))
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .unwrap();

        Portfolio::builder("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .unwrap()
    }

    fn market_at_rate(as_of: time::Date, rate_bp: f64) -> MarketContext {
        let rate = rate_bp / 10_000.0;
        let curve = DiscountCurve::builder("USD")
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (1.0, (-rate * 1.0_f64).exp()),
                (5.0, (-rate * 5.0_f64).exp()),
            ])
            .interp(InterpStyle::Linear)
            .allow_non_monotonic()
            .build()
            .unwrap();
        MarketContext::new().insert(curve)
    }

    #[test]
    fn replay_pv_only_produces_steps_for_each_date() {
        let portfolio = build_test_portfolio();
        let timeline = ReplayTimeline::new(vec![
            (
                date!(2024 - 01 - 01),
                market_at_rate(date!(2024 - 01 - 01), 0.0),
            ),
            (
                date!(2024 - 01 - 02),
                market_at_rate(date!(2024 - 01 - 02), 50.0),
            ),
            (
                date!(2024 - 01 - 03),
                market_at_rate(date!(2024 - 01 - 03), 100.0),
            ),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::PvOnly,
            attribution_method: Default::default(),
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        assert_eq!(result.steps.len(), 3);

        // Step 0 has no P&L
        assert!(result.steps[0].daily_pnl.is_none());
        assert!(result.steps[0].cumulative_pnl.is_none());
        assert!(result.steps[0].attribution.is_none());

        // All steps in PvOnly have no P&L fields
        for step in &result.steps {
            assert!(step.daily_pnl.is_none());
            assert!(step.cumulative_pnl.is_none());
            assert!(step.attribution.is_none());
        }

        // Dates match timeline
        assert_eq!(result.steps[0].date, date!(2024 - 01 - 01));
        assert_eq!(result.steps[1].date, date!(2024 - 01 - 02));
        assert_eq!(result.steps[2].date, date!(2024 - 01 - 03));

        // Summary
        assert_eq!(result.summary.num_steps, 3);
        assert_eq!(result.summary.start_date, date!(2024 - 01 - 01));
        assert_eq!(result.summary.end_date, date!(2024 - 01 - 03));
    }
}
