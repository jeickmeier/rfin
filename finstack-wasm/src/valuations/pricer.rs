use crate::core::market_data::context::JsMarketContext;
use crate::core::utils::js_error;
use crate::valuations::instruments::{
    Abs as JsAbs, BasisSwap as JsBasisSwap, Basket as JsBasket, Bond as JsBond, CDSIndex as JsCDSIndex,
    CdsOption as JsCdsOption, CdsTranche as JsCdsTranche, Clo as JsClo, Cmbs as JsCmbs,
    ConvertibleBond as JsConvertibleBond, CreditDefaultSwap as JsCreditDefaultSwap,
    Deposit as JsDeposit, Equity as JsEquity, EquityOption as JsEquityOption,
    EquityTotalReturnSwap as JsEquityTotalReturnSwap, FiIndexTotalReturnSwap as JsFiIndexTotalReturnSwap,
    ForwardRateAgreement as JsForwardRateAgreement, FxOption as JsFxOption, FxSpot as JsFxSpot,
    FxSwap as JsFxSwap, InflationLinkedBond as JsInflationLinkedBond,
    InflationSwap as JsInflationSwap, InterestRateFuture as JsInterestRateFuture,
    InterestRateOption as JsInterestRateOption, InterestRateSwap as JsInterestRateSwap,
    PrivateMarketsFund as JsPrivateMarketsFund, Repo as JsRepo, Rmbs as JsRmbs, Swaption as JsSwaption,
    VarianceSwap as JsVarianceSwap,
};
use crate::valuations::results::JsValuationResult;
use finstack_valuations::instruments::build_with_metrics_dyn;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::{create_standard_registry, ModelKey, PricerRegistry};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

fn parse_model_key(model: &str) -> Result<ModelKey, JsValue> {
    ModelKey::from_str(model).map_err(js_error)
}

fn metrics_from_array(array: &js_sys::Array) -> Vec<MetricId> {
    array
        .iter()
        .filter_map(|value| value.as_string())
        .map(|name| MetricId::from_str(&name).unwrap_or_else(|_| MetricId::custom(name)))
        .collect()
}

fn pricing_error_to_js(err: finstack_valuations::pricer::PricingError) -> JsValue {
    js_error(err.to_string())
}

fn core_error_to_js(err: finstack_core::Error) -> JsValue {
    js_error(err.to_string())
}

fn price_with_optional_metrics(
    registry: &PricerRegistry,
    instrument: &dyn Instrument,
    model_key: ModelKey,
    market: &JsMarketContext,
    metrics: Option<&js_sys::Array>,
) -> Result<JsValuationResult, JsValue> {
    let base = registry
        .price_with_registry(instrument, model_key, market.inner())
        .map_err(pricing_error_to_js)?;

    if let Some(list) = metrics {
        if list.length() == 0 {
            return Ok(JsValuationResult::new(base));
        }
        let metric_ids = metrics_from_array(list);
        return build_with_metrics_dyn(
            instrument,
            market.inner(),
            base.as_of,
            base.value,
            &metric_ids,
        )
        .map(JsValuationResult::new)
        .map_err(core_error_to_js);
    }

    Ok(JsValuationResult::new(base))
}

#[wasm_bindgen(js_name = PricerRegistry)]
pub struct JsPricerRegistry {
    inner: PricerRegistry,
}

impl JsPricerRegistry {
    pub(crate) fn new(inner: PricerRegistry) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = PricerRegistry)]
