//! Calibration-domain golden tests.

use crate::run_golden;

#[test]
fn golden_calibration_curves_usd_ois_bootstrap() {
    run_golden!("calibration/curves/usd_ois_bootstrap.json");
}

#[test]
fn golden_calibration_curves_usd_sofr_3m_bootstrap() {
    run_golden!("calibration/curves/usd_sofr_3m_bootstrap.json");
}

#[test]
fn golden_calibration_curves_eur_estr_bootstrap() {
    run_golden!("calibration/curves/eur_estr_bootstrap.json");
}

#[test]
fn golden_calibration_curves_gbp_sonia_bootstrap() {
    run_golden!("calibration/curves/gbp_sonia_bootstrap.json");
}

#[test]
fn golden_calibration_curves_jpy_tona_bootstrap() {
    run_golden!("calibration/curves/jpy_tona_bootstrap.json");
}

#[test]
fn golden_calibration_vol_usd_swaption_sabr_cube() {
    run_golden!("calibration/vol/usd_swaption_sabr_cube.json");
}
