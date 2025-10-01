use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency};
use finstack_valuations::instruments::cds_tranche::{CdsTranche, TrancheSide};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_tranche_side(label: Option<String>) -> Result<TrancheSide, JsValue> {
    match label.as_deref() {
        None | Some("buy_protection") => Ok(TrancheSide::BuyProtection),
        Some("sell_protection") => Ok(TrancheSide::SellProtection),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid tranche side: {e}"))),
    }
}

fn parse_frequency(payments_per_year: Option<u32>) -> Result<Frequency, JsValue> {
    let payments = payments_per_year.unwrap_or(4);
    Frequency::from_payments_per_year(payments)
        .map_err(|e| js_error(format!("Invalid payments per year: {e}")))
}

#[wasm_bindgen(js_name = CdsTranche)]
#[derive(Clone, Debug)]
pub struct JsCdsTranche {
    inner: CdsTranche,
}

impl JsCdsTranche {
    pub(crate) fn from_inner(inner: CdsTranche) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> CdsTranche {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CdsTranche)]
impl JsCdsTranche {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        index_name: &str,
        series: u16,
        attach_pct: f64,
        detach_pct: f64,
        notional: &JsMoney,
        maturity: &JsDate,
        running_coupon_bp: f64,
        discount_curve: &str,
        credit_index_curve: &str,
        side: Option<String>,
        payments_per_year: Option<u32>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsCdsTranche, JsValue> {
        if attach_pct < 0.0 || detach_pct <= attach_pct {
            return Err(js_error(
                "detach_pct must be greater than attach_pct and both non-negative".to_string(),
            ));
        }

        let side_value = parse_tranche_side(side)?;
        let freq = parse_frequency(payments_per_year)?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);

        let builder = CdsTranche::builder()
            .id(instrument_id_from_str(instrument_id))
            .index_name(index_name.to_string())
            .series(series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(notional.inner())
            .maturity(maturity.inner())
            .running_coupon_bp(running_coupon_bp)
            .payment_frequency(freq)
            .day_count(dc)
            .business_day_convention(BusinessDayConvention::Following)
            .disc_id(curve_id_from_str(discount_curve))
            .credit_index_id(curve_id_from_str(credit_index_curve))
            .side(side_value)
            .attributes(Default::default());

        builder
            .build()
            .map(JsCdsTranche::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = attachPct)]
    pub fn attach_pct(&self) -> f64 {
        self.inner.attach_pct
    }

    #[wasm_bindgen(getter, js_name = detachPct)]
    pub fn detach_pct(&self) -> f64 {
        self.inner.detach_pct
    }

    #[wasm_bindgen(getter, js_name = runningCouponBp)]
    pub fn running_coupon_bp(&self) -> f64 {
        self.inner.running_coupon_bp
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDSTranche as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CdsTranche(id='{}', attach={:.2}%, detach={:.2}%)",
            self.inner.id, self.inner.attach_pct, self.inner.detach_pct
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCdsTranche {
        JsCdsTranche::from_inner(self.inner.clone())
    }
}

