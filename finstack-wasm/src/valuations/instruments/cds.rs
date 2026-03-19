use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, PremiumLegSpec, ProtectionLegSpec, RECOVERY_SENIOR_UNSECURED,
};
use finstack_valuations::instruments::{Attributes, PayReceive, PricingOverrides};
use finstack_valuations::pricer::InstrumentType;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CreditDefaultSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCreditDefaultSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    spread_bp: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    side: Option<String>,
    recovery_rate: Option<f64>,
}

#[wasm_bindgen(js_class = CreditDefaultSwapBuilder)]
impl JsCreditDefaultSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCreditDefaultSwapBuilder {
        JsCreditDefaultSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsCreditDefaultSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = spreadBp)]
    pub fn spread_bp(mut self, spread_bp: f64) -> JsCreditDefaultSwapBuilder {
        self.spread_bp = Some(spread_bp);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsCreditDefaultSwapBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsCreditDefaultSwapBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsCreditDefaultSwapBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = creditCurve)]
    pub fn credit_curve(mut self, credit_curve: &str) -> JsCreditDefaultSwapBuilder {
        self.credit_curve = Some(credit_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: String) -> JsCreditDefaultSwapBuilder {
        self.side = Some(side);
        self
    }

    #[wasm_bindgen(js_name = recoveryRate)]
    pub fn recovery_rate(mut self, recovery_rate: f64) -> JsCreditDefaultSwapBuilder {
        self.recovery_rate = Some(recovery_rate);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCreditDefaultSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            JsValue::from_str("CreditDefaultSwapBuilder: notional (money) is required")
        })?;
        let spread_bp = self
            .spread_bp
            .ok_or_else(|| JsValue::from_str("CreditDefaultSwapBuilder: spreadBp is required"))?;
        let start_date = self
            .start_date
            .ok_or_else(|| JsValue::from_str("CreditDefaultSwapBuilder: startDate is required"))?;
        let maturity = self
            .maturity
            .ok_or_else(|| JsValue::from_str("CreditDefaultSwapBuilder: maturity is required"))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            JsValue::from_str("CreditDefaultSwapBuilder: discountCurve is required")
        })?;
        let credit_curve = self.credit_curve.as_deref().ok_or_else(|| {
            JsValue::from_str("CreditDefaultSwapBuilder: creditCurve is required")
        })?;
        let side = self
            .side
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CreditDefaultSwapBuilder: side is required"))?;

        let id = instrument_id_from_str(&self.instrument_id);
        let disc = curve_id_from_str(discount_curve);
        let credit = curve_id_from_str(credit_curve);

        let side = match side.to_lowercase().as_str() {
            "buy_protection" => PayReceive::PayFixed,
            "sell_protection" => PayReceive::ReceiveFixed,
            other => {
                return Err(JsValue::from_str(&format!(
                    "Invalid side '{other}'; expected 'buy_protection' or 'sell_protection'"
                )));
            }
        };

        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
            JsValue::from_str(&format!(
                "spread_bp {} cannot be represented as Decimal: {e}",
                spread_bp
            ))
        })?;

        let mut cds = CreditDefaultSwap::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .convention(convention)
            .premium(PremiumLegSpec {
                start: start_date,
                end: maturity,
                frequency: freq,
                stub,
                bdc,
                calendar_id: Some(convention.default_calendar().to_string()),
                day_count: dc,
                spread_bp: spread_bp_decimal,
                discount_curve_id: disc,
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: credit,
                recovery_rate: RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(rr) = self.recovery_rate {
            cds.protection.recovery_rate = rr;
        }

        cds.validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsCreditDefaultSwap::from_inner(cds))
    }
}

#[wasm_bindgen(js_name = CreditDefaultSwap)]
#[derive(Clone, Debug)]
pub struct JsCreditDefaultSwap {
    pub(crate) inner: CreditDefaultSwap,
}

impl InstrumentWrapper for JsCreditDefaultSwap {
    type Inner = CreditDefaultSwap;
    fn from_inner(inner: CreditDefaultSwap) -> Self {
        JsCreditDefaultSwap { inner }
    }
    fn inner(&self) -> CreditDefaultSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CreditDefaultSwap)]
impl JsCreditDefaultSwap {
    /// Parse a CDS from a JSON value (as produced by `toJson`).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCreditDefaultSwap, JsValue> {
        from_js_value(value).map(JsCreditDefaultSwap::from_inner)
    }

    /// Serialize this CDS to a JSON value.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get premium-leg cashflows for this CDS.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    /// - kind is always \"Premium\" for CDS cashflows
    /// - outstanding_balance is null (not applicable)
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<js_sys::Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use js_sys::Array;

        let flows = self
            .inner
            .build_premium_schedule(market.inner(), self.inner.premium.start)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let result = Array::new();
        for (d, amt) in flows {
            let entry = Array::new();
            entry.push(&JsDate::from_core(d).into());
            entry.push(&JsMoney::from_inner(amt).into());
            entry.push(&JsValue::from_str("Premium"));
            entry.push(&JsValue::NULL);
            result.push(&entry);
        }
        Ok(result)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = spreadBp)]
    pub fn spread_bp(&self) -> f64 {
        self.inner.premium.spread_bp.to_f64().unwrap_or(0.0)
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.premium.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = creditCurve)]
    pub fn credit_curve(&self) -> String {
        self.inner.protection.credit_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.start)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.end)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CDS.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CreditDefaultSwap(id='{}', spread_bp={:.1})",
            self.inner.id, self.inner.premium.spread_bp
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCreditDefaultSwap {
        JsCreditDefaultSwap::from_inner(self.inner.clone())
    }
}
