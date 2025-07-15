use rfin_core::market_data::interp::{InterpFn, MonotoneConvex};

#[test]
fn monotone_convex_basic_properties() {
    let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
    let dfs = knots
        .iter()
        .map(|&t| (-0.02f64 * t).exp())
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs.clone()).expect("failed to build interp");

    // Exact fit at knots
    for (i, &t) in [0.0, 1.0, 2.0, 3.0].iter().enumerate() {
        let p = interp.interp(t);
        assert!((p - dfs[i]).abs() < 1e-12);
    }

    // Monotone decreasing and within bounds on dense grid
    let mut prev = interp.interp(0.0);
    for step in 1..=60 {
        let t = 3.0 * step as f64 / 60.0;
        let p = interp.interp(t);
        assert!(p > 0.0, "discount factor negative at t={}", t);
        assert!(p <= prev + 1e-12, "DF not non-increasing at t={}", t);
        prev = p;
    }
}
