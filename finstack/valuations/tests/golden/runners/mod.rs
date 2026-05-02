//! Per-domain golden runners.

use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

pub mod attribution_common;
pub mod calibration_common;
pub mod calibration_curves;
pub mod calibration_hazard;
pub mod calibration_inflation_curves;
pub mod calibration_swaption_vol;
pub mod calibration_vol_smile;
pub mod integration_common;
pub mod integration_credit;
pub mod integration_rates;
pub mod pricing_bond;
pub mod pricing_bond_future;
pub mod pricing_cap_floor;
pub mod pricing_cds;
pub mod pricing_cds_option;
pub mod pricing_cds_tranche;
pub mod pricing_common;
pub mod pricing_convertible;
pub mod pricing_deposit;
pub mod pricing_equity_index_future;
pub mod pricing_equity_option;
pub mod pricing_fra;
pub mod pricing_fx_option;
pub mod pricing_fx_swap;
pub mod pricing_inflation_linked_bond;
pub mod pricing_inflation_swap;
pub mod pricing_ir_future;
pub mod pricing_irs;
pub mod pricing_structured_credit;
pub mod pricing_swaption;
pub mod pricing_term_loan;

pub(crate) fn reject_flattened_outputs(
    runner: &str,
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let snapshot_hint = if fixture.inputs.get("actual_outputs").is_some() {
        " fixture contains inputs.actual_outputs, which is a frozen reference snapshot and not executable input."
    } else {
        ""
    };
    Err(format!(
        "{runner} requires executable inputs that build canonical API calls.{snapshot_hint} Replace the flattened placeholder with calibration/attribution inputs before enabling this golden."
    ))
}
