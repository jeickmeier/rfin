use crate::core::error::js_error;
use crate::core::market_data::dividends::JsDividendSchedule;
use crate::core::market_data::fx::JsFxMatrix;
use crate::core::market_data::scalars::{JsMarketScalar, JsScalarTimeSeries};
use crate::core::market_data::surfaces::JsVolSurface;
use crate::core::market_data::term_structures::{
    JsBaseCorrelationCurve, JsCreditIndexData, JsDiscountCurve, JsForwardCurve, JsHazardCurve,
    JsInflationCurve,
};
use crate::core::utils::js_array_from_iter;
use finstack_core::market_data::context::{ContextStats, MarketContext};
use finstack_core::types::CurveId;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Discriminant for curve type in dynamic dispatch.
///
/// Enables generic curve retrieval without knowing the exact curve type at compile time.
///
/// # Example
/// ```javascript
/// const ctx = new MarketContext();
/// const curve = ctx.getCurve("USD-SOFR", CurveKind.Discount);
/// ```
#[wasm_bindgen(js_name = CurveKind)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsCurveKind {
    /// Discount curve for present value calculations
    Discount = 0,
    /// Forward rate curve for projection
    Forward = 1,
    /// Hazard rate curve for credit risk
    Hazard = 2,
    /// Inflation curve for CPI/RPI indexation
    Inflation = 3,
    /// Base correlation curve for structured credit
    BaseCorrelation = 4,
}

