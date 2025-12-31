mod wrapper;

mod agency_mbs;
mod asian_option;
mod autocallable;
mod barrier_option;
mod basis_swap;
mod bond;
mod bond_future;
mod cap_floor;
mod cds;
mod cds_index;
mod cds_option;
mod cds_tranche;
mod cliquet_option;
mod cms_option;
mod commodity_forward;
mod commodity_option;
mod commodity_swap;
mod convertible;
mod dcf;
mod deposit;
mod equity;
mod equity_index_future;
mod equity_option;
mod fra;
mod fx;
mod fx_barrier_option;
mod fx_forward;
mod fx_variance_swap;
mod inflation_cap_floor;
mod inflation_linked_bond;
mod inflation_swap;
mod ir_future;
mod irs;
mod lookback_option;
mod ndf;
mod private_markets_fund;
mod quanto_option;
mod range_accrual;
mod real_estate;
mod repo;
mod revolving_credit;
mod structured_credit;
mod swaption;
mod term_loan;
mod trs;
mod variance_swap;
mod vol_index_future;
mod vol_index_option;
mod xccy_swap;
mod yoy_inflation_swap;

// Re-export wrapper trait for internal use
pub(crate) use wrapper::InstrumentWrapper;

// Agency MBS instruments: exported directly via wasm_bindgen
#[allow(unused_imports)]
pub use agency_mbs::{
    JsAgencyCmo as AgencyCmo, JsAgencyMbsPassthrough as AgencyMbsPassthrough,
    JsAgencyTba as AgencyTba, JsCmoTranche as CmoTranche, JsCmoWaterfall as CmoWaterfall,
    JsDollarRoll as DollarRoll,
};

use finstack_valuations::instruments::common::traits::Instrument;
use js_sys::Reflect;
use wasm_bindgen::JsValue;

pub use asian_option::{JsAsianOption as AsianOption, JsAveragingMethod as AveragingMethod};
pub use autocallable::JsAutocallable as Autocallable;
pub use barrier_option::JsBarrierOption as BarrierOption;
pub use basis_swap::JsBasisSwap as BasisSwap;
pub use bond::JsBond as Bond;
pub use bond_future::{
    JsBondFuture as BondFuture, JsBondFutureSpecs as BondFutureSpecs,
    JsFuturePosition as FuturePosition,
};
// Also export JsBond name for use within this crate
pub use bond::JsBond;
pub use cap_floor::JsInterestRateOption as InterestRateOption;
pub use cds::JsCreditDefaultSwap as CreditDefaultSwap;
pub use cds_index::JsCDSIndex as CDSIndex;
pub use cds_option::JsCdsOption as CdsOption;
pub use cds_tranche::JsCdsTranche as CdsTranche;
pub use cliquet_option::JsCliquetOption as CliquetOption;
pub use cms_option::JsCmsOption as CmsOption;
pub use commodity_option::JsCommodityOption as CommodityOption;
// Commodity instruments: exported directly via wasm_bindgen
pub use convertible::JsConvertibleBond as ConvertibleBond;
pub use dcf::evaluate_dcf_wasm;
pub use deposit::JsDeposit as Deposit;
pub use equity::JsEquity as Equity;
pub use equity_index_future::{
    JsEquityFutureSpecs as EquityFutureSpecs, JsEquityIndexFuture as EquityIndexFuture,
};
pub use equity_option::JsEquityOption as EquityOption;
pub use fra::JsForwardRateAgreement as ForwardRateAgreement;
pub use fx::{JsFxOption as FxOption, JsFxSpot as FxSpot, JsFxSwap as FxSwap};
pub use fx_barrier_option::JsFxBarrierOption as FxBarrierOption;
pub use fx_forward::JsFxForward as FxForward;
pub use fx_variance_swap::{JsFxVarianceSwap as FxVarianceSwap, JsVarianceSwapSide as VarianceSwapSide};
pub use inflation_cap_floor::{
    JsInflationCapFloor as InflationCapFloor, JsInflationCapFloorType as InflationCapFloorType,
};
pub use inflation_linked_bond::JsInflationLinkedBond as InflationLinkedBond;
pub use inflation_swap::JsInflationSwap as InflationSwap;
pub use ir_future::JsInterestRateFuture as InterestRateFuture;
pub use irs::JsInterestRateSwap as InterestRateSwap;
pub use lookback_option::{JsLookbackOption as LookbackOption, JsLookbackType as LookbackType};
pub use ndf::JsNdf as Ndf;
pub use private_markets_fund::JsPrivateMarketsFund as PrivateMarketsFund;
pub use quanto_option::JsQuantoOption as QuantoOption;
pub use range_accrual::JsRangeAccrual as RangeAccrual;
pub use real_estate::{
    JsRealEstateAsset as RealEstateAsset, JsRealEstateValuationMethod as RealEstateValuationMethod,
};
pub use repo::JsRepo as Repo;
pub use revolving_credit::JsRevolvingCredit as RevolvingCredit;
pub use structured_credit::{
    JsBasket as Basket, JsCoverageTestRules as CoverageTestRules,
    JsCoverageTrigger as CoverageTrigger, JsPool as Pool, JsStructuredCredit as StructuredCredit,
    JsTrancheStructure as TrancheStructure, JsWaterfall as WaterfallEngine,
    JsWaterfallDistribution as WaterfallDistribution,
};
pub use swaption::JsSwaption as Swaption;
pub use term_loan::JsTermLoan as TermLoan;
pub use trs::{
    JsEquityTotalReturnSwap as EquityTotalReturnSwap,
    JsFiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
};
pub use variance_swap::{JsRealizedVarMethod as RealizedVarMethod, JsVarianceSwap as VarianceSwap};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_future::JsVolatilityIndexFuture as VolatilityIndexFuture;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_option::JsVolatilityIndexOption as VolatilityIndexOption;
pub use xccy_swap::{
    JsLegSide as LegSide, JsNotionalExchange as NotionalExchange, JsXccySwap as XccySwap,
    JsXccySwapLeg as XccySwapLeg,
};
pub use yoy_inflation_swap::{
    JsPayReceiveInflation as PayReceiveInflation, JsYoYInflationSwap as YoYInflationSwap,
};

