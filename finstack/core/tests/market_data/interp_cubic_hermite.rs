use finstack_core::math::interp::{CubicHermite, InterpFn, ExtrapolationPolicy};

fn build_inputs() -> (Box<[f64]>, Box<[f64]>) {
    let knots: Vec<f64> = vec![0.0, 1.0, 2.0, 4.0, 7.0];
    let dfs: Vec<f64> = knots.iter().map(|&t| (-0.025f64 * t).exp()).collect();
    (knots.into_boxed_slice(), dfs.into_boxed_slice())
}

#[test]
fn cubic_hermite_exact_knots() {
    let (knots, dfs) = build_inputs();
    let interp = CubicHermite::new(knots.clone(), dfs.clone(), ExtrapolationPolicy::default()).unwrap();
    for (i, &t) in knots.iter().enumerate() {
        assert!((interp.interp(t) - dfs[i]).abs() < 1e-12);
    }
}

#[test]
fn cubic_hermite_monotone_decreasing() {
    let (knots, dfs) = build_inputs();
    let interp = CubicHermite::new(knots.clone(), dfs, ExtrapolationPolicy::default()).unwrap();
    let mut prev = interp.interp(0.0);
    for step in 1..=100 {
        let t = knots.last().unwrap() * (step as f64) / 100.0;
        let p = interp.interp(t);
        assert!(p > 0.0);
        assert!(p <= prev + 1e-12);
        prev = p;
    }
}
