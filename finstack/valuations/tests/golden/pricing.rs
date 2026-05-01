//! Pricing-domain golden tests.

use crate::run_golden;

#[test]
fn golden_deposit_usd_3m() {
    run_golden!("pricing/deposit/usd_deposit_3m.json");
}

#[test]
fn golden_fra_usd_3x6() {
    run_golden!("pricing/fra/usd_fra_3x6.json");
}

#[test]
fn golden_fx_swap_eurusd_3m() {
    run_golden!("pricing/fx_swap/eurusd_fx_swap_3m.json");
}

#[test]
fn golden_irs_usd_sofr_5y_receive_fixed_swpm() {
    run_golden!("pricing/irs/usd_sofr_5y_receive_fixed_swpm.json");
}

#[test]
fn golden_irs_usd_ois_5y() {
    run_golden!("pricing/irs/usd_ois_swap_5y.json");
}

#[test]
fn golden_irs_usd_sofr_5y_par() {
    run_golden!("pricing/irs/usd_irs_sofr_5y_par.json");
}

#[test]
fn golden_irs_usd_sofr_10y() {
    run_golden!("pricing/irs/usd_irs_sofr_10y.json");
}

#[test]
fn golden_irs_usd_sofr_2y() {
    run_golden!("pricing/irs/usd_irs_sofr_2y.json");
}

#[test]
fn golden_irs_eur_estr_5y() {
    run_golden!("pricing/irs/eur_irs_estr_5y.json");
}

#[test]
fn golden_irs_gbp_sonia_5y() {
    run_golden!("pricing/irs/gbp_irs_sonia_5y.json");
}

#[test]
fn golden_ir_future_sofr_3m_quarterly() {
    run_golden!("pricing/ir_future/sofr_3m_quarterly.json");
}

#[test]
fn golden_ir_future_sofr_1m_serial() {
    run_golden!("pricing/ir_future/sofr_1m_serial.json");
}

#[test]
fn golden_cap_floor_usd_cap_5y_atm_black() {
    run_golden!("pricing/cap_floor/usd_cap_5y_atm_black.json");
}

#[test]
fn golden_cap_floor_usd_floor_5y_atm_normal() {
    run_golden!("pricing/cap_floor/usd_floor_5y_atm_normal.json");
}

#[test]
fn golden_swaption_usd_5y_into_5y_payer_atm() {
    run_golden!("pricing/swaption/usd_swaption_5y_into_5y_payer_atm.json");
}

#[test]
fn golden_swaption_usd_5y_into_5y_receiver_25_otm() {
    run_golden!("pricing/swaption/usd_swaption_5y_into_5y_receiver_25_otm.json");
}
