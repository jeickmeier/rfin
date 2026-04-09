use crate::core::dates::date::JsDate;
use crate::core::error::core_to_js;
use crate::core::error::js_error;
use crate::core::market_data::interp::{parse_extrapolation, parse_interp_style};
use crate::core::utils::js_array_from_iter;
use finstack_core::currency::Currency as CoreCurrency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::term_structures::{
    BaseCorrelationCurve, CreditIndexData, DiscountCurve, FlatCurve, ForwardCurve, HazardCurve,
    InflationCurve, PriceCurve, Seniority,
};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

fn parse_day_count_jsvalue(value: &JsValue) -> Result<Option<DayCount>, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    if let Some(name) = value.as_string() {
        DayCount::from_str(&name)
            .map(Some)
            .map_err(|e| js_error(e.to_string()))
    } else {
        Err(js_error(
            "dayCount must be provided as a string identifier (e.g. 'act_365f')",
        ))
    }
}

fn parse_interp_value(value: &JsValue) -> Result<InterpStyle, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(InterpStyle::Linear);
    }
    if let Some(name) = value.as_string() {
        return parse_interp_style(Some(&name), InterpStyle::Linear);
    }
    if let Some(code) = value.as_f64() {
        return match code as u32 {
            0 => Ok(InterpStyle::Linear),
            1 => Ok(InterpStyle::LogLinear),
            2 => Ok(InterpStyle::MonotoneConvex),
            3 => Ok(InterpStyle::CubicHermite),
            4 => Ok(InterpStyle::LogLinear),
            other => Err(js_error(format!(
                "Unknown interpolation style enum discriminant: {other}"
            ))),
        };
    }
    Err(js_error(
        "interp must be provided as an InterpStyle enum value or style name",
    ))
}

fn parse_extrap_value(value: &JsValue) -> Result<ExtrapolationPolicy, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(ExtrapolationPolicy::FlatZero);
    }
    if let Some(name) = value.as_string() {
        return parse_extrapolation(Some(&name));
    }
    if let Some(code) = value.as_f64() {
        return match code as u32 {
            0 => Ok(ExtrapolationPolicy::FlatZero),
            1 => Ok(ExtrapolationPolicy::FlatForward),
            other => Err(js_error(format!(
                "Unknown extrapolation policy discriminant: {other}"
            ))),
        };
    }
    Err(js_error(
        "extrapolation must be provided as an ExtrapolationPolicy enum value or policy name",
    ))
}

#[wasm_bindgen(js_name = DiscountCurve)]
#[derive(Clone)]
pub struct JsDiscountCurve {
    inner: Arc<DiscountCurve>,
}

