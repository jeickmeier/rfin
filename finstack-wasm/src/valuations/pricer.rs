use crate::core::error::core_to_js;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::valuations::instruments::{
    AsianOption as JsAsianOption, Autocallable as JsAutocallable, BarrierOption as JsBarrierOption,
    BasisSwap as JsBasisSwap, Basket as JsBasket, Bond as JsBond, CDSIndex as JsCDSIndex,
    CdsOption as JsCdsOption, CdsTranche as JsCdsTranche, CliquetOption as JsCliquetOption,
    CmsOption as JsCmsOption, ConvertibleBond as JsConvertibleBond,
    CreditDefaultSwap as JsCreditDefaultSwap, Deposit as JsDeposit, Equity as JsEquity,
    EquityOption as JsEquityOption, EquityTotalReturnSwap as JsEquityTotalReturnSwap,
    FiIndexTotalReturnSwap as JsFiIndexTotalReturnSwap, ForwardRateAgreement as JsForwardRateAgreement,
    FxBarrierOption as JsFxBarrierOption, FxOption as JsFxOption, FxSpot as JsFxSpot,
    FxSwap as JsFxSwap, InflationLinkedBond as JsInflationLinkedBond,
    InflationSwap as JsInflationSwap, InstrumentWrapper,
    InterestRateFuture as JsInterestRateFuture, InterestRateOption as JsInterestRateOption,
    InterestRateSwap as JsInterestRateSwap, LookbackOption as JsLookbackOption,
    PrivateMarketsFund as JsPrivateMarketsFund, QuantoOption as JsQuantoOption,
    RangeAccrual as JsRangeAccrual, Repo as JsRepo, RevolvingCredit as JsRevolvingCredit,
    StructuredCredit as JsStructuredCredit, Swaption as JsSwaption, VarianceSwap as JsVarianceSwap,
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
    core_to_js(err)
}

