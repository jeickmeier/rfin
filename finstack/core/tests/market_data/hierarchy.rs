use finstack_core::market_data::hierarchy::{HierarchyNode, MarketDataHierarchy, NodePath};
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
