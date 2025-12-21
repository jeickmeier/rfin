use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::utils::js_array_from_iter;
use finstack_core::currency::Currency;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::{FxConfig, FxConversionPolicy, FxMatrix, FxQuery, FxRateResult};
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = FxConversionPolicy)]
#[derive(Clone, Copy, Debug)]
pub struct JsFxConversionPolicy {
    inner: FxConversionPolicy,
}

impl JsFxConversionPolicy {
    pub(crate) fn inner(&self) -> FxConversionPolicy {
        self.inner
    }

    fn new(inner: FxConversionPolicy) -> Self {
        Self { inner }
    }
}

impl From<JsFxConversionPolicy> for FxConversionPolicy {
    fn from(value: JsFxConversionPolicy) -> Self {
        value.inner
    }
}

impl From<FxConversionPolicy> for JsFxConversionPolicy {
    fn from(value: FxConversionPolicy) -> Self {
        Self::new(value)
    }
}

#[wasm_bindgen(js_class = FxConversionPolicy)]
impl JsFxConversionPolicy {
    #[wasm_bindgen(js_name = CashflowDate)]
    pub fn cashflow_date() -> JsFxConversionPolicy {
        Self::new(FxConversionPolicy::CashflowDate)
    }

    #[wasm_bindgen(js_name = PeriodEnd)]
    pub fn period_end() -> JsFxConversionPolicy {
        Self::new(FxConversionPolicy::PeriodEnd)
    }

    #[wasm_bindgen(js_name = PeriodAverage)]
    pub fn period_average() -> JsFxConversionPolicy {
        Self::new(FxConversionPolicy::PeriodAverage)
    }

    #[wasm_bindgen(js_name = Custom)]
    pub fn custom() -> JsFxConversionPolicy {
        Self::new(FxConversionPolicy::Custom)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsFxConversionPolicy, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "cashflow_date" | "cashflow" | "spot" => Ok(Self::cashflow_date()),
            "period_end" | "end" => Ok(Self::period_end()),
            "period_average" | "average" => Ok(Self::period_average()),
            "custom" => Ok(Self::custom()),
            other => Err(js_error(format!("Unknown FX conversion policy: {other}"))),
        }
    }

    #[wasm_bindgen(js_name = name)]
    pub fn name(&self) -> String {
        match self.inner {
            FxConversionPolicy::CashflowDate => "cashflow_date".to_string(),
            FxConversionPolicy::PeriodEnd => "period_end".to_string(),
            FxConversionPolicy::PeriodAverage => "period_average".to_string(),
            FxConversionPolicy::Custom => "custom".to_string(),
            _ => "custom".to_string(),
        }
    }
}

#[wasm_bindgen(js_name = FxConfig)]
#[derive(Clone, Debug)]
pub struct JsFxConfig {
    inner: FxConfig,
}

impl Default for JsFxConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FxConfig)]
impl JsFxConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFxConfig {
        JsFxConfig {
            inner: FxConfig::default(),
        }
    }

    #[wasm_bindgen(getter, js_name = pivotCurrency)]
    pub fn pivot_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.pivot_currency)
    }

    #[wasm_bindgen(js_name = setPivotCurrency)]
    pub fn set_pivot_currency(&mut self, currency: &JsCurrency) {
        self.inner.pivot_currency = currency.inner();
    }

    #[wasm_bindgen(getter, js_name = enableTriangulation)]
    pub fn enable_triangulation(&self) -> bool {
        self.inner.enable_triangulation
    }

    #[wasm_bindgen(js_name = setEnableTriangulation)]
    pub fn set_enable_triangulation(&mut self, flag: bool) {
        self.inner.enable_triangulation = flag;
    }

    #[wasm_bindgen(getter, js_name = cacheCapacity)]
    pub fn cache_capacity(&self) -> usize {
        self.inner.cache_capacity
    }

    #[wasm_bindgen(js_name = setCacheCapacity)]
    pub fn set_cache_capacity(&mut self, capacity: usize) {
        self.inner.cache_capacity = capacity;
    }
}

