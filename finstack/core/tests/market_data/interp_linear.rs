/// Tests for the piece‐wise linear DF interpolator.
use finstack_core::market_data::interp::{InterpFn, LinearDf};

fn build_inputs() -> (Box<[f64]>, Box<[f64]>) {
    // Simple flat 2 % zero‐rate curve => DF = exp(−0.02 t)
    let knots: Vec<f64> = (0..=4).map(|i| i as f64).collect();
    let dfs: Vec<f64> = knots.iter().map(|&t| (-0.02f64 * t).exp()).collect();
    (knots.into_boxed_slice(), dfs.into_boxed_slice())
}

#[test]
fn linear_exact_fit() {
    let (knots, dfs) = build_inputs();
    let interp = LinearDf::new(knots.clone(), dfs.clone()).expect("build");

    for (i, &t) in knots.iter().enumerate() {
        assert!((interp.interp(t) - dfs[i]).abs() < 1e-12);
    }
}

#[test]
fn linear_midpoint_matches_manual_formula() {
    let (knots, dfs) = build_inputs();
    let interp = LinearDf::new(knots.clone(), dfs.clone()).unwrap();

    for seg in 0..knots.len() - 1 {
        let t_mid = 0.5 * (knots[seg] + knots[seg + 1]);
        let expected = 0.5 * (dfs[seg] + dfs[seg + 1]);
        assert!((interp.interp(t_mid) - expected).abs() < 1e-12);
    }
}
