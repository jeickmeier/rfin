use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::credit_derivatives::cds::{
    CDSConvention, PayReceive as CdsPayReceive,
};
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::credit_derivatives::cds_index::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::CreditParams;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use rust_decimal::prelude::ToPrimitive;
use wasm_bindgen::prelude::*;

const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

#[wasm_bindgen(js_name = CDSIndex)]
#[derive(Clone, Debug)]
pub struct JsCDSIndex {
    pub(crate) inner: CDSIndex,
}

impl InstrumentWrapper for JsCDSIndex {
    type Inner = CDSIndex;
    fn from_inner(inner: CDSIndex) -> Self {
        JsCDSIndex { inner }
    }
    fn inner(&self) -> CDSIndex {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = CDSIndexBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCDSIndexBuilder {
    instrument_id: String,
    index_name: Option<String>,
    series: Option<u16>,
    version: Option<u16>,
    notional: Option<finstack_core::money::Money>,
    fixed_coupon_bp: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    side: Option<String>,
    recovery_rate: Option<f64>,
    index_factor: Option<f64>,
}

#[wasm_bindgen(js_class = CDSIndexBuilder)]
impl JsCDSIndexBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCDSIndexBuilder {
        JsCDSIndexBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = indexName)]
    pub fn index_name(mut self, index_name: String) -> JsCDSIndexBuilder {
        self.index_name = Some(index_name);
        self
    }

    #[wasm_bindgen(js_name = series)]
    pub fn series(mut self, series: u16) -> JsCDSIndexBuilder {
        self.series = Some(series);
        self
    }