/// Downcast a JavaScript instrument wrapper into a core instrument reference.
///
/// This performs only type checks and cloning; it does not add any binding-specific logic,
/// keeping bindings as thin passthroughs to the Rust implementations.
///
/// Note: Since wasm_bindgen doesn't automatically implement `JsCast` for structs with private fields,
/// we use `unchecked_ref` with runtime type checking via constructor name matching.
/// This is safe because we verify the type before casting.
#[allow(unsafe_code)]
pub(crate) fn extract_instrument(value: &JsValue) -> Result<Box<dyn Instrument>, JsValue> {
    macro_rules! try_extract {
        ($js_type:ty, $js_name:expr) => {{
            // Check if the value is an instance of the expected type by checking constructor name
            let is_instance = Reflect::get(value, &JsValue::from_str("constructor"))
                .ok()
                .and_then(|c| Reflect::get(&c, &JsValue::from_str("name")).ok())
                .and_then(|n| n.as_string())
                .map(|n| n == $js_name)
                .unwrap_or(false);

            if is_instance {
                // Safe because we've verified the type via constructor name check
                // JsValue and wasm_bindgen structs are both pointer-sized, so we can cast
                let inst: &$js_type = unsafe { &*(value as *const JsValue as *const $js_type) };
                return Ok(Box::new(inst.inner()));
            }
        }};
    }

    try_extract!(agency_mbs::JsAgencyMbsPassthrough, "AgencyMbsPassthrough");
    try_extract!(agency_mbs::JsAgencyTba, "AgencyTba");
    try_extract!(agency_mbs::JsDollarRoll, "DollarRoll");
    try_extract!(agency_mbs::JsAgencyCmo, "AgencyCmo");
    try_extract!(bond::JsBond, "Bond");
    try_extract!(bond_future::JsBondFuture, "BondFuture");
    try_extract!(commodity_option::JsCommodityOption, "CommodityOption");
    try_extract!(deposit::JsDeposit, "Deposit");
    try_extract!(basis_swap::JsBasisSwap, "BasisSwap");
    try_extract!(fra::JsForwardRateAgreement, "ForwardRateAgreement");
    try_extract!(cap_floor::JsInterestRateOption, "InterestRateOption");
    try_extract!(ir_future::JsInterestRateFuture, "InterestRateFuture");
    try_extract!(irs::JsInterestRateSwap, "InterestRateSwap");
    try_extract!(fx::JsFxSpot, "FxSpot");
    try_extract!(fx::JsFxOption, "FxOption");
    try_extract!(fx::JsFxSwap, "FxSwap");
    try_extract!(equity::JsEquity, "Equity");
    try_extract!(equity_index_future::JsEquityIndexFuture, "EquityIndexFuture");
    try_extract!(equity_option::JsEquityOption, "EquityOption");
    try_extract!(convertible::JsConvertibleBond, "ConvertibleBond");
    try_extract!(swaption::JsSwaption, "Swaption");
    try_extract!(trs::JsEquityTotalReturnSwap, "EquityTotalReturnSwap");
    try_extract!(trs::JsFiIndexTotalReturnSwap, "FiIndexTotalReturnSwap");
    try_extract!(variance_swap::JsVarianceSwap, "VarianceSwap");
    try_extract!(cds::JsCreditDefaultSwap, "CreditDefaultSwap");
    try_extract!(cds_index::JsCDSIndex, "CDSIndex");
    try_extract!(cds_option::JsCdsOption, "CdsOption");
    try_extract!(cds_tranche::JsCdsTranche, "CdsTranche");
    try_extract!(repo::JsRepo, "Repo");
    try_extract!(
        inflation_linked_bond::JsInflationLinkedBond,
        "InflationLinkedBond"
    );
    try_extract!(inflation_swap::JsInflationSwap, "InflationSwap");
    try_extract!(structured_credit::JsStructuredCredit, "StructuredCredit");
    try_extract!(
        private_markets_fund::JsPrivateMarketsFund,
        "PrivateMarketsFund"
    );
    try_extract!(structured_credit::JsBasket, "Basket");
    try_extract!(asian_option::JsAsianOption, "AsianOption");
    try_extract!(autocallable::JsAutocallable, "Autocallable");
    try_extract!(barrier_option::JsBarrierOption, "BarrierOption");
    try_extract!(cliquet_option::JsCliquetOption, "CliquetOption");
    try_extract!(cms_option::JsCmsOption, "CmsOption");
    try_extract!(commodity_forward::JsCommodityForward, "CommodityForward");
    try_extract!(commodity_swap::JsCommoditySwap, "CommoditySwap");
    try_extract!(fx_barrier_option::JsFxBarrierOption, "FxBarrierOption");
    try_extract!(fx_forward::JsFxForward, "FxForward");
    try_extract!(fx_variance_swap::JsFxVarianceSwap, "FxVarianceSwap");
    try_extract!(lookback_option::JsLookbackOption, "LookbackOption");
    try_extract!(ndf::JsNdf, "Ndf");
    try_extract!(quanto_option::JsQuantoOption, "QuantoOption");
    try_extract!(range_accrual::JsRangeAccrual, "RangeAccrual");
    try_extract!(revolving_credit::JsRevolvingCredit, "RevolvingCredit");
    try_extract!(term_loan::JsTermLoan, "TermLoan");
    try_extract!(
        vol_index_future::JsVolatilityIndexFuture,
        "VolatilityIndexFuture"
    );
    try_extract!(
        vol_index_option::JsVolatilityIndexOption,
        "VolatilityIndexOption"
    );
    try_extract!(xccy_swap::JsXccySwap, "XccySwap");
    try_extract!(
        yoy_inflation_swap::JsYoYInflationSwap,
        "YoYInflationSwap"
    );
    try_extract!(
        inflation_cap_floor::JsInflationCapFloor,
        "InflationCapFloor"
    );
    try_extract!(real_estate::JsRealEstateAsset, "RealEstateAsset");

    Err(JsValue::from_str(
        "Unsupported instrument type; construct instruments from finstack-wasm valuations module",
    ))
}