fn price_with_optional_metrics(
    registry: &PricerRegistry,
    instrument: &dyn Instrument,
    model_key: ModelKey,
    market: &JsMarketContext,
    metrics: Option<Vec<MetricId>>,
) -> Result<JsValuationResult, JsValue> {
    // Extract as_of date from the first discount curve in the market context
    // This ensures pricing uses the correct valuation date that matches the curves
    let as_of = market
        .inner()
        .curves_of_type("Discount")
        .next()
        .and_then(|(_, storage)| match storage {
            finstack_core::market_data::context::CurveStorage::Discount(curve) => {
                Some(curve.base_date())
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            // Fallback to default if no discount curves are available
            finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap()
        });

    let base = registry
        .price_with_registry(instrument, model_key, market.inner(), as_of)
        .map_err(pricing_error_to_js)?;

    if let Some(metric_ids) = metrics {
        if metric_ids.is_empty() {
            return Ok(JsValuationResult::new(base));
        }
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

/// Configuration for pricing requests with optional metrics.
///
/// Use this builder to specify additional risk metrics to compute alongside
/// the base present value. Without metrics, only PV is returned.
///
/// @example
/// ```javascript
/// // Price with metrics
/// const opts = new PricingRequest().withMetrics(["dv01", "duration_mod", "ytm"]);
/// const result = registry.priceBond(bond, "discounting", market, opts);
///
/// // Access computed metrics
/// console.log(`DV01: ${result.metric("dv01")}`);
/// console.log(`Duration: ${result.metric("duration_mod")}`);
/// ```
#[wasm_bindgen(js_name = PricingRequest)]
pub struct JsPricingRequest {
    metrics: Option<Vec<MetricId>>,
}

impl Default for JsPricingRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = PricingRequest)]
impl JsPricingRequest {
    /// Create a new pricing request with no additional metrics.
    ///
    /// @returns {PricingRequest} Empty request (only PV will be computed)
    ///
    /// @example
    /// ```javascript
    /// const opts = new PricingRequest();
    /// const result = registry.priceBond(bond, "discounting", market, opts);
    /// // result.presentValue is available, but no metrics
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { metrics: None }
    }

    /// Add risk metrics to compute.
    ///
    /// @param {Array<string>} metrics - Metric names (e.g., ["dv01", "ytm", "duration_mod"])
    /// @returns {PricingRequest} Self for chaining
    ///
    /// @example
    /// ```javascript
    /// const opts = new PricingRequest()
    ///   .withMetrics(["clean_price", "accrued", "ytm", "dv01", "z_spread"]);
    /// const result = registry.priceBond(bond, "discounting", market, opts);
    /// ```
    #[wasm_bindgen(js_name = withMetrics)]
    pub fn with_metrics(mut self, metrics: js_sys::Array) -> Self {
        self.metrics = Some(metrics_from_array(&metrics));
        self
    }
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
    /// Create an empty pricing registry (use createStandardRegistry() for pre-loaded engines).
    ///
    /// @returns {PricerRegistry} Registry without any registered pricing engines
    ///
    /// @example
    /// ```javascript
    /// // Typically use the standard registry instead:
    /// const registry = createStandardRegistry();
    ///
    /// // But you can create an empty one for custom engines:
    /// const custom = new PricerRegistry();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new_empty() -> JsPricerRegistry {
        JsPricerRegistry::new(PricerRegistry::new())
    }

    /// Price a bond instrument using the specified model and market data.
    ///
    /// @param {Bond} bond - Bond instrument created via Bond constructors
    /// @param {string} model - Pricing model key ("discounting", "tree", etc.)
    /// @param {MarketContext} market - Market data context with curves and scalars
    /// @param {PricingRequest?} opts - Optional pricing configuration for risk metrics
    /// @returns {ValuationResult} Pricing result with present value and metadata
    /// @throws {Error} If the model is unsupported or required market data is missing
    ///
    /// @example
    /// ```javascript
    /// const registry = createStandardRegistry();
    /// const bond = Bond.fixedSemiannual("bond1", notional, 0.05, issue, maturity, "USD-OIS");
    /// const market = new MarketContext();
    /// market.insertDiscount(discountCurve);
    ///
    /// // Price without metrics
    /// const result = registry.priceBond(bond, "discounting", market);
    /// console.log(result.presentValue.format());  // "USD 1,023,456.78"
    ///
    /// // Price with metrics
    /// const opts = new PricingRequest().withMetrics(["dv01", "ytm", "duration_mod"]);
    /// const resultWithMetrics = registry.priceBond(bond, "discounting", market, opts);
    /// console.log(`DV01: ${resultWithMetrics.metric("dv01")}`);
    /// ```
    #[wasm_bindgen(js_name = priceBond)]
    pub fn price_bond(
        &self,
        bond: &JsBond,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceDeposit)]
    pub fn price_deposit(
        &self,
        deposit: &JsDeposit,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = deposit.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceInterestRateSwap)]
    pub fn price_interest_rate_swap(
        &self,
        swap: &JsInterestRateSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceRangeAccrual)]
    pub fn price_range_accrual(
        &self,
        range_accrual: &JsRangeAccrual,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = range_accrual.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceForwardRateAgreement)]
    pub fn price_forward_rate_agreement(
        &self,
        fra: &JsForwardRateAgreement,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fra.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceSwaption)]
    pub fn price_swaption(
        &self,
        swaption: &JsSwaption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swaption.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCmsOption)]
    pub fn price_cms_option(
        &self,
        cms_option: &JsCmsOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = cms_option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceBasisSwap)]
    pub fn price_basis_swap(
        &self,
        swap: &JsBasisSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceInterestRateOption)]
    pub fn price_interest_rate_option(
        &self,
        option: &JsInterestRateOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceInterestRateFuture)]
    pub fn price_interest_rate_future(
        &self,
        future: &JsInterestRateFuture,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = future.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceFxSpot)]
    pub fn price_fx_spot(
        &self,
        fx_spot: &JsFxSpot,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_spot.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceFxOption)]
    pub fn price_fx_option(
        &self,
        fx_option: &JsFxOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceFxBarrierOption)]
    pub fn price_fx_barrier_option(
        &self,
        fx_barrier_option: &JsFxBarrierOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_barrier_option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceQuantoOption)]
    pub fn price_quanto_option(
        &self,
        quanto_option: &JsQuantoOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = quanto_option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceFxSwap)]
    pub fn price_fx_swap(
        &self,
        fx_swap: &JsFxSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fx_swap.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCreditDefaultSwap)]
    pub fn price_credit_default_swap(
        &self,
        cds: &JsCreditDefaultSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = cds.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCDSIndex)]
    pub fn price_cds_index(
        &self,
        index: &JsCDSIndex,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = index.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCdsTranche)]
    pub fn price_cds_tranche(
        &self,
        tranche: &JsCdsTranche,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = tranche.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCdsOption)]
    pub fn price_cds_option(
        &self,
        option: &JsCdsOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceEquity)]
    pub fn price_equity(
        &self,
        equity: &JsEquity,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = equity.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceEquityOption)]
    pub fn price_equity_option(
        &self,
        option: &JsEquityOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceBarrierOption)]
    pub fn price_barrier_option(
        &self,
        option: &JsBarrierOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceAsianOption)]
    pub fn price_asian_option(
        &self,
        option: &JsAsianOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceLookbackOption)]
    pub fn price_lookback_option(
        &self,
        option: &JsLookbackOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceCliquetOption)]
    pub fn price_cliquet_option(
        &self,
        option: &JsCliquetOption,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = option.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceRepo)]
    pub fn price_repo(
        &self,
        repo: &JsRepo,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = repo.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceInflationLinkedBond)]
    pub fn price_inflation_linked_bond(
        &self,
        bond: &JsInflationLinkedBond,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceInflationSwap)]
    pub fn price_inflation_swap(
        &self,
        swap: &JsInflationSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceVarianceSwap)]
    pub fn price_variance_swap(
        &self,
        swap: &JsVarianceSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = swap.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceConvertibleBond)]
    pub fn price_convertible_bond(
        &self,
        bond: &JsConvertibleBond,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceEquityTotalReturnSwap)]
    pub fn price_equity_total_return_swap(
        &self,
        trs: &JsEquityTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceFiIndexTotalReturnSwap)]
    pub fn price_fi_index_total_return_swap(
        &self,
        trs: &JsFiIndexTotalReturnSwap,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = trs.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    // Structured Products - JSON-based
    #[wasm_bindgen(js_name = priceStructuredCredit)]
    pub fn price_structured_credit(
        &self,
        sc: &JsStructuredCredit,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = sc.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = priceBasket)]
    pub fn price_basket(
        &self,
        basket: &JsBasket,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = basket.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    #[wasm_bindgen(js_name = pricePrivateMarketsFund)]
    pub fn price_private_markets_fund(
        &self,
        fund: &JsPrivateMarketsFund,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = fund.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    /// Price an autocallable structured product using the specified model and market data.
    ///
    /// @param {Autocallable} autocallable - Autocallable instrument created via fromJson
    /// @param {string} model - Pricing model key ("monte_carlo_gbm", etc.)
    /// @param {MarketContext} market - Market data context with curves and scalars
    /// @param {PricingRequest?} opts - Optional pricing configuration for risk metrics
    /// @returns {ValuationResult} Pricing result with present value and metadata
    /// @throws {Error} If the model is unsupported or required market data is missing
    #[wasm_bindgen(js_name = priceAutocallable)]
    pub fn price_autocallable(
        &self,
        autocallable: &JsAutocallable,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = autocallable.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }

    /// Price a revolving credit facility using the specified model and market data.
    ///
    /// @param {RevolvingCredit} facility - Revolving credit facility created via fromJson
    /// @param {string} model - Pricing model key ("discounting", "monte_carlo_gbm", etc.)
    /// @param {MarketContext} market - Market data context with curves and scalars
    /// @param {PricingRequest?} opts - Optional pricing configuration for risk metrics
    /// @returns {ValuationResult} Pricing result with present value and metadata
    /// @throws {Error} If the model is unsupported or required market data is missing
    ///
    /// @example
    /// ```javascript
    /// const registry = createStandardRegistry();
    /// const facility = RevolvingCredit.fromJson(facilityJson);
    /// const market = new MarketContext();
    /// market.insertDiscount(discountCurve);
    ///
    /// // Price with deterministic spec
    /// const result = registry.priceRevolvingCredit(facility, "discounting", market);
    ///
    /// // Price with stochastic spec (requires monte_carlo_gbm)
    /// const mcResult = registry.priceRevolvingCredit(facility, "monte_carlo_gbm", market);
    /// ```
    #[wasm_bindgen(js_name = priceRevolvingCredit)]
    pub fn price_revolving_credit(
        &self,
        facility: &JsRevolvingCredit,
        model: &str,
        market: &JsMarketContext,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = facility.inner();
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, metrics)
    }
}

