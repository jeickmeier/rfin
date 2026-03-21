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
mod levered_real_estate_equity;
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

use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use js_sys::Reflect;
use wasm_bindgen::JsValue;
use wasm_bindgen::__rt::WasmRefCell;

#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use asian_option::JsAsianOptionBuilder as AsianOptionBuilder;
pub use asian_option::{JsAsianOption as AsianOption, JsAveragingMethod as AveragingMethod};
pub use autocallable::JsAutocallable as Autocallable;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use autocallable::JsAutocallableBuilder as AutocallableBuilder;
pub use barrier_option::JsBarrierOption as BarrierOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use barrier_option::JsBarrierOptionBuilder as BarrierOptionBuilder;
pub use basis_swap::JsBasisSwap as BasisSwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use basis_swap::JsBasisSwapBuilder as BasisSwapBuilder;
pub use bond::JsBond as Bond;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use bond::JsBondBuilder as BondBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use bond_future::JsBondFutureBuilder as BondFutureBuilder;
pub use bond_future::{
    JsBondFuture as BondFuture, JsBondFutureSpecs as BondFutureSpecs,
    JsFuturePosition as FuturePosition,
};
// Also export JsBond name for use within this crate
pub use bond::JsBond;
pub use cap_floor::JsInterestRateOption as InterestRateOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cap_floor::JsInterestRateOptionBuilder as InterestRateOptionBuilder;
pub use cds::JsCreditDefaultSwap as CreditDefaultSwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cds::JsCreditDefaultSwapBuilder as CreditDefaultSwapBuilder;
pub use cds_index::JsCDSIndex as CDSIndex;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cds_index::JsCDSIndexBuilder as CDSIndexBuilder;
pub use cds_option::JsCDSOption as CDSOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cds_option::JsCDSOptionBuilder as CDSOptionBuilder;
pub use cds_tranche::JsCDSTranche as CDSTranche;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cds_tranche::JsCDSTrancheBuilder as CDSTrancheBuilder;
pub use cliquet_option::JsCliquetOption as CliquetOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cliquet_option::JsCliquetOptionBuilder as CliquetOptionBuilder;
pub use cms_option::JsCmsOption as CmsOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use cms_option::JsCmsOptionBuilder as CmsOptionBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use commodity_forward::JsCommodityForward as CommodityForward;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use commodity_forward::JsCommodityForwardBuilder as CommodityForwardBuilder;
pub use commodity_option::JsCommodityOption as CommodityOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use commodity_option::JsCommodityOptionBuilder as CommodityOptionBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use commodity_swap::JsCommoditySwap as CommoditySwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use commodity_swap::JsCommoditySwapBuilder as CommoditySwapBuilder;
// Commodity instruments: exported directly via wasm_bindgen
pub use convertible::JsConvertibleBond as ConvertibleBond;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use convertible::JsConvertibleBondBuilder as ConvertibleBondBuilder;
pub use dcf::evaluate_dcf_wasm;
pub use deposit::JsDeposit as Deposit;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use deposit::JsDepositBuilder as DepositBuilder;
pub use equity::JsEquity as Equity;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use equity::JsEquityBuilder as EquityBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use equity_index_future::JsEquityIndexFutureBuilder as EquityIndexFutureBuilder;
pub use equity_index_future::{
    JsEquityFutureSpecs as EquityFutureSpecs, JsEquityIndexFuture as EquityIndexFuture,
};
pub use equity_option::JsEquityOption as EquityOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use equity_option::JsEquityOptionBuilder as EquityOptionBuilder;
pub use fra::JsForwardRateAgreement as ForwardRateAgreement;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use fra::JsForwardRateAgreementBuilder as ForwardRateAgreementBuilder;
pub use fx::{JsFxOption as FxOption, JsFxSpot as FxSpot, JsFxSwap as FxSwap};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use fx::{
    JsFxOptionBuilder as FxOptionBuilder, JsFxSpotBuilder as FxSpotBuilder,
    JsFxSwapBuilder as FxSwapBuilder,
};
pub use fx_barrier_option::JsFxBarrierOption as FxBarrierOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use fx_barrier_option::JsFxBarrierOptionBuilder as FxBarrierOptionBuilder;
pub use fx_forward::JsFxForward as FxForward;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use fx_forward::JsFxForwardBuilder as FxForwardBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use fx_variance_swap::JsFxVarianceSwapBuilder as FxVarianceSwapBuilder;
pub use fx_variance_swap::{
    JsFxVarianceSwap as FxVarianceSwap, JsVarianceSwapSide as VarianceSwapSide,
};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use inflation_cap_floor::JsInflationCapFloorBuilder as InflationCapFloorBuilder;
pub use inflation_cap_floor::{
    JsInflationCapFloor as InflationCapFloor, JsInflationCapFloorType as InflationCapFloorType,
};
pub use inflation_linked_bond::JsInflationLinkedBond as InflationLinkedBond;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use inflation_linked_bond::JsInflationLinkedBondBuilder as InflationLinkedBondBuilder;
pub use inflation_swap::JsInflationSwap as InflationSwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use inflation_swap::JsInflationSwapBuilder as InflationSwapBuilder;
pub use ir_future::JsInterestRateFuture as InterestRateFuture;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use ir_future::JsInterestRateFutureBuilder as InterestRateFutureBuilder;
pub use irs::JsInterestRateSwap as InterestRateSwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use irs::JsInterestRateSwapBuilder as InterestRateSwapBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use levered_real_estate_equity::JsLeveredRealEstateEquity as LeveredRealEstateEquity;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use levered_real_estate_equity::JsLeveredRealEstateEquityBuilder as LeveredRealEstateEquityBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use lookback_option::JsLookbackOptionBuilder as LookbackOptionBuilder;
pub use lookback_option::{JsLookbackOption as LookbackOption, JsLookbackType as LookbackType};
pub use ndf::JsNdf as Ndf;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use ndf::JsNdfBuilder as NdfBuilder;
pub use private_markets_fund::JsPrivateMarketsFund as PrivateMarketsFund;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use private_markets_fund::JsPrivateMarketsFundBuilder as PrivateMarketsFundBuilder;
pub use quanto_option::JsQuantoOption as QuantoOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use quanto_option::JsQuantoOptionBuilder as QuantoOptionBuilder;
pub use range_accrual::JsRangeAccrual as RangeAccrual;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use range_accrual::JsRangeAccrualBuilder as RangeAccrualBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use real_estate::JsRealEstateAssetBuilder as RealEstateAssetBuilder;
pub use real_estate::{
    JsRealEstateAsset as RealEstateAsset, JsRealEstateValuationMethod as RealEstateValuationMethod,
};
pub use repo::JsRepo as Repo;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use repo::JsRepoBuilder as RepoBuilder;
pub use revolving_credit::JsRevolvingCredit as RevolvingCredit;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use revolving_credit::JsRevolvingCreditBuilder as RevolvingCreditBuilder;
pub use structured_credit::{
    JsBasket as Basket, JsCoverageTestRules as CoverageTestRules,
    JsCoverageTrigger as CoverageTrigger, JsPool as Pool, JsStructuredCredit as StructuredCredit,
    JsTrancheStructure as TrancheStructure, JsWaterfall as WaterfallEngine,
    JsWaterfallDistribution as WaterfallDistribution,
};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use structured_credit::{
    JsBasketBuilder as BasketBuilder, JsStructuredCreditBuilder as StructuredCreditBuilder,
};
pub use swaption::JsSwaption as Swaption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use swaption::JsSwaptionBuilder as SwaptionBuilder;
pub use term_loan::JsTermLoan as TermLoan;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use term_loan::JsTermLoanBuilder as TermLoanBuilder;
pub use trs::{
    JsEquityTotalReturnSwap as EquityTotalReturnSwap,
    JsFiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use trs::{
    JsEquityTotalReturnSwapBuilder as EquityTotalReturnSwapBuilder,
    JsFiIndexTotalReturnSwapBuilder as FiIndexTotalReturnSwapBuilder,
};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use variance_swap::JsVarianceSwapBuilder as VarianceSwapBuilder;
pub use variance_swap::{JsRealizedVarMethod as RealizedVarMethod, JsVarianceSwap as VarianceSwap};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_future::JsVolatilityIndexFuture as VolatilityIndexFuture;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_future::JsVolatilityIndexFutureBuilder as VolatilityIndexFutureBuilder;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_option::JsVolatilityIndexOption as VolatilityIndexOption;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use vol_index_option::JsVolatilityIndexOptionBuilder as VolatilityIndexOptionBuilder;
pub use xccy_swap::{
    JsLegSide as LegSide, JsNotionalExchange as NotionalExchange, JsXccySwap as XccySwap,
    JsXccySwapLeg as XccySwapLeg,
};
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use xccy_swap::{
    JsXccySwapBuilder as XccySwapBuilder, JsXccySwapLegBuilder as XccySwapLegBuilder,
};
pub use yoy_inflation_swap::JsYoYInflationSwap as YoYInflationSwap;
#[allow(unused_imports)] // Exported for external consumers via wasm_bindgen
pub use yoy_inflation_swap::JsYoYInflationSwapBuilder as YoYInflationSwapBuilder;

/// Downcast a JavaScript instrument wrapper into a core instrument reference.
///
/// This performs only type checks and cloning; it does not add any binding-specific logic,
/// keeping bindings as thin passthroughs to the Rust implementations.
///
/// Note: These wrappers currently do not implement `JsCast`, so extraction uses
/// constructor-name checks and wasm-bindgen-managed pointers.
#[allow(unsafe_code)]
pub(crate) fn extract_instrument(value: &JsValue) -> Result<Box<dyn Instrument>, JsValue> {
    macro_rules! try_extract {
        ($js_type:ty, $js_name:expr) => {{
            // Check if the value is an instance of the expected type by checking constructor name.
            let is_instance = Reflect::get(value, &JsValue::from_str("constructor"))
                .ok()
                .and_then(|c| Reflect::get(&c, &JsValue::from_str("name")).ok())
                .and_then(|n| n.as_string())
                .map(|n| n == $js_name)
                .unwrap_or(false);

            if is_instance {
                // wasm-bindgen stores a pointer to a `WasmRefCell<T>` in `__wbg_ptr`.
                let ptr_val = Reflect::get(value, &JsValue::from_str("__wbg_ptr"))
                    .or_else(|_| Reflect::get(value, &JsValue::from_str("ptr")))
                    .map_err(|_| JsValue::from_str("Could not find Rust pointer"))?;

                let ptr_f64 = ptr_val
                    .as_f64()
                    .ok_or_else(|| JsValue::from_str("Pointer is not a number"))?;

                // wasm32 pointers are u32; JS numbers can represent all u32 exactly.
                if !ptr_f64.is_finite() || ptr_f64 < 0.0 || ptr_f64.fract() != 0.0 {
                    return Err(JsValue::from_str("Rust pointer is not a valid u32"));
                }
                if ptr_f64 > (u32::MAX as f64) {
                    return Err(JsValue::from_str("Rust pointer out of range"));
                }

                let ptr_u32 = ptr_f64 as u32;
                let cell_ptr = ptr_u32 as usize as *const WasmRefCell<$js_type>;
                if cell_ptr.is_null() || (cell_ptr as usize) < 0x1000 {
                    return Err(JsValue::from_str("Rust pointer is invalid"));
                }

                // SAFETY: pointer is managed by wasm-bindgen for this wrapper type.
                let cell = unsafe { &*cell_ptr };
                let borrowed = cell.borrow();
                return Ok(Box::new(borrowed.inner()));
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
    try_extract!(
        equity_index_future::JsEquityIndexFuture,
        "EquityIndexFuture"
    );
    try_extract!(equity_option::JsEquityOption, "EquityOption");
    try_extract!(convertible::JsConvertibleBond, "ConvertibleBond");
    try_extract!(swaption::JsSwaption, "Swaption");
    try_extract!(trs::JsEquityTotalReturnSwap, "EquityTotalReturnSwap");
    try_extract!(trs::JsFiIndexTotalReturnSwap, "FiIndexTotalReturnSwap");
    try_extract!(variance_swap::JsVarianceSwap, "VarianceSwap");
    try_extract!(cds::JsCreditDefaultSwap, "CreditDefaultSwap");
    try_extract!(cds_index::JsCDSIndex, "CDSIndex");
    try_extract!(cds_option::JsCDSOption, "CDSOption");
    try_extract!(cds_tranche::JsCDSTranche, "CDSTranche");
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
    try_extract!(yoy_inflation_swap::JsYoYInflationSwap, "YoYInflationSwap");
    try_extract!(
        inflation_cap_floor::JsInflationCapFloor,
        "InflationCapFloor"
    );
    try_extract!(real_estate::JsRealEstateAsset, "RealEstateAsset");
    try_extract!(
        levered_real_estate_equity::JsLeveredRealEstateEquity,
        "LeveredRealEstateEquity"
    );

    Err(JsValue::from_str(
        "Unsupported instrument type; construct instruments from finstack-wasm valuations module",
    ))
}
