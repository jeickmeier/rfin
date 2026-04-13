use finstack_core::market_data::surfaces::VolCube;
use finstack_core::math::volatility::sabr::SabrParams;

#[test]
fn test_vol_cube_builder_basic() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::builder("USD-SWAPTION")
        .expiries(&[1.0, 5.0])
        .tenors(&[2.0, 10.0])
        .node(p, 0.03)
        .node(p, 0.035)
        .node(p, 0.04)
        .node(p, 0.045)
        .build()
        .unwrap();

    assert_eq!(cube.id().as_str(), "USD-SWAPTION");
    assert_eq!(cube.expiries(), &[1.0, 5.0]);
    assert_eq!(cube.tenors(), &[2.0, 10.0]);
    assert_eq!(cube.grid_shape(), (2, 2));
}

#[test]
fn test_vol_cube_from_grid() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let params = vec![p; 4];
    let forwards = vec![0.03, 0.035, 0.04, 0.045];
    let cube =
        VolCube::from_grid("TEST", &[1.0, 5.0], &[2.0, 10.0], &params, &forwards).unwrap();
    assert_eq!(cube.grid_shape(), (2, 2));
}

#[test]
fn test_vol_cube_validation_rejects_bad_input() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    // Wrong number of params
    let result = VolCube::from_grid("BAD", &[1.0, 5.0], &[2.0, 10.0], &[p; 3], &[0.03; 3]);
    assert!(result.is_err());
    // Unsorted expiries
    let result = VolCube::from_grid("BAD", &[5.0, 1.0], &[2.0, 10.0], &[p; 4], &[0.03; 4]);
    assert!(result.is_err());
}

#[test]
fn test_vol_cube_vol_at_grid_node() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let fwd = 0.03;
    let cube = VolCube::from_grid("TEST", &[1.0], &[5.0], &[p], &[fwd]).unwrap();
    let strike = 0.03;
    let vol = cube.vol(1.0, 5.0, strike).unwrap();
    let expected = p.implied_vol_lognormal(fwd, strike, 1.0);
    assert!(
        (vol - expected).abs() < 1e-14,
        "grid-node vol {vol} != direct SABR {expected}"
    );
}

#[test]
fn test_vol_cube_vol_interpolated() {
    let p_lo = SabrParams::new(0.020, 0.5, -0.2, 0.3).unwrap();
    let p_hi = SabrParams::new(0.050, 0.5, -0.2, 0.5).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p_lo, p_lo, p_hi, p_hi],
        &[0.03, 0.04, 0.03, 0.04],
    )
    .unwrap();
    let strike = 0.035;
    let vol_mid = cube.vol(3.0, 7.5, strike).unwrap();
    assert!(vol_mid.is_finite() && vol_mid > 0.0);
}

#[test]
fn test_vol_cube_vol_clamped_extrapolation() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube =
        VolCube::from_grid("TEST", &[1.0, 5.0], &[5.0, 10.0], &[p; 4], &[0.03; 4]).unwrap();
    let vol = cube.vol_clamped(0.1, 2.0, 0.03);
    assert!(vol.is_finite() && vol > 0.0);
}

#[test]
fn test_vol_cube_vol_checked_out_of_bounds() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube =
        VolCube::from_grid("TEST", &[1.0, 5.0], &[5.0, 10.0], &[p; 4], &[0.03; 4]).unwrap();
    assert!(cube.vol(0.1, 7.0, 0.03).is_err());
    assert!(cube.vol(3.0, 2.0, 0.03).is_err());
}

#[test]
fn test_vol_cube_materialize_tenor_slice() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p; 4],
        &[0.03, 0.035, 0.04, 0.045],
    )
    .unwrap();
    let strikes = vec![0.01, 0.02, 0.03, 0.04, 0.05];
    let surface = cube.materialize_tenor_slice(5.0, &strikes).unwrap();
    assert_eq!(surface.expiries(), &[1.0, 5.0]);
    assert_eq!(surface.strikes(), &strikes[..]);
    let cube_vol = cube.vol(1.0, 5.0, 0.03).unwrap();
    let surf_vol = surface.value_checked(1.0, 0.03).unwrap();
    assert!(
        (cube_vol - surf_vol).abs() < 1e-14,
        "materialized surface vol {surf_vol} != cube vol {cube_vol}"
    );
}

#[test]
fn test_vol_cube_materialize_expiry_slice() {
    let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
    let cube = VolCube::from_grid(
        "TEST",
        &[1.0, 5.0],
        &[5.0, 10.0],
        &[p; 4],
        &[0.03, 0.035, 0.04, 0.045],
    )
    .unwrap();
    let strikes = vec![0.02, 0.03, 0.04];
    let surface = cube.materialize_expiry_slice(1.0, &strikes).unwrap();
    assert_eq!(surface.expiries(), &[5.0, 10.0]);
    assert_eq!(surface.strikes(), &strikes[..]);
}
