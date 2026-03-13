use crate::core::dates::date::JsDate;
use crate::core::error::core_to_js;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::valuations::instruments::extract_instrument;
use crate::valuations::results::JsValuationResult;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::{
    create_standard_registry, register_credit_pricers, register_equity_pricers,
    register_fx_pricers, register_rates_pricers, ModelKey, PricerRegistry,
};
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
    as_of: &JsDate,
) -> Result<JsValuationResult, JsValue> {
    let as_of_date = as_of.inner();

    // If metrics requested, use the canonical price_with_metrics path
    if let Some(metric_ids) = metrics {
        if !metric_ids.is_empty() {
            return instrument
                .price_with_metrics(market.inner(), as_of_date, &metric_ids)
                .map(JsValuationResult::new)
                .map_err(core_error_to_js);
        }
    }

    // Otherwise just get base price via registry
    registry
        .price_with_registry(instrument, model_key, market.inner(), as_of_date, None)
        .map(JsValuationResult::new)
        .map_err(pricing_error_to_js)
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
/// const result = registry.priceInstrument(bond, "discounting", market, asOf, opts);
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
    /// const result = registry.priceInstrument(bond, "discounting", market, asOf, opts);
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
    /// const result = registry.priceInstrument(bond, "discounting", market, asOf, opts);
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

    /// Price any supported instrument using dynamic dispatch.
    ///
    /// This is the primary API for pricing all instruments in finstack-wasm. The instrument
    /// must be a valid finstack-wasm wrapper type (e.g., `Bond`, `InterestRateSwap`, `EquityOption`).
    ///
    /// ## Supported Instruments
    ///
    /// **Fixed Income**: Bond, Deposit, Repo, InflationLinkedBond, ConvertibleBond, TermLoan
    ///
    /// **Interest Rate Derivatives**: InterestRateSwap, BasisSwap, ForwardRateAgreement,
    /// Swaption, InterestRateOption (caps/floors), InterestRateFuture, RangeAccrual, CmsOption,
    /// XccySwap, InflationSwap, YoYInflationSwap, InflationCapFloor
    ///
    /// **FX**: FxSpot, FxOption, FxSwap, FxForward, FxBarrierOption, FxVarianceSwap, Ndf
    ///
    /// **Equity**: Equity, EquityOption, BarrierOption, AsianOption, LookbackOption,
    /// CliquetOption, QuantoOption, VarianceSwap, Autocallable, EquityIndexFuture,
    /// EquityTotalReturnSwap
    ///
    /// **Credit**: CreditDefaultSwap, CDSIndex, CDSTranche, CDSOption
    ///
    /// **Structured Products**: StructuredCredit, Basket, PrivateMarketsFund, RevolvingCredit
    ///
    /// **Agency MBS**: AgencyMbsPassthrough, AgencyTba, DollarRoll, AgencyCmo
    ///
    /// **Commodities**: CommodityForward, CommoditySwap, CommodityOption
    ///
    /// **Real Assets**: RealEstateAsset
    ///
    /// ## Pricing Models
    ///
    /// Common model keys:
    /// - `"discounting"` - Standard discounted cash flow (bonds, swaps, deposits)
    /// - `"black76"` - Black-76 model (swaptions, caps/floors, FX options)
    /// - `"black_scholes"` - Black-Scholes model (equity options)
    /// - `"monte_carlo_gbm"` - Monte Carlo with GBM (exotic options, autocallables)
    /// - `"hazard_rate"` - Hazard rate model (CDS, credit derivatives)
    /// - `"tree"` - Binomial/trinomial trees (convertibles, American options)
    ///
    /// @param {any} instrument - A finstack-wasm instrument instance
    /// @param {string} model - Pricing model key
    /// @param {MarketContext} market - Market data context with curves, surfaces, and scalars
    /// @param {FsDate} asOf - Valuation date
    /// @param {PricingRequest?} opts - Optional pricing configuration for risk metrics
    /// @returns {ValuationResult} Pricing result with present value and optional metrics
    /// @throws {Error} If the instrument type is unsupported, model is invalid, or required market data is missing
    ///
    /// @example
    /// ```javascript
    /// import { createStandardRegistry, Bond, InterestRateSwap, EquityOption, MarketContext, FsDate, Money, PricingRequest } from 'finstack-wasm';
    ///
    /// const registry = createStandardRegistry();
    /// const market = new MarketContext();
    /// market.insertDiscount(discountCurve);
    /// const asOf = new FsDate(2024, 1, 2);
    ///
    /// // Price a bond
    /// const bond = Bond.fixedSemiannual("bond1", Money.of(1000000, "USD"), 0.05, issue, maturity, "USD-OIS");
    /// const bondResult = registry.priceInstrument(bond, "discounting", market, asOf);
    /// console.log(`Bond PV: ${bondResult.presentValue.format()}`);
    ///
    /// // Price a swap with metrics
    /// const swap = new InterestRateSwap(...);
    /// const opts = new PricingRequest().withMetrics(["dv01", "annuity", "par_rate"]);
    /// const swapResult = registry.priceInstrument(swap, "discounting", market, asOf, opts);
    /// console.log(`Swap DV01: ${swapResult.metric("dv01")}`);
    ///
    /// // Price an equity option
    /// const option = new EquityOption(...);
    /// const optResult = registry.priceInstrument(option, "black_scholes", market, asOf);
    /// console.log(`Option PV: ${optResult.presentValue.format()}`);
    /// ```
    #[wasm_bindgen(js_name = priceInstrument)]
    pub fn price_instrument(
        &self,
        instrument: &JsValue,
        model: &str,
        market: &JsMarketContext,
        as_of: &JsDate,
        opts: Option<JsPricingRequest>,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = extract_instrument(instrument)?;
        let metrics = opts.and_then(|o| o.metrics);
        price_with_optional_metrics(
            &self.inner,
            instrument.as_ref(),
            model_key,
            market,
            metrics,
            as_of,
        )
    }
}

