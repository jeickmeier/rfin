//! Integration-domain golden tests.

use crate::run_golden;

#[test]
fn golden_integration_usd_ois_calib_then_price_5y_irs() {
    run_golden!("integration/usd_ois_calib_then_price_5y_irs.json");
}

#[test]
fn golden_integration_swaption_calib_then_price_atm() {
    run_golden!("integration/swaption_calib_then_price_atm.json");
}

#[test]
fn golden_integration_cds_hazard_calib_then_price_off_par() {
    run_golden!("integration/cds_hazard_calib_then_price_off_par.json");
}
