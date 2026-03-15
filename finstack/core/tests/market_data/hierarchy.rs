use finstack_core::market_data::hierarchy::{
    HierarchyNode, HierarchyTarget, MarketDataHierarchy, NodePath, ResolutionMode, TagFilter,
    TagPredicate,
};
use finstack_core::types::CurveId;

#[test]
fn empty_hierarchy_has_no_roots() {
    let h = MarketDataHierarchy::new();
    assert!(h.roots().is_empty());
}

#[test]
fn hierarchy_node_stores_name_and_curves() {
    let node = HierarchyNode::new("USD");
    assert_eq!(node.name(), "USD");
    assert!(node.curve_ids().is_empty());
    assert!(node.children().is_empty());
    assert!(node.tags().is_empty());
}

#[test]
fn node_path_is_vec_of_strings() {
    let path: NodePath = vec!["Rates".into(), "USD".into()];
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], "Rates");
}

#[test]
fn builder_creates_hierarchy_with_slash_paths() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/USD/Forward/SOFR")
        .curve_ids(&["USD-SOFR-3M", "USD-SOFR-6M"])
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .tag("rating", "A")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .build()
        .unwrap();

    // Check structure
    assert_eq!(h.roots().len(), 2); // Rates, Credit
    assert!(h.roots().contains_key("Rates"));
    assert!(h.roots().contains_key("Credit"));

    // Check deep path
    let path: NodePath = vec!["Rates".into(), "USD".into(), "OIS".into()];
    let node = h.get_node(&path).unwrap();
    assert_eq!(node.curve_ids().len(), 1);
    assert_eq!(node.curve_ids()[0], CurveId::from("USD-OIS"));

    // Check tags
    let credit_path: NodePath = vec![
        "Credit".into(),
        "US".into(),
        "IG".into(),
        "Financials".into(),
    ];
    let credit_node = h.get_node(&credit_path).unwrap();
    assert_eq!(credit_node.tags().get("sector").unwrap(), "Financials");
    assert_eq!(credit_node.tags().get("rating").unwrap(), "A");
}

#[test]
fn builder_rejects_duplicate_curve_ids() {
    let result = MarketDataHierarchy::builder()
        .add_node("Rates/USD")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US")
        .curve_ids(&["USD-OIS"]) // duplicate!
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("USD-OIS"),
        "Error should mention the duplicate: {err}"
    );
}

#[test]
fn all_curve_ids_collects_entire_tree() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/EUR/ESTR")
        .curve_ids(&["EUR-ESTR"])
        .add_node("Credit/US/IG")
        .curve_ids(&["JPM-5Y"])
        .build()
        .unwrap();

    let all = h.all_curve_ids();
    assert_eq!(all.len(), 3);
}

#[test]
fn path_for_curve_finds_correct_location() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .build()
        .unwrap();

    let path = h.path_for_curve(&CurveId::from("JPM-5Y")).unwrap();
    assert_eq!(path, vec!["Credit", "US", "IG", "Financials"]);

    assert!(h.path_for_curve(&CurveId::from("NONEXISTENT")).is_none());
}

#[test]
fn serde_round_trip() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&h).unwrap();
    let deserialized: MarketDataHierarchy = serde_json::from_str(&json).unwrap();

    // Verify structure preserved
    assert_eq!(deserialized.roots().len(), h.roots().len());
    assert_eq!(deserialized.all_curve_ids().len(), h.all_curve_ids().len());

    let path: NodePath = vec![
        "Credit".into(),
        "US".into(),
        "IG".into(),
        "Financials".into(),
    ];
    let node = deserialized.get_node(&path).unwrap();
    assert_eq!(node.tags().get("sector").unwrap(), "Financials");
    assert_eq!(node.curve_ids().len(), 2);
}

#[test]
fn insert_and_remove_curve() {
    let mut h = MarketDataHierarchy::builder()
        .add_node("Rates/USD")
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    h.insert_curve("Rates/USD", "USD-SOFR-3M");
    let path: NodePath = vec!["Rates".into(), "USD".into()];
    assert_eq!(h.get_node(&path).unwrap().curve_ids().len(), 2);

    assert!(h.remove_curve(&CurveId::from("USD-OIS")));
    assert_eq!(h.get_node(&path).unwrap().curve_ids().len(), 1);
    assert!(!h.remove_curve(&CurveId::from("NONEXISTENT")));
}

