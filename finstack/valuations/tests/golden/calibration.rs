//! Calibration-domain golden tests.

use crate::run_golden;

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_usd_ois_bootstrap() {
    run_golden!("calibration/curves/usd_ois_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_usd_sofr_3m_bootstrap() {
    run_golden!("calibration/curves/usd_sofr_3m_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_eur_estr_bootstrap() {
    run_golden!("calibration/curves/eur_estr_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_gbp_sonia_bootstrap() {
    run_golden!("calibration/curves/gbp_sonia_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_jpy_tona_bootstrap() {
    run_golden!("calibration/curves/jpy_tona_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_vol_usd_swaption_sabr_cube() {
    run_golden!("calibration/vol/usd_swaption_sabr_cube.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_curves_usd_cpi_zc_inflation_bootstrap() {
    run_golden!("calibration/curves/usd_cpi_zc_inflation_bootstrap.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_vol_spx_equity_vol_smile() {
    run_golden!("calibration/vol/spx_equity_vol_smile.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_vol_eurusd_fx_vol_smile() {
    run_golden!("calibration/vol/eurusd_fx_vol_smile.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_hazard_cdx_ig_hazard() {
    run_golden!("calibration/hazard/cdx_ig_hazard.json");
}

#[test]
#[ignore = "requires executable calibration inputs; current fixture is a flattened reference placeholder"]
fn golden_calibration_hazard_single_name_hazard_5y() {
    run_golden!("calibration/hazard/single_name_hazard_5y.json");
}