fn stats_to_object(stats: ContextStats) -> js_sys::Object {
    use js_sys::{Object, Reflect};

    let obj = Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("totalCurves"),
        &JsValue::from_f64(stats.total_curves as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("surfaceCount"),
        &JsValue::from_f64(stats.surface_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("priceCount"),
        &JsValue::from_f64(stats.price_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("seriesCount"),
        &JsValue::from_f64(stats.series_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("inflationIndexCount"),
        &JsValue::from_f64(stats.inflation_index_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("creditIndexCount"),
        &JsValue::from_f64(stats.credit_index_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("dividendScheduleCount"),
        &JsValue::from_f64(stats.dividend_schedule_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("collateralMappingCount"),
        &JsValue::from_f64(stats.collateral_mapping_count as f64),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("hasFx"),
        &JsValue::from_bool(stats.has_fx),
    );

    let counts = Object::new();
    for (kind, count) in stats.curve_counts {
        let _ = Reflect::set(
            &counts,
            &JsValue::from_str(kind),
            &JsValue::from_f64(count as f64),
        );
    }
    let _ = Reflect::set(&obj, &JsValue::from_str("curveCounts"), &counts.into());
    obj
}

#[wasm_bindgen(js_name = MarketContext)]
#[derive(Clone)]
pub struct JsMarketContext {
    inner: MarketContext,
}

impl JsMarketContext {
    pub(crate) fn inner(&self) -> &MarketContext {
        &self.inner
    }

    pub(crate) fn from_owned(inner: MarketContext) -> Self {
        Self { inner }
    }
}

impl Default for JsMarketContext {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = MarketContext)]
impl JsMarketContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsMarketContext {
        JsMarketContext {
            inner: MarketContext::new(),
        }
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_ctx(&self) -> JsMarketContext {
        JsMarketContext {
            inner: self.inner.clone(),
        }
    }

    #[wasm_bindgen(js_name = insertDiscount)]
    pub fn insert_discount(&mut self, curve: &JsDiscountCurve) {
        self.inner.insert_discount_mut(curve.inner());
    }

    #[wasm_bindgen(js_name = insertForward)]
    pub fn insert_forward(&mut self, curve: &JsForwardCurve) {
        self.inner.insert_forward_mut(curve.inner());
    }

    #[wasm_bindgen(js_name = insertHazard)]
    pub fn insert_hazard(&mut self, curve: &JsHazardCurve) {
        self.inner.insert_hazard_mut(curve.inner());
    }

    #[wasm_bindgen(js_name = insertInflation)]
    pub fn insert_inflation(&mut self, curve: &JsInflationCurve) {
        self.inner.insert_inflation_mut(curve.inner());
    }

    #[wasm_bindgen(js_name = insertBaseCorrelation)]
    pub fn insert_base_correlation(&mut self, curve: &JsBaseCorrelationCurve) {
        self.inner.insert_base_correlation_mut(curve.inner());
    }

    #[wasm_bindgen(js_name = insertSurface)]
    pub fn insert_surface(&mut self, surface: &JsVolSurface) {
        self.inner.insert_surface_mut(surface.inner());
    }

    #[wasm_bindgen(js_name = insertPrice)]
    pub fn insert_price(&mut self, id: &str, scalar: &JsMarketScalar) {
        self.inner.insert_price_mut(id, scalar.inner());
    }

    #[wasm_bindgen(js_name = insertSeries)]
    pub fn insert_series(&mut self, series: &JsScalarTimeSeries) {
        self.inner.insert_series_mut(series.inner());
    }

    #[wasm_bindgen(js_name = insertDividends)]
    pub fn insert_dividends(&mut self, schedule: &JsDividendSchedule) {
        self.inner.insert_dividends_arc_mut(schedule.inner());
    }

    #[wasm_bindgen(js_name = insertCreditIndex)]
    pub fn insert_credit_index(&mut self, id: &str, data: &JsCreditIndexData) {
        self.inner
            .insert_credit_index_mut(id, data.inner().as_ref().clone());
    }

    #[wasm_bindgen(js_name = insertFx)]
    pub fn insert_fx(&mut self, matrix: &JsFxMatrix) {
        self.inner.insert_fx_mut(matrix.inner());
    }

    #[wasm_bindgen(js_name = mapCollateral)]
    pub fn map_collateral(&mut self, csa_code: &str, curve_id: &str) {
        self.inner
            .map_collateral_mut(csa_code, CurveId::from(curve_id));
    }

    /// Generic curve retrieval with dynamic dispatch.
    ///
    /// Retrieves a curve of any type by ID and kind discriminant. Returns a `JsValue`
    /// that can be cast to the appropriate curve type.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    /// * `kind` - The curve type discriminant
    ///
    /// # Returns
    /// A `JsValue` containing the curve, which can be downcast to the specific type
    /// (e.g., `JsDiscountCurve`, `JsForwardCurve`, etc.)
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.getCurve("USD-SOFR", CurveKind.Discount);
    /// // curve is a JsDiscountCurve
    /// ```
    #[wasm_bindgen(js_name = getCurve)]
    pub fn get_curve(&self, id: &str, kind: JsCurveKind) -> Result<JsValue, JsValue> {
        match kind {
            JsCurveKind::Discount => {
                let arc = self
                    .inner
                    .get_discount(id)
                    .map_err(|e| js_error(e.to_string()))?;
                Ok(JsDiscountCurve::from_arc(arc).into())
            }
            JsCurveKind::Forward => {
                let arc = self
                    .inner
                    .get_forward(id)
                    .map_err(|e| js_error(e.to_string()))?;
                Ok(JsForwardCurve::from_arc(arc).into())
            }
            JsCurveKind::Hazard => {
                let arc = self
                    .inner
                    .get_hazard(id)
                    .map_err(|e| js_error(e.to_string()))?;
                Ok(JsHazardCurve::from_arc(arc).into())
            }
            JsCurveKind::Inflation => {
                let arc = self
                    .inner
                    .get_inflation(id)
                    .map_err(|e| js_error(e.to_string()))?;
                Ok(JsInflationCurve::from_arc(arc).into())
            }
            JsCurveKind::BaseCorrelation => {
                let arc = self
                    .inner
                    .get_base_correlation(id)
                    .map_err(|e| js_error(e.to_string()))?;
                Ok(JsBaseCorrelationCurve::from_arc(arc).into())
            }
        }
    }

    /// Retrieves a discount curve by ID.
    ///
    /// Type-safe convenience method for discount curve retrieval.
    /// Internally calls `get_curve` with `CurveKind::Discount`.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    ///
    /// # Returns
    /// A `JsDiscountCurve` instance
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.discount("USD-SOFR");
    /// const df = curve.df("2025-12-31");
    /// ```
    #[wasm_bindgen(js_name = discount)]
    pub fn discount(&self, id: &str) -> Result<JsDiscountCurve, JsValue> {
        let arc = self
            .inner
            .get_discount(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsDiscountCurve::from_arc(arc))
    }

    /// Retrieves a forward rate curve by ID.
    ///
    /// Type-safe convenience method for forward curve retrieval.
    /// Internally calls `get_curve` with `CurveKind::Forward`.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    ///
    /// # Returns
    /// A `JsForwardCurve` instance
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.forward("USD-SOFR");
    /// const rate = curve.forwardRate("2025-01-01", "2025-07-01");
    /// ```
    #[wasm_bindgen(js_name = forward)]
    pub fn forward(&self, id: &str) -> Result<JsForwardCurve, JsValue> {
        let arc = self
            .inner
            .get_forward(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsForwardCurve::from_arc(arc))
    }

    /// Retrieves a hazard rate curve by ID.
    ///
    /// Type-safe convenience method for hazard curve retrieval.
    /// Internally calls `get_curve` with `CurveKind::Hazard`.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    ///
    /// # Returns
    /// A `JsHazardCurve` instance
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.hazard("AAPL-5Y");
    /// const sp = curve.survivalProbability("2025-12-31");
    /// ```
    #[wasm_bindgen(js_name = hazard)]
    pub fn hazard(&self, id: &str) -> Result<JsHazardCurve, JsValue> {
        let arc = self
            .inner
            .get_hazard(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsHazardCurve::from_arc(arc))
    }

    /// Retrieves an inflation curve by ID.
    ///
    /// Type-safe convenience method for inflation curve retrieval.
    /// Internally calls `get_curve` with `CurveKind::Inflation`.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    ///
    /// # Returns
    /// A `JsInflationCurve` instance
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.inflation("US-CPI");
    /// const index = curve.indexValue("2025-12-31");
    /// ```
    #[wasm_bindgen(js_name = inflation)]
    pub fn inflation(&self, id: &str) -> Result<JsInflationCurve, JsValue> {
        let arc = self
            .inner
            .get_inflation(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsInflationCurve::from_arc(arc))
    }

    /// Retrieves a base correlation curve by ID.
    ///
    /// Type-safe convenience method for base correlation curve retrieval.
    /// Internally calls `get_curve` with `CurveKind::BaseCorrelation`.
    ///
    /// # Arguments
    /// * `id` - The curve identifier
    ///
    /// # Returns
    /// A `JsBaseCorrelationCurve` instance
    ///
    /// # Example
    /// ```javascript
    /// const ctx = new MarketContext();
    /// const curve = ctx.baseCorrelation("CDX-IG");
    /// const corr = curve.correlation("2025-12-31", 0.05);
    /// ```
    #[wasm_bindgen(js_name = baseCorrelation)]
    pub fn base_correlation(&self, id: &str) -> Result<JsBaseCorrelationCurve, JsValue> {
        let arc = self
            .inner
            .get_base_correlation(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsBaseCorrelationCurve::from_arc(arc))
    }

    #[wasm_bindgen(js_name = surface)]
    pub fn surface(&self, id: &str) -> Result<JsVolSurface, JsValue> {
        let arc = self
            .inner
            .surface(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsVolSurface::from_arc(arc))
    }

    #[wasm_bindgen(js_name = price)]
    pub fn price(&self, id: &str) -> Result<JsMarketScalar, JsValue> {
        let scalar = self.inner.price(id).map_err(|e| js_error(e.to_string()))?;
        Ok(JsMarketScalar::from_inner(scalar.clone()))
    }

    #[wasm_bindgen(js_name = series)]
    pub fn series(&self, id: &str) -> Result<JsScalarTimeSeries, JsValue> {
        let series = self.inner.series(id).map_err(|e| js_error(e.to_string()))?;
        Ok(JsScalarTimeSeries::from_arc(Arc::new(series.clone())))
    }

    #[wasm_bindgen(js_name = creditIndex)]
    pub fn credit_index(&self, id: &str) -> Result<JsCreditIndexData, JsValue> {
        let data = self
            .inner
            .credit_index(id)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsCreditIndexData::from_arc(data))
    }

    #[wasm_bindgen(js_name = dividendSchedule)]
    pub fn dividend_schedule(&self, id: &str) -> Option<JsDividendSchedule> {
        self.inner
            .dividend_schedule(id)
            .map(|schedule| JsDividendSchedule::from_arc(schedule.clone()))
    }

    #[wasm_bindgen(js_name = curveIds)]
    pub fn curve_ids(&self) -> js_sys::Array {
        let ids = self
            .inner
            .curve_ids()
            .map(|id| JsValue::from_str(id.as_ref()));
        js_array_from_iter(ids)
    }

    #[wasm_bindgen(js_name = curveIdsByType)]
    pub fn curve_ids_by_type(&self, curve_type: &str) -> js_sys::Array {
        let ids = self
            .inner
            .curves_of_type(curve_type)
            .map(|(id, _)| JsValue::from_str(id.as_ref()));
        js_array_from_iter(ids)
    }

    #[wasm_bindgen(js_name = countByType)]
    pub fn count_by_type(&self) -> js_sys::Object {
        use js_sys::{Object, Reflect};
        let obj = Object::new();
        for (kind, count) in self.inner.count_by_type() {
            let _ = Reflect::set(
                &obj,
                &JsValue::from_str(kind),
                &JsValue::from_f64(count as f64),
            );
        }
        obj
    }

    #[wasm_bindgen(js_name = stats)]
    pub fn stats(&self) -> js_sys::Object {
        stats_to_object(self.inner.stats())
    }

    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = totalObjects)]
    pub fn total_objects(&self) -> usize {
        self.inner.total_objects()
    }

    #[wasm_bindgen(js_name = hasFx)]
    pub fn has_fx(&self) -> bool {
        self.inner.fx.is_some()
    }
}