// ─── Resolution engine tests ─────────────────────────────────────────────────

/// Build a hierarchy that places "SHARED-CURVE" at two depths under Credit:
///   Credit          (depth 0 from root Credit)
///   Credit/US       (depth 1)
///   Credit/US/IG    (depth 2)
///
/// We use `insert_curve` which bypasses the builder's duplicate-detection so
/// the same CurveId can appear at multiple nodes — exactly the scenario needed
/// to verify `MostSpecificWins`.
fn hierarchy_with_curve_at_multiple_depths() -> MarketDataHierarchy {
    let mut h = MarketDataHierarchy::new();
    h.insert_curve("Credit", "SHARED-CURVE");
    h.insert_curve("Credit/US", "SHARED-CURVE");
    h.insert_curve("Credit/US/IG", "SHARED-CURVE");
    h.insert_curve("Credit/US/IG", "IG-ONLY");
    h
}

#[test]
fn resolve_most_specific_wins_deduplicates_by_depth() {
    let h = hierarchy_with_curve_at_multiple_depths();

    let target = HierarchyTarget {
        path: vec!["Credit".into()],
        tag_filter: None,
    };
    let mut ids = h.resolve(&target, ResolutionMode::MostSpecificWins);
    ids.sort();

    // SHARED-CURVE appears at depth 0, 1, and 2. MostSpecificWins should
    // keep only the depth-2 instance, returning it exactly once.
    // IG-ONLY is only at depth 2.
    assert_eq!(
        ids.len(),
        2,
        "expected 2 distinct curves, got {}: {:?}",
        ids.len(),
        ids
    );
    assert!(ids.contains(&CurveId::from("SHARED-CURVE")));
    assert!(ids.contains(&CurveId::from("IG-ONLY")));
}

#[test]
fn resolve_cumulative_returns_all_occurrences() {
    let h = hierarchy_with_curve_at_multiple_depths();

    let target = HierarchyTarget {
        path: vec!["Credit".into()],
        tag_filter: None,
    };
    let ids = h.resolve(&target, ResolutionMode::Cumulative);

    // SHARED-CURVE is at Credit, Credit/US, Credit/US/IG — 3 occurrences.
    // IG-ONLY is at Credit/US/IG — 1 occurrence. Total: 4.
    assert_eq!(
        ids.len(),
        4,
        "expected 4 total curve entries (including duplicates), got {}: {:?}",
        ids.len(),
        ids
    );
    let shared_count = ids
        .iter()
        .filter(|id| *id == &CurveId::from("SHARED-CURVE"))
        .count();
    assert_eq!(
        shared_count, 3,
        "expected SHARED-CURVE three times, got {shared_count}"
    );
}

#[test]
fn resolve_returns_empty_for_nonexistent_path() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD")
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    let target = HierarchyTarget {
        path: vec!["Nonexistent".into()],
        tag_filter: None,
    };
    assert!(h
        .resolve(&target, ResolutionMode::MostSpecificWins)
        .is_empty());
    assert!(h.resolve(&target, ResolutionMode::Cumulative).is_empty());
}

#[test]
fn query_by_tags_finds_curves_where_node_tag_matches() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .add_node("Credit/US/IG/Technology")
        .tag("sector", "Technology")
        .curve_ids(&["MSFT-5Y"])
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "sector".into(),
            value: "Financials".into(),
        }],
    };

    let mut ids = h.query_by_tags(&filter);
    ids.sort();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&CurveId::from("JPM-5Y")));
    assert!(ids.contains(&CurveId::from("GS-5Y")));
    assert!(!ids.contains(&CurveId::from("MSFT-5Y")));
    assert!(!ids.contains(&CurveId::from("USD-OIS")));
}