#[wasm_bindgen(js_name = FxRateResult)]
#[derive(Clone, Debug)]
pub struct JsFxRateResult {
    pub rate: f64,
    pub triangulated: bool,
}

impl From<FxRateResult> for JsFxRateResult {
    fn from(value: FxRateResult) -> Self {
        JsFxRateResult {
            rate: value.rate,
            triangulated: value.triangulated,
        }
    }
}

// SimpleFxProvider is now provided by finstack-core

#[wasm_bindgen(js_name = FxMatrix)]
#[derive(Clone)]
pub struct JsFxMatrix {
    provider: Arc<SimpleFxProvider>,
    inner: Arc<FxMatrix>,
}

impl JsFxMatrix {
    fn new_with(provider: Arc<SimpleFxProvider>, config: FxConfig) -> Self {
        let matrix = FxMatrix::with_config(provider.clone(), config);
        Self {
            provider,
            inner: Arc::new(matrix),
        }
    }

    pub(crate) fn inner(&self) -> Arc<FxMatrix> {
        Arc::clone(&self.inner)
    }
}

impl Default for JsFxMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FxMatrix)]
impl JsFxMatrix {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFxMatrix {
        let provider = Arc::new(SimpleFxProvider::new());
        JsFxMatrix::new_with(provider, FxConfig::default())
    }

    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(config: &JsFxConfig) -> JsFxMatrix {
        let provider = Arc::new(SimpleFxProvider::new());
        JsFxMatrix::new_with(provider, config.inner)
    }

    #[wasm_bindgen(js_name = setQuote)]
    pub fn set_quote(&self, from: &JsCurrency, to: &JsCurrency, rate: f64) {
        let from_ccy = from.inner();
        let to_ccy = to.inner();
        self.provider.set_quote(from_ccy, to_ccy, rate);
        self.inner.set_quote(from_ccy, to_ccy, rate);
    }

    #[wasm_bindgen(js_name = setQuotes)]
    pub fn set_quotes(&self, quotes: js_sys::Array) -> Result<(), JsValue> {
        let mut converted: Vec<(Currency, Currency, f64)> =
            Vec::with_capacity(quotes.length() as usize);
        for entry in quotes.iter() {
            if !entry.is_object() {
                return Err(js_error(
                    "Each quote must be provided as [baseCurrencyCode, quoteCurrencyCode, rate]",
                ));
            }
            let tuple = js_sys::Array::from(&entry);
            if tuple.length() != 3 {
                return Err(js_error(
                    "Each quote must have three elements: [baseCurrencyCode, quoteCurrencyCode, rate]",
                ));
            }
            let base_code = tuple
                .get(0)
                .as_string()
                .ok_or_else(|| js_error("Currency codes must be strings"))?;
            let quote_code = tuple
                .get(1)
                .as_string()
                .ok_or_else(|| js_error("Currency codes must be strings"))?;
            let rate = tuple
                .get(2)
                .as_f64()
                .ok_or_else(|| js_error("Rate must be a number"))?;

            let base_ccy = Currency::from_str(&base_code)
                .map_err(|_| js_error(format!("Invalid base currency: {base_code}")))?;
            let quote_ccy = Currency::from_str(&quote_code)
                .map_err(|_| js_error(format!("Invalid quote currency: {quote_code}")))?;

            converted.push((base_ccy, quote_ccy, rate));
        }
        self.provider.set_quotes(&converted);
        for (from, to, rate) in converted {
            self.inner.set_quote(from, to, rate);
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = rate)]
    pub fn rate(
        &self,
        from: &JsCurrency,
        to: &JsCurrency,
        on: &JsDate,
        policy: &JsFxConversionPolicy,
    ) -> Result<JsFxRateResult, JsValue> {
        let query = FxQuery::with_policy(from.inner(), to.inner(), on.inner(), policy.inner());
        let result = self
            .inner
            .rate(query)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsFxRateResult::from(result))
    }

    #[wasm_bindgen(js_name = cacheStats)]
    pub fn cache_stats(&self) -> usize {
        self.inner.cache_stats()
    }

    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    #[wasm_bindgen(js_name = clearExpired)]
    pub fn clear_expired(&self) {
        self.inner.clear_expired();
    }
}
