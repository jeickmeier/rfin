//! Attribution-domain golden tests.

use crate::run_golden;

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_brinson_fachler_2period() {
    run_golden!("attribution/brinson_fachler_2period.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_brinson_hood_beebower() {
    run_golden!("attribution/brinson_hood_beebower.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_multi_factor_ff3_attribution() {
    run_golden!("attribution/multi_factor_ff3_attribution.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_currency_local_decomposition() {
    run_golden!("attribution/currency_local_decomposition.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_contribution_to_return() {
    run_golden!("attribution/contribution_to_return.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_fi_carry_decomposition() {
    run_golden!("attribution/fi_carry_decomposition.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_fi_curve_parallel_slope_twist() {
    run_golden!("attribution/fi_curve_attribution_parallel_slope_twist.json");
}

#[test]
#[ignore = "requires executable attribution inputs; current fixture is a flattened reference placeholder"]
fn golden_attribution_fi_risk_based_carry_rates_credit_residual() {
    run_golden!("attribution/fi_risk_based_carry_rates_credit_residual.json");
}