impl JsDiscountCurve {
    pub(crate) fn from_arc(inner: Arc<DiscountCurve>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<DiscountCurve> {
        Arc::clone(&self.inner)
    }

    fn base_date(&self) -> Date {
        self.inner.base_date()
    }
}

#[wasm_bindgen(js_class = DiscountCurve)]
impl JsDiscountCurve {
    /// Create a discount curve with (time, discount_factor) knot points.
    ///
    /// @param {string} id - Curve identifier used to retrieve it later from MarketContext
    /// @param {Date} base_date - Anchor date corresponding to t = 0
    /// @param {Array<number> | Float64Array} times - Time knots in years from base_date
    /// @param {Array<number> | Float64Array} discount_factors - Discount factor values at each time point
    /// @param {string} day_count - Day count convention (e.g., "act_365f", "30_360")
    /// @param {string} interp - Interpolation style ("linear", "monotone_convex", "log_linear", etc.)
    /// @param {string} extrapolation - Extrapolation policy ("flat_zero", "flat_forward")
    /// @param {boolean} require_monotonic - Enforce monotonically decreasing discount factors (set false to allow non-monotonic)
    /// @returns {DiscountCurve} Curve object exposing discount factor and zero rate helpers
    /// @throws {Error} If knots are invalid, times/factors length mismatch, or fewer than 2 points
    ///
    /// @example
    /// ```javascript
    /// const baseDate = new Date(2024, 1, 2);
    /// const curve = new DiscountCurve(
    ///   "USD-OIS",
    ///   baseDate,
    ///   [0.0, 0.5, 1.0, 2.0, 5.0],                    // times in years
    ///   [1.0, 0.9975, 0.9950, 0.9850, 0.9650],        // discount factors
    ///   "act_365f",                                    // day count
    ///   "monotone_convex",                             // interpolation
    ///   "flat_forward",                                // extrapolation
    ///   true                                           // require monotonic
    /// );
    ///
    /// console.log(curve.df(1.0));       // 0.9950 (discount factor at 1 year)
    /// console.log(curve.zero(1.0));     // ~0.005012 (zero rate at 1 year)
    /// console.log(curve.forward(0.5, 1.0));  // forward rate 0.5y → 1y
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        base_date: &JsDate,
        times: Vec<f64>,
        discount_factors: Vec<f64>,
        day_count: JsValue,
        interp: JsValue,
        extrapolation: JsValue,
        require_monotonic: bool,
    ) -> Result<JsDiscountCurve, JsValue> {
        if times.len() != discount_factors.len() {
            return Err(js_error(
                "times and discountFactors must be the same length",
            ));
        }
        if times.len() < 2 {
            return Err(js_error(
                "at least two knots are required to build a discount curve",
            ));
        }

        let points: Vec<(f64, f64)> = times.into_iter().zip(discount_factors).collect();
        let style = if interp.is_undefined() || interp.is_null() {
            InterpStyle::LogLinear
        } else {
            parse_interp_value(&interp)?
        };
        let extrap = if extrapolation.is_undefined() || extrapolation.is_null() {
            ExtrapolationPolicy::FlatForward
        } else {
            parse_extrap_value(&extrapolation)?
        };
        let picked_day_count = parse_day_count_jsvalue(&day_count)?.unwrap_or(DayCount::Act365F);

        let mut builder = DiscountCurve::builder(id)
            .base_date(base_date.inner())
            .knots(points)
            .interp(style)
            .extrapolation(extrap)
            .day_count(picked_day_count);

        if !require_monotonic {
            builder = builder.allow_non_monotonic();
        }

        let curve = builder.build().map_err(core_to_js)?;
        Ok(JsDiscountCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter, js_name = id)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date_js(&self) -> JsDate {
        JsDate::from_core(self.base_date())
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count_name(&self) -> String {
        format!("{:?}", self.inner.day_count())
    }

    pub fn df(&self, time: f64) -> f64 {
        self.inner.df(time)
    }

    pub fn zero(&self, time: f64) -> f64 {
        self.inner.zero(time)
    }

    pub fn forward(&self, t1: f64, t2: f64) -> Result<f64, JsValue> {
        self.inner
            .forward(t1, t2)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = dfOnDate)]
    pub fn df_on_date(&self, date: &JsDate, day_count: JsValue) -> Result<f64, JsValue> {
        let dc = parse_day_count_jsvalue(&day_count)?.unwrap_or(self.inner.day_count());
        let yf = dc
            .year_fraction(self.base_date(), date.inner(), DayCountCtx::default())
            .map_err(|e| js_error(e.to_string()))?;
        Ok(self.inner.df(yf))
    }
}

#[wasm_bindgen(js_name = ForwardCurve)]
#[derive(Clone)]
pub struct JsForwardCurve {
    inner: Arc<ForwardCurve>,
}

impl JsForwardCurve {
    pub(crate) fn from_arc(inner: Arc<ForwardCurve>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<ForwardCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = ForwardCurve)]
impl JsForwardCurve {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        base_date: &JsDate,
        tenor_years: f64,
        times: Vec<f64>,
        forwards: Vec<f64>,
        day_count: JsValue,
        reset_lag: Option<i32>,
        interp: JsValue,
    ) -> Result<JsForwardCurve, JsValue> {
        if times.len() != forwards.len() {
            return Err(js_error("times and forwards must have the same length"));
        }
        if times.is_empty() {
            return Err(js_error("at least one forward rate point is required"));
        }

        let style = parse_interp_value(&interp)?;
        let mut builder = ForwardCurve::builder(id, tenor_years)
            .base_date(base_date.inner())
            .knots(times.into_iter().zip(forwards))
            .interp(style);

        if let Some(dc) = parse_day_count_jsvalue(&day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(lag) = reset_lag {
            builder = builder.reset_lag(lag);
        }

        let curve = builder.build().map_err(core_to_js)?;
        Ok(JsForwardCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> JsDate {
        JsDate::from_core(self.inner.base_date())
    }

    #[wasm_bindgen(getter, js_name = resetLag)]
    pub fn reset_lag(&self) -> i32 {
        self.inner.reset_lag()
    }

    #[wasm_bindgen(getter)]
    pub fn tenor(&self) -> f64 {
        self.inner.tenor()
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count())
    }

    pub fn rate(&self, time: f64) -> f64 {
        self.inner.rate(time)
    }

    #[wasm_bindgen(js_name = ratePeriod)]
    pub fn rate_period(&self, t1: f64, t2: f64) -> f64 {
        self.inner.rate_period(t1, t2)
    }
}

#[wasm_bindgen(js_name = HazardCurve)]
#[derive(Clone)]
pub struct JsHazardCurve {
    inner: Arc<HazardCurve>,
}

impl JsHazardCurve {
    pub(crate) fn from_arc(inner: Arc<HazardCurve>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<HazardCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = HazardCurve)]
impl JsHazardCurve {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        base_date: &JsDate,
        times: Vec<f64>,
        hazard_rates: Vec<f64>,
        recovery_rate: Option<f64>,
        day_count: JsValue,
        issuer: Option<String>,
        seniority: Option<String>,
        currency: Option<String>,
        par_tenors: Option<Vec<f64>>,
        par_spreads_bp: Option<Vec<f64>>,
    ) -> Result<JsHazardCurve, JsValue> {
        if times.len() != hazard_rates.len() {
            return Err(js_error("times and hazardRates must have the same length"));
        }
        if times.is_empty() {
            return Err(js_error("at least one hazard rate point is required"));
        }

        let mut builder = HazardCurve::builder(id)
            .base_date(base_date.inner())
            .knots(times.into_iter().zip(hazard_rates));

        if let Some(r) = recovery_rate {
            builder = builder.recovery_rate(r);
        }
        if let Some(dc) = parse_day_count_jsvalue(&day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(name) = issuer {
            builder = builder.issuer(name);
        }
        if let Some(s) = seniority {
            let parsed = Seniority::from_str(&s).map_err(js_error)?;
            builder = builder.seniority(parsed);
        }
        if let Some(code) = currency {
            let ccy = CoreCurrency::from_str(&code)
                .map_err(|_| js_error(format!("Unknown currency code: {code}")))?;
            builder = builder.currency(ccy);
        }
        if let (Some(tenors), Some(spreads)) = (par_tenors, par_spreads_bp) {
            if tenors.len() != spreads.len() {
                return Err(js_error(
                    "parTenors and parSpreads must have the same length",
                ));
            }
            builder = builder.par_spreads(tenors.into_iter().zip(spreads));
        }

        let curve = builder.build().map_err(core_to_js)?;
        Ok(JsHazardCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> JsDate {
        JsDate::from_core(self.inner.base_date())
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate()
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count())
    }

    pub fn sp(&self, time: f64) -> f64 {
        self.inner.sp(time)
    }

    #[wasm_bindgen(js_name = defaultProb)]
    pub fn default_prob(&self, t1: f64, t2: f64) -> Result<f64, JsValue> {
        self.inner
            .default_prob(t1, t2)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen(js_name = InflationCurve)]
#[derive(Clone)]
pub struct JsInflationCurve {
    inner: Arc<InflationCurve>,
}

impl JsInflationCurve {
    pub(crate) fn from_arc(inner: Arc<InflationCurve>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<InflationCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = InflationCurve)]
impl JsInflationCurve {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        base_cpi: f64,
        times: Vec<f64>,
        cpi_levels: Vec<f64>,
        interp: JsValue,
    ) -> Result<JsInflationCurve, JsValue> {
        if times.len() != cpi_levels.len() {
            return Err(js_error("times and cpiLevels must have the same length"));
        }
        if times.is_empty() {
            return Err(js_error("at least one CPI knot is required"));
        }
        let style = parse_interp_value(&interp)?;
        let curve = InflationCurve::builder(id)
            .base_cpi(base_cpi)
            .knots(times.into_iter().zip(cpi_levels.into_iter()))
            .interp(style)
            .build()
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsInflationCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseCpi)]
    pub fn base_cpi(&self) -> f64 {
        self.inner.base_cpi()
    }

    pub fn cpi(&self, time: f64) -> f64 {
        self.inner.cpi(time)
    }

    #[wasm_bindgen(js_name = inflationRate)]
    pub fn inflation_rate(&self, t1: f64, t2: f64) -> f64 {
        self.inner.inflation_rate(t1, t2)
    }
}

#[wasm_bindgen(js_name = BaseCorrelationCurve)]
#[derive(Clone)]
pub struct JsBaseCorrelationCurve {
    inner: Arc<BaseCorrelationCurve>,
}

impl JsBaseCorrelationCurve {
    pub(crate) fn from_arc(inner: Arc<BaseCorrelationCurve>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<BaseCorrelationCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = BaseCorrelationCurve)]
impl JsBaseCorrelationCurve {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        detachment_points: Vec<f64>,
        correlations: Vec<f64>,
    ) -> Result<JsBaseCorrelationCurve, JsValue> {
        if detachment_points.len() != correlations.len() {
            return Err(js_error(
                "detachmentPoints and correlations must have the same length",
            ));
        }
        if detachment_points.len() < 2 {
            return Err(js_error(
                "at least two detachment points are required for a base correlation curve",
            ));
        }

        let curve = BaseCorrelationCurve::builder(id)
            .knots(detachment_points.into_iter().zip(correlations.into_iter()))
            .build()
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsBaseCorrelationCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    pub fn correlation(&self, detachment_pct: f64) -> f64 {
        self.inner.correlation(detachment_pct)
    }

    #[wasm_bindgen(js_name = points)]
    pub fn points(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for (d, c) in self
            .inner
            .detachment_points()
            .iter()
            .zip(self.inner.correlations().iter())
        {
            let tuple = js_sys::Array::new();
            tuple.push(&JsValue::from_f64(*d));
            tuple.push(&JsValue::from_f64(*c));
            arr.push(&tuple);
        }
        arr
    }
}

#[wasm_bindgen(js_name = CreditIndexData)]
#[derive(Clone)]
pub struct JsCreditIndexData {
    inner: Arc<CreditIndexData>,
}

impl JsCreditIndexData {
    pub(crate) fn from_arc(inner: Arc<CreditIndexData>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<CreditIndexData> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = CreditIndexData)]
impl JsCreditIndexData {
    #[wasm_bindgen(constructor)]
    pub fn new(
        num_constituents: u16,
        recovery_rate: f64,
        index_curve: &JsHazardCurve,
        base_correlation_curve: &JsBaseCorrelationCurve,
        issuer_ids: Option<Vec<String>>,
        issuer_curves: Option<Vec<JsHazardCurve>>,
    ) -> Result<JsCreditIndexData, JsValue> {
        let mut builder = CreditIndexData::builder()
            .num_constituents(num_constituents)
            .recovery_rate(recovery_rate)
            .index_credit_curve(index_curve.inner())
            .base_correlation_curve(base_correlation_curve.inner());

        match (issuer_ids, issuer_curves) {
            (Some(ids), Some(curves)) => {
                if ids.len() != curves.len() {
                    return Err(js_error(
                        "issuerIds and issuerCurves must have the same length",
                    ));
                }
                let mut map: HashMap<String, Arc<HazardCurve>> = HashMap::default();
                map.reserve(ids.len());
                for (id, curve) in ids.into_iter().zip(curves.into_iter()) {
                    map.insert(id, curve.inner());
                }
                builder = builder.issuer_curves(map);
            }
            (None, None) => {}
            _ => {
                return Err(js_error(
                    "issuerIds and issuerCurves must both be provided or both omitted",
                ));
            }
        }

        let data = builder.build().map_err(|e| js_error(e.to_string()))?;
        Ok(JsCreditIndexData {
            inner: Arc::new(data),
        })
    }

    #[wasm_bindgen(getter, js_name = numConstituents)]
    pub fn num_constituents(&self) -> u16 {
        self.inner.num_constituents
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    #[wasm_bindgen(getter, js_name = indexCurve)]
    pub fn index_curve(&self) -> JsHazardCurve {
        JsHazardCurve::from_arc(Arc::clone(&self.inner.index_credit_curve))
    }

    #[wasm_bindgen(getter, js_name = baseCorrelationCurve)]
    pub fn base_correlation_curve(&self) -> JsBaseCorrelationCurve {
        JsBaseCorrelationCurve::from_arc(Arc::clone(&self.inner.base_correlation_curve))
    }

    #[wasm_bindgen(js_name = hasIssuerCurves)]
    pub fn has_issuer_curves(&self) -> bool {
        self.inner.has_issuer_curves()
    }

    #[wasm_bindgen(js_name = issuerIds)]
    pub fn issuer_ids(&self) -> js_sys::Array {
        let ids = self.inner.issuer_ids();
        js_array_from_iter(ids.into_iter().map(JsValue::from))
    }

    #[wasm_bindgen(js_name = issuerCurve)]
    pub fn issuer_curve(&self, issuer_id: &str) -> Option<JsHazardCurve> {
        self.inner
            .issuer_credit_curves
            .as_ref()
            .and_then(|map| map.get(issuer_id))
            .map(|arc| JsHazardCurve::from_arc(Arc::clone(arc)))
    }
}

#[wasm_bindgen(js_name = VolatilityIndexCurve)]
#[derive(Clone)]
pub struct JsVolatilityIndexCurve {
    inner: Arc<finstack_core::market_data::term_structures::VolatilityIndexCurve>,
}

impl JsVolatilityIndexCurve {
    #[allow(dead_code)] // Used when inserted into MarketContext from external callers
    pub(crate) fn from_arc(
        inner: Arc<finstack_core::market_data::term_structures::VolatilityIndexCurve>,
    ) -> Self {
        Self { inner }
    }

    #[allow(dead_code)] // Used when extracted from MarketContext by external callers
    pub(crate) fn inner(
        &self,
    ) -> Arc<finstack_core::market_data::term_structures::VolatilityIndexCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = VolatilityIndexCurve)]
impl JsVolatilityIndexCurve {
    /// Create a volatility index curve with (time, forward_level) knot points.
    ///
    /// @param {string} id - Curve identifier used to retrieve it later from MarketContext
    /// @param {Date} base_date - Anchor date corresponding to t = 0
    /// @param {Array<number>} times - Time knots in years from base_date
    /// @param {Array<number>} levels - Forward volatility index levels at each time point
    /// @param {number} spot_level - Current spot level of the volatility index
    /// @param {string} day_count - Day count convention (e.g., "act_365f")
    /// @param {string} interp - Interpolation style ("linear", "monotone_convex", etc.)
    /// @param {string} extrapolation - Extrapolation policy ("flat_forward")
    /// @returns {VolatilityIndexCurve} Curve object exposing forward level methods
    /// @throws {Error} If knots are invalid, times/levels length mismatch, or fewer than 2 points
    ///
    /// @example
    /// ```javascript
    /// const baseDate = new Date(2024, 1, 2);
    /// const curve = new VolatilityIndexCurve(
    ///   "VIX",
    ///   baseDate,
    ///   [0.0, 0.25, 0.5, 1.0, 2.0],      // times in years
    ///   [18.0, 19.5, 20.0, 21.5, 22.0],  // forward volatility index levels
    ///   18.0,                             // spot level
    ///   "act_365f",                       // day count
    ///   "linear",                         // interpolation
    ///   "flat_forward"                    // extrapolation
    /// );
    ///
    /// console.log(curve.forwardLevel(0.5));  // 20.0 (forward level at 6 months)
    /// console.log(curve.spotLevel);          // 18.0 (spot level)
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        base_date: &JsDate,
        times: Vec<f64>,
        levels: Vec<f64>,
        spot_level: Option<f64>,
        day_count: JsValue,
        interp: JsValue,
        extrapolation: JsValue,
    ) -> Result<JsVolatilityIndexCurve, JsValue> {
        if times.len() != levels.len() {
            return Err(js_error("times and levels must have the same length"));
        }
        if times.len() < 2 {
            return Err(js_error(
                "at least two knots are required to build a volatility index curve",
            ));
        }

        let points: Vec<(f64, f64)> = times.into_iter().zip(levels).collect();
        let style = parse_interp_value(&interp)?;
        let extrap = parse_extrap_value(&extrapolation)?;
        let picked_day_count = parse_day_count_jsvalue(&day_count)?.unwrap_or(DayCount::Act365F);

        let mut builder =
            finstack_core::market_data::term_structures::VolatilityIndexCurve::builder(id)
                .base_date(base_date.inner())
                .knots(points)
                .interp(style)
                .extrapolation(extrap)
                .day_count(picked_day_count);

        if let Some(spot) = spot_level {
            builder = builder.spot_level(spot);
        }

        let curve = builder.build().map_err(core_to_js)?;
        Ok(JsVolatilityIndexCurve {
            inner: Arc::new(curve),
        })
    }

    #[wasm_bindgen(getter, js_name = id)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date_js(&self) -> JsDate {
        JsDate::from_core(self.inner.base_date())
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count_name(&self) -> String {
        format!("{:?}", self.inner.day_count())
    }

    #[wasm_bindgen(getter, js_name = spotLevel)]
    pub fn spot_level(&self) -> f64 {
        self.inner.spot_level()
    }

    #[wasm_bindgen(js_name = forwardLevel)]
    pub fn forward_level(&self, time: f64) -> f64 {
        self.inner.forward_level(time)
    }

    #[wasm_bindgen(js_name = points)]
    pub fn points(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for (&t, &lvl) in self.inner.knots().iter().zip(self.inner.levels().iter()) {
            let tuple = js_sys::Array::new();
            tuple.push(&JsValue::from_f64(t));
            tuple.push(&JsValue::from_f64(lvl));
            arr.push(&tuple);
        }
        arr
    }
}

// ======================================================================
// FlatCurve
// ======================================================================

/// A term structure with a constant continuously compounded rate.
///
/// Useful for approximate valuations, performance metrics (NPV/IRR),
/// and testing.
///
/// @example
/// ```javascript
/// const curve = new FlatCurve(0.05, baseDate, "act_365f", "FLAT-5%");
/// const df = curve.df(1.0);  // e^(-0.05) ≈ 0.9512
/// ```
#[wasm_bindgen(js_name = FlatCurve)]
#[derive(Clone)]
pub struct JsFlatCurve {
    inner: Arc<FlatCurve>,
}

impl JsFlatCurve {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Arc<FlatCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = FlatCurve)]
impl JsFlatCurve {
    /// Create a flat curve with a constant rate.
    ///
    /// @param {number} rate - Continuously compounded annual rate (decimal)
    /// @param {FsDate} baseDate - Reference date
    /// @param {string} dayCount - Day count convention name
    /// @param {string} id - Curve identifier
    #[wasm_bindgen(constructor)]
    pub fn new(
        rate: f64,
        base_date: &JsDate,
        day_count: &JsValue,
        id: &str,
    ) -> Result<JsFlatCurve, JsValue> {
        let dc =
            parse_day_count_jsvalue(day_count)?.unwrap_or(finstack_core::dates::DayCount::Act365F);
        Ok(JsFlatCurve {
            inner: Arc::new(FlatCurve::new(rate, base_date.inner(), dc, id)),
        })
    }