impl JsPricerRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new_empty() -> JsPricerRegistry {
        JsPricerRegistry::new(PricerRegistry::new())
    }

    #[wasm_bindgen(js_name = priceBond)]
    pub fn price_bond(
        &self,
        bond: &JsBond,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceBondWithMetrics)]
    pub fn price_bond_with_metrics(
        &self,
        bond: &JsBond,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    #[wasm_bindgen(js_name = priceDeposit)]
    pub fn price_deposit(
        &self,
        deposit: &JsDeposit,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = deposit.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceDepositWithMetrics)]
    pub fn price_deposit_with_metrics(
        &self,
        deposit: &JsDeposit,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = deposit.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Interest Rate Swap
    #[wasm_bindgen(js_name = priceInterestRateSwap)]
    pub fn price_interest_rate_swap(
        &self,
        swap: &JsInterestRateSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceInterestRateSwapWithMetrics)]
    pub fn price_interest_rate_swap_with_metrics(
        &self,
        swap: &JsInterestRateSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Forward Rate Agreement
    #[wasm_bindgen(js_name = priceForwardRateAgreement)]
    pub fn price_forward_rate_agreement(
        &self,
        fra: &JsForwardRateAgreement,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fra.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceForwardRateAgreementWithMetrics)]
    pub fn price_forward_rate_agreement_with_metrics(
        &self,
        fra: &JsForwardRateAgreement,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fra.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Swaption
    #[wasm_bindgen(js_name = priceSwaption)]
    pub fn price_swaption(
        &self,
        swaption: &JsSwaption,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swaption.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceSwaptionWithMetrics)]
    pub fn price_swaption_with_metrics(
        &self,
        swaption: &JsSwaption,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swaption.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Basis Swap
    #[wasm_bindgen(js_name = priceBasisSwap)]
    pub fn price_basis_swap(
        &self,
        swap: &JsBasisSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceBasisSwapWithMetrics)]
    pub fn price_basis_swap_with_metrics(
        &self,
        swap: &JsBasisSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Interest Rate Option (Cap/Floor)
    #[wasm_bindgen(js_name = priceInterestRateOption)]
    pub fn price_interest_rate_option(
        &self,
        option: &JsInterestRateOption,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceInterestRateOptionWithMetrics)]
    pub fn price_interest_rate_option_with_metrics(
        &self,
        option: &JsInterestRateOption,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Interest Rate Future
    #[wasm_bindgen(js_name = priceInterestRateFuture)]
    pub fn price_interest_rate_future(
        &self,
        future: &JsInterestRateFuture,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = future.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceInterestRateFutureWithMetrics)]
    pub fn price_interest_rate_future_with_metrics(
        &self,
        future: &JsInterestRateFuture,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = future.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // FX Spot
    #[wasm_bindgen(js_name = priceFxSpot)]
    pub fn price_fx_spot(
        &self,
        fx_spot: &JsFxSpot,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_spot.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceFxSpotWithMetrics)]
    pub fn price_fx_spot_with_metrics(
        &self,
        fx_spot: &JsFxSpot,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_spot.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // FX Option
    #[wasm_bindgen(js_name = priceFxOption)]
    pub fn price_fx_option(
        &self,
        fx_option: &JsFxOption,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceFxOptionWithMetrics)]
    pub fn price_fx_option_with_metrics(
        &self,
        fx_option: &JsFxOption,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // FX Swap
    #[wasm_bindgen(js_name = priceFxSwap)]
    pub fn price_fx_swap(
        &self,
        fx_swap: &JsFxSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceFxSwapWithMetrics)]
    pub fn price_fx_swap_with_metrics(
        &self,
        fx_swap: &JsFxSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Credit Default Swap
    #[wasm_bindgen(js_name = priceCreditDefaultSwap)]
    pub fn price_credit_default_swap(
        &self,
        cds: &JsCreditDefaultSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = cds.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceCreditDefaultSwapWithMetrics)]
    pub fn price_credit_default_swap_with_metrics(
        &self,
        cds: &JsCreditDefaultSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = cds.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // CDS Index
    #[wasm_bindgen(js_name = priceCDSIndex)]
    pub fn price_cds_index(
        &self,
        index: &JsCDSIndex,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = index.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceCDSIndexWithMetrics)]
    pub fn price_cds_index_with_metrics(
        &self,
        index: &JsCDSIndex,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = index.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // CDS Tranche
    #[wasm_bindgen(js_name = priceCdsTranche)]
    pub fn price_cds_tranche(
        &self,
        tranche: &JsCdsTranche,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = tranche.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceCdsTrancheWithMetrics)]
    pub fn price_cds_tranche_with_metrics(
        &self,
        tranche: &JsCdsTranche,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = tranche.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // CDS Option
    #[wasm_bindgen(js_name = priceCdsOption)]
    pub fn price_cds_option(
        &self,
        option: &JsCdsOption,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceCdsOptionWithMetrics)]
    pub fn price_cds_option_with_metrics(
        &self,
        option: &JsCdsOption,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Equity
    #[wasm_bindgen(js_name = priceEquity)]
    pub fn price_equity(
        &self,
        equity: &JsEquity,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = equity.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceEquityWithMetrics)]
    pub fn price_equity_with_metrics(
        &self,
        equity: &JsEquity,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = equity.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Equity Option
    #[wasm_bindgen(js_name = priceEquityOption)]
    pub fn price_equity_option(
        &self,
        option: &JsEquityOption,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceEquityOptionWithMetrics)]
    pub fn price_equity_option_with_metrics(
        &self,
        option: &JsEquityOption,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Repo
    #[wasm_bindgen(js_name = priceRepo)]
    pub fn price_repo(
        &self,
        repo: &JsRepo,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = repo.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceRepoWithMetrics)]
    pub fn price_repo_with_metrics(
        &self,
        repo: &JsRepo,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = repo.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Inflation Linked Bond
    #[wasm_bindgen(js_name = priceInflationLinkedBond)]
    pub fn price_inflation_linked_bond(
        &self,
        bond: &JsInflationLinkedBond,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceInflationLinkedBondWithMetrics)]
    pub fn price_inflation_linked_bond_with_metrics(
        &self,
        bond: &JsInflationLinkedBond,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Inflation Swap
    #[wasm_bindgen(js_name = priceInflationSwap)]
    pub fn price_inflation_swap(
        &self,
        swap: &JsInflationSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceInflationSwapWithMetrics)]
    pub fn price_inflation_swap_with_metrics(
        &self,
        swap: &JsInflationSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Variance Swap
    #[wasm_bindgen(js_name = priceVarianceSwap)]
    pub fn price_variance_swap(
        &self,
        swap: &JsVarianceSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceVarianceSwapWithMetrics)]
    pub fn price_variance_swap_with_metrics(
        &self,
        swap: &JsVarianceSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Convertible Bond
    #[wasm_bindgen(js_name = priceConvertibleBond)]
    pub fn price_convertible_bond(
        &self,
        bond: &JsConvertibleBond,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceConvertibleBondWithMetrics)]
    pub fn price_convertible_bond_with_metrics(
        &self,
        bond: &JsConvertibleBond,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Equity TRS
    #[wasm_bindgen(js_name = priceEquityTotalReturnSwap)]
    pub fn price_equity_total_return_swap(
        &self,
        trs: &JsEquityTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceEquityTotalReturnSwapWithMetrics)]
    pub fn price_equity_total_return_swap_with_metrics(
        &self,
        trs: &JsEquityTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // FI Index TRS
    #[wasm_bindgen(js_name = priceFiIndexTotalReturnSwap)]
    pub fn price_fi_index_total_return_swap(
        &self,
        trs: &JsFiIndexTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceFiIndexTotalReturnSwapWithMetrics)]
    pub fn price_fi_index_total_return_swap_with_metrics(
        &self,
        trs: &JsFiIndexTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    // Structured Products - JSON-based
    #[wasm_bindgen(js_name = priceAbs)]
    pub fn price_abs(
        &self,
        abs: &JsAbs,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = abs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceClo)]
    pub fn price_clo(
        &self,
        clo: &JsClo,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = clo.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceCmbs)]
    pub fn price_cmbs(
        &self,
        cmbs: &JsCmbs,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = cmbs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceRmbs)]
    pub fn price_rmbs(
        &self,
        rmbs: &JsRmbs,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = rmbs.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceBasket)]
    pub fn price_basket(
        &self,
        basket: &JsBasket,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = basket.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = pricePrivateMarketsFund)]
    pub fn price_private_markets_fund(
        &self,
        fund: &JsPrivateMarketsFund,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fund.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }
}

#[wasm_bindgen(js_name = createStandardRegistry)]
pub fn create_standard_registry_js() -> JsPricerRegistry {
    JsPricerRegistry::new(create_standard_registry())
}