/// Create a pricing registry populated with all standard finstack pricers.
///
/// This is the main entry point for instrument valuation. The standard registry
/// includes pricing engines for all supported instrument types (bonds, swaps,
/// options, credit derivatives, etc.) using common models like discounting,
/// Black-76, and hazard rate approaches.
///
/// ⚠️ Note: In WebAssembly builds, creating the full standard registry may be
/// memory intensive depending on the enabled feature set. If you hit memory
/// issues, prefer the smaller registries like `createRatesRegistry()`,
/// `createCreditRegistry()`, `createEquityRegistry()`, or `createFxRegistry()`.
///
/// @returns {PricerRegistry} Registry with all built-in pricing engines loaded
///
/// @example
/// ```javascript
/// import { createStandardRegistry, Bond, InterestRateSwap, MarketContext, FsDate, Money, PricingRequest } from 'finstack-wasm';
///
/// const registry = createStandardRegistry();
///
/// // Create market data
/// const asOf = new FsDate(2024, 1, 2);
/// const curve = new DiscountCurve("USD-OIS", asOf, ...);
/// const market = new MarketContext();
/// market.insertDiscount(curve);
///
/// // Price a bond
/// const bond = Bond.fixedSemiannual("bond1", Money.of(1000000, "USD"), 0.05, issue, maturity, "USD-OIS");
/// const bondResult = registry.priceInstrument(bond, "discounting", market, asOf);
///
/// // Price a swap with metrics
/// const swap = new InterestRateSwap(...);
/// const opts = new PricingRequest().withMetrics(["dv01", "annuity", "par_rate"]);
/// const swapResult = registry.priceInstrument(swap, "discounting", market, asOf, opts);
///
/// console.log(`Bond PV: ${bondResult.presentValue.format()}`);
/// console.log(`Swap PV: ${swapResult.presentValue.format()}`);
/// console.log(`Swap DV01: ${swapResult.metric("dv01")}`);
/// ```
#[wasm_bindgen(js_name = createStandardRegistry)]
pub fn create_standard_registry_js() -> JsPricerRegistry {
    JsPricerRegistry::new(create_standard_registry())
}

/// Create a pricing registry populated with *rates* pricers.
///
/// This is intended for memory-constrained environments (like WASM) where
/// `createStandardRegistry()` may be too large.
#[wasm_bindgen(js_name = createRatesRegistry)]
pub fn create_rates_registry_js() -> JsPricerRegistry {
    let mut registry = PricerRegistry::new();
    register_rates_pricers(&mut registry);
    JsPricerRegistry::new(registry)
}

/// Create a pricing registry populated with *credit* pricers.
#[wasm_bindgen(js_name = createCreditRegistry)]
pub fn create_credit_registry_js() -> JsPricerRegistry {
    let mut registry = PricerRegistry::new();
    register_credit_pricers(&mut registry);
    JsPricerRegistry::new(registry)
}

/// Create a pricing registry populated with *equity* pricers.
#[wasm_bindgen(js_name = createEquityRegistry)]
pub fn create_equity_registry_js() -> JsPricerRegistry {
    let mut registry = PricerRegistry::new();
    register_equity_pricers(&mut registry);
    JsPricerRegistry::new(registry)
}

/// Create a pricing registry populated with *FX* pricers.
#[wasm_bindgen(js_name = createFxRegistry)]
pub fn create_fx_registry_js() -> JsPricerRegistry {
    let mut registry = PricerRegistry::new();
    register_fx_pricers(&mut registry);
    JsPricerRegistry::new(registry)
}