    /// The constant rate.
    #[wasm_bindgen(getter)]
    pub fn rate(&self) -> f64 {
        self.inner.rate()
    }

    /// Base date for the curve.
    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> JsDate {
        use finstack_core::market_data::traits::Discounting;
        JsDate::from_core(self.inner.base_date())
    }

    /// Discount factor at time t (years from base date).
    ///
    /// @param {number} t - Time in years
    /// @returns {number} Discount factor
    #[wasm_bindgen(js_name = df)]
    pub fn df(&self, t: f64) -> f64 {
        use finstack_core::market_data::traits::Discounting;
        self.inner.df(t)
    }

    /// Curve identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        use finstack_core::market_data::traits::TermStructure;
        self.inner.id().to_string()
    }
}

// ======================================================================
// PriceCurve
// ======================================================================

/// Forward price curve for commodities and other price-based assets.
///
/// @example
/// ```javascript
/// const curve = PriceCurve.build("WTI-FWD", baseDate, "act_365f", 75.0,
///   [0.0, 0.25, 0.5, 1.0], [75.0, 76.5, 77.2, 78.0], "linear", "flat_zero");
/// console.log(curve.price(0.25));  // 76.5
/// ```
#[wasm_bindgen(js_name = PriceCurve)]
#[derive(Clone)]
pub struct JsPriceCurve {
    inner: Arc<PriceCurve>,
}