#[test]
fn tag_predicate_equals_matches_exact_value_only() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG")
        .tag("rating", "A")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/EU/HY")
        .tag("rating", "BB")
        .curve_ids(&["PEUGEOT-3Y"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "rating".into(),
            value: "A".into(),
        }],
    };
    let ids = h.query_by_tags(&filter);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], CurveId::from("JPM-5Y"));

    // A predicate for a value that doesn't exist should return nothing.
    let filter_none = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "rating".into(),
            value: "AAA".into(),
        }],
    };
    assert!(h.query_by_tags(&filter_none).is_empty());
}

#[test]
fn tag_predicate_in_matches_any_of_the_given_values() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG")
        .tag("rating", "A")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/EU/IG")
        .tag("rating", "AA")
        .curve_ids(&["SIEMENS-5Y"])
        .add_node("Credit/EU/HY")
        .tag("rating", "BB")
        .curve_ids(&["PEUGEOT-3Y"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::In {
            key: "rating".into(),
            values: vec!["A".into(), "AA".into()],
        }],
    };
    let mut ids = h.query_by_tags(&filter);
    ids.sort();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&CurveId::from("JPM-5Y")));
    assert!(ids.contains(&CurveId::from("SIEMENS-5Y")));
    assert!(!ids.contains(&CurveId::from("PEUGEOT-3Y")));
}

#[test]
fn tag_predicate_exists_matches_key_regardless_of_value() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/EU/IG")
        .tag("sector", "Industrials")
        .curve_ids(&["BASF-5Y"])
        .add_node("Rates/USD/OIS")
        // No sector tag
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Exists {
            key: "sector".into(),
        }],
    };
    let mut ids = h.query_by_tags(&filter);
    ids.sort();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&CurveId::from("JPM-5Y")));
    assert!(ids.contains(&CurveId::from("BASF-5Y")));
    assert!(!ids.contains(&CurveId::from("USD-OIS")));
}

#[test]
fn tag_predicate_exists_returns_empty_when_key_absent_from_all_nodes() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Exists {
            key: "nonexistent-key".into(),
        }],
    };
    assert!(h.query_by_tags(&filter).is_empty());
}

#[test]
fn resolve_cumulative_with_tag_filter_scopes_to_subtree() {
    // Build a hierarchy with two regions (USD and EUR), each with children
    // tagged with `asset_class`. Only the USD subtree nodes tagged as "rates"
    // should appear in the result.
    //
    // Structure:
    //   USD
    //   USD/Rates        (asset_class=rates)  → curves: USD-OIS, USD-SOFR
    //   USD/Credit       (asset_class=credit) → curves: JPM-5Y
    //   EUR
    //   EUR/Rates        (asset_class=rates)  → curves: EUR-ESTR
    let mut h = MarketDataHierarchy::new();
    h.insert_curve("USD/Rates", "USD-OIS");
    h.insert_curve("USD/Rates", "USD-SOFR");
    h.insert_curve("USD/Credit", "JPM-5Y");
    h.insert_curve("EUR/Rates", "EUR-ESTR");

    // Tag the nodes via the builder on a fresh hierarchy (insert_curve does not
    // attach tags). Rebuild using builder to attach tags properly.
    let h = MarketDataHierarchy::builder()
        .add_node("USD/Rates")
        .tag("asset_class", "rates")
        .curve_ids(&["USD-OIS", "USD-SOFR"])
        .add_node("USD/Credit")
        .tag("asset_class", "credit")
        .curve_ids(&["JPM-5Y"])
        .add_node("EUR/Rates")
        .tag("asset_class", "rates")
        .curve_ids(&["EUR-ESTR"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "asset_class".into(),
            value: "rates".into(),
        }],
    };

    // Resolve within the USD subtree with the tag filter in Cumulative mode.
    let target = HierarchyTarget {
        path: vec!["USD".into()],
        tag_filter: Some(filter),
    };
    let mut ids = h.resolve(&target, ResolutionMode::Cumulative);
    ids.sort();

    // Only USD/Rates curves should appear — EUR-ESTR is excluded (wrong path),
    // and JPM-5Y is excluded (wrong tag).
    assert_eq!(
        ids.len(),
        2,
        "expected 2 curves (USD-OIS, USD-SOFR), got {}: {:?}",
        ids.len(),
        ids
    );
    assert!(ids.contains(&CurveId::from("USD-OIS")));
    assert!(ids.contains(&CurveId::from("USD-SOFR")));
    assert!(
        !ids.contains(&CurveId::from("EUR-ESTR")),
        "EUR curve must be excluded by path scoping"
    );
    assert!(
        !ids.contains(&CurveId::from("JPM-5Y")),
        "credit curve must be excluded by tag filter"
    );
}