    #[wasm_bindgen(js_name = version)]
    pub fn version(mut self, version: u16) -> JsCDSIndexBuilder {
        self.version = Some(version);
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsCDSIndexBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedCouponBp)]
    pub fn fixed_coupon_bp(mut self, fixed_coupon_bp: f64) -> JsCDSIndexBuilder {
        self.fixed_coupon_bp = Some(fixed_coupon_bp);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsCDSIndexBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsCDSIndexBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsCDSIndexBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = creditCurve)]
    pub fn credit_curve(mut self, credit_curve: &str) -> JsCDSIndexBuilder {
        self.credit_curve = Some(credit_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: String) -> JsCDSIndexBuilder {
        self.side = Some(side);
        self
    }

    #[wasm_bindgen(js_name = recoveryRate)]
    pub fn recovery_rate(mut self, recovery_rate: f64) -> JsCDSIndexBuilder {
        self.recovery_rate = Some(recovery_rate);
        self
    }

    #[wasm_bindgen(js_name = indexFactor)]
    pub fn index_factor(mut self, index_factor: f64) -> JsCDSIndexBuilder {
        self.index_factor = Some(index_factor);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCDSIndex, JsValue> {
        let index_name = self
            .index_name
            .as_deref()
            .ok_or_else(|| js_error("CDSIndexBuilder: indexName is required".to_string()))?;
        let series = self
            .series
            .ok_or_else(|| js_error("CDSIndexBuilder: series is required".to_string()))?;
        let version = self
            .version
            .ok_or_else(|| js_error("CDSIndexBuilder: version is required".to_string()))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("CDSIndexBuilder: notional (money) is required".to_string()))?;
        let fixed_coupon_bp = self
            .fixed_coupon_bp
            .ok_or_else(|| js_error("CDSIndexBuilder: fixedCouponBp is required".to_string()))?;
        let start_date = self
            .start_date
            .ok_or_else(|| js_error("CDSIndexBuilder: startDate is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("CDSIndexBuilder: maturity is required".to_string()))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("CDSIndexBuilder: discountCurve is required".to_string()))?;
        let credit_curve = self
            .credit_curve
            .as_deref()
            .ok_or_else(|| js_error("CDSIndexBuilder: creditCurve is required".to_string()))?;

        let side_value = parse_optional_with_default(self.side, CdsPayReceive::PayFixed)?;
        let recovery = self.recovery_rate.unwrap_or(STANDARD_RECOVERY_SENIOR);
        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let mut index_params = CDSIndexParams::new(index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = self.index_factor {
            index_params = index_params.with_index_factor(factor);
        }

        let construction =
            CDSIndexConstructionParams::new(notional, side_value, CDSConvention::IsdaNa);

        let disc_curve = curve_id_from_str(discount_curve);
        let credit_curve_id = curve_id_from_str(credit_curve);
        let credit_params =
            CreditParams::new(index_name.to_string(), recovery, credit_curve_id.clone());

        let index = CDSIndex::new_standard(
            instrument_id_from_str(&self.instrument_id),
            &index_params,
            &construction,
            start_date,
            maturity,
            &credit_params,
            disc_curve,
            credit_curve_id,
        )
        .map_err(js_error)?;

        Ok(JsCDSIndex::from_inner(index))
    }
}

#[wasm_bindgen(js_class = CDSIndex)]
impl JsCDSIndex {
    /// Create a standardized CDS index instrument.
    ///
    /// Conventions:
    /// - `fixed_coupon_bp` is in **basis points** (e.g. `100.0` for 1%).
    /// - `recovery_rate` is in **decimal** (e.g. `0.40`).
    /// - `side` defaults to paying fixed (selling protection leg direction depends on model conventions).
    ///
    /// @param instrument_id - Unique identifier
    /// @param index_name - Index name (e.g. `"CDX.NA.IG"`)
    /// @param series - Series number
    /// @param version - Version number
    /// @param notional - Index notional (currency-tagged)
    /// @param fixed_coupon_bp - Fixed coupon in basis points
    /// @param start_date - Effective start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param credit_curve - Credit/hazard curve ID
    /// @param side - Optional pay/receive side string
    /// @param recovery_rate - Optional recovery rate override (0..1)
    /// @param index_factor - Optional factor for index notionals
    /// @returns A new `CDSIndex`
    /// @throws {Error} If recovery is outside [0,1] or inputs invalid
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        index_name: &str,
        series: u16,
        version: u16,
        notional: &JsMoney,
        fixed_coupon_bp: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        side: Option<String>,
        recovery_rate: Option<f64>,
        index_factor: Option<f64>,
    ) -> Result<JsCDSIndex, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "CDSIndex constructor is deprecated; use CDSIndexBuilder instead.",
        ));
        let side_value = parse_optional_with_default(side, CdsPayReceive::PayFixed)?;
        let recovery = recovery_rate.unwrap_or(STANDARD_RECOVERY_SENIOR);

        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let mut index_params = CDSIndexParams::new(index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = index_factor {
            index_params = index_params.with_index_factor(factor);
        }

        let construction =
            CDSIndexConstructionParams::new(notional.inner(), side_value, CDSConvention::IsdaNa);

        let disc_curve = curve_id_from_str(discount_curve);
        let credit_curve_id = curve_id_from_str(credit_curve);

        let credit_params =
            CreditParams::new(index_name.to_string(), recovery, credit_curve_id.clone());

        let index = CDSIndex::new_standard(
            instrument_id_from_str(instrument_id),
            &index_params,
            &construction,
            start_date.inner(),
            maturity.inner(),
            &credit_params,
            disc_curve,
            credit_curve_id,
        )
        .map_err(js_error)?;

        Ok(JsCDSIndex::from_inner(index))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCDSIndex, JsValue> {
        from_js_value(value).map(JsCDSIndex::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get premium-leg cashflows for this CDS index.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use finstack_core::dates::DayCountCtx;
        use finstack_valuations::cashflow::builder::build_dates;

        let disc = market
            .inner()
            .get_discount(self.inner.premium.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = build_dates(
            self.inner.premium.start,
            self.inner.premium.end,
            self.inner.premium.freq,
            self.inner.premium.stub,
            self.inner.premium.bdc,
            false,
            0,
            self.inner
                .premium
                .calendar_id
                .as_deref()
                .unwrap_or(finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
        )
        .map_err(|e| js_error(e.to_string()))?;

        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(Array::new());
        }

        let spread_decimal = self.inner.premium.spread_bp.to_f64().unwrap_or(0.0) / 10000.0;

        let result = Array::new();
        let mut prev = dates[0];
        for &d in &dates[1..] {
            if d <= as_of {
                prev = d;
                continue;
            }
            let year_frac = self
                .inner
                .premium
                .dc
                .year_fraction(prev, d, DayCountCtx::default())
                .map_err(|e| js_error(e.to_string()))?;
            let amount = self.inner.notional.amount() * spread_decimal * year_frac;
            let entry = Array::new();
            entry.push(&JsDate::from_core(d).into());
            entry.push(
                &JsMoney::from_inner(finstack_core::money::Money::new(
                    amount,
                    self.inner.notional.currency(),
                ))
                .into(),
            );
            entry.push(&JsValue::from_str("Premium"));
            entry.push(&JsValue::NULL);
            result.push(&entry);
            prev = d;
        }

        Ok(result)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = indexName)]
    pub fn index_name(&self) -> String {
        self.inner.index_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedCouponBp)]
    pub fn fixed_coupon_bp(&self) -> f64 {
        self.inner.premium.spread_bp.to_f64().unwrap_or(0.0)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.end)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDSIndex as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CDSIndex(id='{}', name='{}', series={})",
            self.inner.id, self.inner.index_name, self.inner.series
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCDSIndex {
        JsCDSIndex::from_inner(self.inner.clone())
    }
}