impl JsPriceCurve {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Arc<PriceCurve> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = PriceCurve)]
impl JsPriceCurve {
    /// Build a price curve from knot points.
    ///
    /// @param {string} id - Curve identifier
    /// @param {FsDate} baseDate - Reference date
    /// @param {string} dayCount - Day count convention name
    /// @param {number} spotPrice - Current spot price
    /// @param {Float64Array} times - Knot times in years
    /// @param {Float64Array} prices - Forward prices at each knot
    /// @param {string} [interp] - Interpolation style (default: "linear")
    /// @param {string} [extrapolation] - Extrapolation policy (default: "flat_zero")
    #[wasm_bindgen(js_name = build)]
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        id: &str,
        base_date: &JsDate,
        day_count: &JsValue,
        spot_price: f64,
        times: Vec<f64>,
        prices: Vec<f64>,
        interp: Option<String>,
        extrapolation: Option<String>,
    ) -> Result<JsPriceCurve, JsValue> {
        if times.len() != prices.len() {
            return Err(js_error("times and prices must have the same length"));
        }

        let dc =
            parse_day_count_jsvalue(day_count)?.unwrap_or(finstack_core::dates::DayCount::Act365F);
        let interp_style = parse_interp_value(
            &interp
                .map(|s| JsValue::from_str(&s))
                .unwrap_or(JsValue::UNDEFINED),
        )?;
        let extrap_policy = parse_extrap_value(
            &extrapolation
                .map(|s| JsValue::from_str(&s))
                .unwrap_or(JsValue::UNDEFINED),
        )?;

        let knots: Vec<(f64, f64)> = times.into_iter().zip(prices).collect();

        let curve = PriceCurve::builder(id)
            .base_date(base_date.inner())
            .day_count(dc)
            .spot_price(spot_price)
            .knots(knots)
            .interp(interp_style)
            .extrapolation(extrap_policy)
            .build()
            .map_err(core_to_js)?;

        Ok(JsPriceCurve {
            inner: Arc::new(curve),
        })
    }

    /// Forward price at time t (years from base date).
    ///
    /// @param {number} t - Time in years
    /// @returns {number} Forward price
    #[wasm_bindgen(js_name = price)]
    pub fn price(&self, t: f64) -> f64 {
        self.inner.price(t)
    }

    /// Spot price (price at t=0).
    #[wasm_bindgen(getter, js_name = spotPrice)]
    pub fn spot_price(&self) -> f64 {
        self.inner.spot_price()
    }

    /// Curve identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date for the curve.
    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> JsDate {
        JsDate::from_core(self.inner.base_date())
    }
}