#[test]
fn resolve_most_specific_wins_with_tag_filter() {
    // Build a hierarchy where the same CurveId appears at two depths, both
    // under nodes matching the tag filter. MostSpecificWins should return it
    // exactly once, from the deeper node.
    //
    // Structure:
    //   Rates                (asset_class=rates) → SHARED-RATE         (depth 0)
    //   Rates/USD            (asset_class=rates) → SHARED-RATE, USD-OIS (depth 1)
    //
    // Build with tagged nodes first, then use insert_curve to add the duplicate
    // at the parent depth (bypassing builder's duplicate detection).
    let mut h = MarketDataHierarchy::builder()
        .add_node("Rates")
        .tag("asset_class", "rates")
        .curve_ids(&[])
        .add_node("Rates/USD")
        .tag("asset_class", "rates")
        .curve_ids(&["SHARED-RATE", "USD-OIS"])
        .build()
        .unwrap();

    // Insert SHARED-RATE at the parent (Rates) level to create the multi-depth
    // scenario. insert_curve bypasses duplicate detection so this succeeds.
    h.insert_curve("Rates", "SHARED-RATE");

    let filter = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "asset_class".into(),
            value: "rates".into(),
        }],
    };

    let target = HierarchyTarget {
        path: vec!["Rates".into()],
        tag_filter: Some(filter),
    };
    let mut ids = h.resolve(&target, ResolutionMode::MostSpecificWins);
    ids.sort();

    // SHARED-RATE appears at depth 0 (Rates) and depth 1 (Rates/USD).
    // MostSpecificWins must return it exactly once (from depth 1).
    // USD-OIS appears only at depth 1.
    assert_eq!(
        ids.len(),
        2,
        "expected 2 distinct curves (SHARED-RATE, USD-OIS), got {}: {:?}",
        ids.len(),
        ids
    );
    assert!(ids.contains(&CurveId::from("SHARED-RATE")));
    assert!(ids.contains(&CurveId::from("USD-OIS")));
    let shared_count = ids
        .iter()
        .filter(|id| *id == &CurveId::from("SHARED-RATE"))
        .count();
    assert_eq!(
        shared_count, 1,
        "SHARED-RATE must appear exactly once, got {shared_count}"
    );
}

// ─── Completeness tracking tests ─────────────────────────────────────────────

#[test]
fn completeness_report_returns_none_without_hierarchy() {
    use finstack_core::market_data::context::MarketContext;
    let market = MarketContext::new();
    assert!(market.completeness_report().is_none());
}

#[test]
fn completeness_report_detects_missing_and_unclassified() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use time::Month;

    let base = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/EUR/ESTR")
        .curve_ids(&["EUR-ESTR"]) // will be missing from MarketContext
        .build()
        .unwrap();

    // Build MarketContext with only USD-OIS (EUR-ESTR is missing)
    // Also add an unclassified curve not in hierarchy
    let usd_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .unwrap();
    let extra_curve = DiscountCurve::builder("GBP-SONIA")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.96)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(usd_curve).insert(extra_curve);
    market.set_hierarchy(h);

    let report = market.completeness_report().unwrap();

    // EUR-ESTR is declared but missing
    assert_eq!(report.missing.len(), 1);
    assert_eq!(report.missing[0].1, CurveId::from("EUR-ESTR"));

    // GBP-SONIA is present but not in hierarchy
    assert_eq!(report.unclassified.len(), 1);
    assert_eq!(report.unclassified[0], CurveId::from("GBP-SONIA"));

    // Coverage: Rates root has 2 expected, 1 present = 50%
    assert!(!report.coverage.is_empty());
}
