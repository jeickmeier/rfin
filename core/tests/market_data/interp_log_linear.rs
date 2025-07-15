use rfin_core::market_data::interp::{FlatFwd, InterpFn, LogLinearDf};

fn build_inputs() -> (Box<[f64]>, Box<[f64]>) {
    let knots: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0];
    let zero_rate = 0.03f64;
    let dfs: Vec<f64> = knots.iter().map(|&t| (-zero_rate * t).exp()).collect();
    (knots.into_boxed_slice(), dfs.into_boxed_slice())
}

#[test]
fn logdf_exact_knots() {
    let (knots, dfs) = build_inputs();
    let interp = LogLinearDf::new(knots.clone(), dfs.clone()).unwrap();
    for (i, &t) in knots.iter().enumerate() {
        assert!((interp.interp(t) - dfs[i]).abs() < 1e-12);
    }
}

#[test]
fn logdf_geometric_midpoint() {
    let (knots, dfs) = build_inputs();
    let interp = LogLinearDf::new(knots.clone(), dfs.clone()).unwrap();
    for seg in 0..knots.len() - 1 {
        let t_mid = 0.5 * (knots[seg] + knots[seg + 1]);
        // Expected via linear on log => geometric mean of dfs
        let expected = (dfs[seg].ln() * 0.5 + dfs[seg + 1].ln() * 0.5).exp();
        assert!((interp.interp(t_mid) - expected).abs() < 1e-12);
    }
}

#[test]
fn flat_fwd_matches_logdf() {
    let (knots, dfs) = build_inputs();
    let log_interp = LogLinearDf::new(knots.clone(), dfs.clone()).unwrap();
    let flat = FlatFwd::new(knots, dfs).unwrap();
    for step in 0..=30 {
        let t = step as f64 * 0.1;
        assert!((flat.interp(t) - log_interp.interp(t)).abs() < 1e-12);
    }
}
