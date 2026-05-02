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

#[test]
fn golden_bond_ust_2y_bullet() {
    run_golden!("pricing/bond/ust_2y_bullet.json");
}

#[test]
fn golden_bond_ust_10y_bullet() {
    run_golden!("pricing/bond/ust_10y_bullet.json");
}

#[test]
fn golden_bond_ust_30y_long_duration() {
    run_golden!("pricing/bond/ust_30y_long_duration.json");
}

#[test]
fn golden_bond_corp_ig_5y_zspread() {
    run_golden!("pricing/bond/corp_ig_5y_zspread.json");
}

#[test]
fn golden_bond_corp_hy_5y_ytm_recovery() {
    run_golden!("pricing/bond/corp_hy_5y_ytm_recovery.json");
}

#[test]
fn golden_bond_with_accrued_midperiod() {
    run_golden!("pricing/bond/bond_with_accrued_midperiod.json");
}

#[test]
fn golden_bond_corp_callable_7nc3() {
    run_golden!("pricing/bond/corp_callable_7nc3.json");
}

#[test]
fn golden_bond_amortizing_bond_known_schedule() {
    run_golden!("pricing/bond/amortizing_bond_known_schedule.json");
}

#[test]
fn golden_convertible_conv_bond_atm_3y() {
    run_golden!("pricing/convertible/conv_bond_atm_3y.json");
}

#[test]
fn golden_convertible_conv_bond_distressed() {
    run_golden!("pricing/convertible/conv_bond_distressed.json");
}

#[test]
fn golden_term_loan_b_5y_floating() {
    run_golden!("pricing/term_loan/term_loan_b_5y_floating.json");
}

#[test]
fn golden_equity_option_bs_atm_call_1y() {
    run_golden!("pricing/equity_option/bs_atm_call_1y.json");
}

#[test]
fn golden_equity_option_bs_otm_call_25d() {
    run_golden!("pricing/equity_option/bs_otm_call_25d.json");
}

#[test]
fn golden_equity_option_bs_itm_put() {
    run_golden!("pricing/equity_option/bs_itm_put.json");
}

#[test]
fn golden_equity_option_bs_short_dated_1m() {
    run_golden!("pricing/equity_option/bs_short_dated_1m.json");
}

#[test]
fn golden_equity_option_bs_with_dividend_yield() {
    run_golden!("pricing/equity_option/bs_with_dividend_yield.json");
}

#[test]
fn golden_fx_option_gk_eurusd_atm_3m() {
    run_golden!("pricing/fx_option/gk_eurusd_atm_3m.json");
}

#[test]
fn golden_fx_option_gk_eurusd_25d_call() {
    run_golden!("pricing/fx_option/gk_eurusd_25d_call.json");
}

#[test]
fn golden_fx_option_gk_usdjpy_atm_1y() {
    run_golden!("pricing/fx_option/gk_usdjpy_atm_1y.json");
}

#[test]
fn golden_fx_option_gk_eurusd_otm_call_6m() {
    run_golden!("pricing/fx_option/gk_eurusd_otm_call_6m.json");
}

#[test]
fn golden_cds_5y_par_spread() {
    run_golden!("pricing/cds/cds_5y_par_spread.json");
}

#[test]
fn golden_cds_5y_running_upfront() {
    run_golden!("pricing/cds/cds_5y_running_upfront.json");
}

#[test]
fn golden_cds_off_par_hazard() {
    run_golden!("pricing/cds/cds_off_par_hazard.json");
}

#[test]
fn golden_cds_high_yield_recovery() {
    run_golden!("pricing/cds/cds_high_yield_recovery.json");
}

#[test]
fn golden_cds_option_payer_atm_3m() {
    run_golden!("pricing/cds_option/cds_option_payer_atm_3m.json");
}

#[test]
fn golden_cds_tranche_cdx_ig_5y_3_7_mezz() {
    run_golden!("pricing/cds_tranche/cdx_ig_5y_3_7_mezz.json");
}

#[test]
fn golden_structured_credit_clo_mezzanine_base_case() {
    run_golden!("pricing/structured_credit/clo_mezzanine_base_case.json");
}

#[test]
fn golden_structured_credit_abs_credit_card_senior() {
    run_golden!("pricing/structured_credit/abs_credit_card_senior.json");
}

#[test]
fn golden_bond_future_ust_ty_10y_front_month() {
    run_golden!("pricing/bond_future/ust_ty_10y_front_month.json");
}

#[test]
fn golden_equity_index_future_spx_es_3m() {
    run_golden!("pricing/equity_index_future/spx_es_3m.json");
}

#[test]
fn golden_inflation_linked_bond_5y() {
    run_golden!("pricing/inflation_linked_bond/inflation_linked_bond_5y.json");
}

#[test]
fn golden_inflation_swap_zc_5y() {
    run_golden!("pricing/inflation_swap/inflation_zc_swap_5y.json");
}
