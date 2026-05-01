//! Pricing-domain golden tests.

use crate::run_golden;

#[test]
fn golden_irs_usd_sofr_5y_receive_fixed_swpm() {
    run_golden!("pricing/irs/usd_sofr_5y_receive_fixed_swpm.json");
}
