#[cfg(feature = "scenarios")]
mod replay_tests {
    use finstack_core::market_data::context::MarketContext;
    use finstack_portfolio::ReplayTimeline;
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
}