/// Create a pricing registry populated with all standard finstack pricers.
///
/// This is the main entry point for instrument valuation. The standard registry
/// includes pricing engines for all supported instrument types (bonds, swaps,
/// options, credit derivatives, etc.) using common models like discounting,
/// Black-76, and hazard rate approaches.
///
/// @returns {PricerRegistry} Registry with all built-in pricing engines loaded
///
/// @example
/// ```javascript
/// import { createStandardRegistry, Bond, Money, MarketContext, DiscountCurve, Date } from 'finstack-wasm';
///
/// const registry = createStandardRegistry();
///
/// // Create market data
/// const baseDate = new Date(2024, 1, 2);
/// const curve = new DiscountCurve("USD-OIS", baseDate, ...);
/// const market = new MarketContext();
/// market.insertDiscount(curve);
///
/// // Price instruments
/// const bond = Bond.fixedSemiannual(...);
/// const bondResult = registry.priceBond(bond, "discounting", market);
///
/// const swap = new InterestRateSwap(
///   'swap_1', notional, rate, start, end,
///   'USD-OIS', 'USD-SOFR-3M', 'receive_fixed', null, DayCount.thirty360(), null, DayCount.act360(), null, null, null, 2
/// );
/// const swapResult = registry.priceInterestRateSwapWithMetrics(
///   swap,
///   "discounting",
///   market,
///   ["dv01", "annuity", "par_rate"]
/// );
///
/// console.log(`Bond PV: ${bondResult.presentValue.format()}`);
/// console.log(`Swap PV: ${swapResult.presentValue.format()}`);
/// console.log(`Swap DV01: ${swapResult.metric("dv01")}`);
/// ```
#[wasm_bindgen(js_name = createStandardRegistry)]
pub fn create_standard_registry_js() -> JsPricerRegistry {
    JsPricerRegistry::new(create_standard_registry())
}
