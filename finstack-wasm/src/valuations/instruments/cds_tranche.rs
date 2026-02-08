use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_valuations::instruments::credit_derivatives::cds_tranche::{CDSTranche, TrancheSide};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CdsTranche)]
#[derive(Clone, Debug)]
pub struct JsCDSTranche {
    pub(crate) inner: CDSTranche,
}

impl InstrumentWrapper for JsCDSTranche {
    type Inner = CDSTranche;
    fn from_inner(inner: CDSTranche) -> Self {
        JsCDSTranche { inner }
    }
    fn inner(&self) -> CDSTranche {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = CdsTrancheBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCDSTrancheBuilder {
    instrument_id: String,
    index_name: Option<String>,
    series: Option<u16>,
    attach_pct: Option<f64>,
    detach_pct: Option<f64>,
    notional: Option<finstack_core::money::Money>,
    maturity: Option<finstack_core::dates::Date>,
    running_coupon_bp: Option<f64>,
    discount_curve: Option<String>,
    credit_index_curve: Option<String>,
    side: Option<String>,
    payments_per_year: Option<u32>,
    day_count: Option<DayCount>,
}

#[wasm_bindgen(js_class = CDSTrancheBuilder)]
impl JsCDSTrancheBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCDSTrancheBuilder {
        JsCDSTrancheBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = indexName)]
    pub fn index_name(mut self, index_name: String) -> JsCDSTrancheBuilder {
        self.index_name = Some(index_name);
        self
    }

    #[wasm_bindgen(js_name = series)]
    pub fn series(mut self, series: u16) -> JsCDSTrancheBuilder {
        self.series = Some(series);
        self
    }

    #[wasm_bindgen(js_name = attachPct)]
    pub fn attach_pct(mut self, attach_pct: f64) -> JsCDSTrancheBuilder {
        self.attach_pct = Some(attach_pct);
        self
    }

    #[wasm_bindgen(js_name = detachPct)]
    pub fn detach_pct(mut self, detach_pct: f64) -> JsCDSTrancheBuilder {
        self.detach_pct = Some(detach_pct);
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsCDSTrancheBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsCDSTrancheBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = runningCouponBp)]
    pub fn running_coupon_bp(mut self, running_coupon_bp: f64) -> JsCDSTrancheBuilder {
        self.running_coupon_bp = Some(running_coupon_bp);
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsCDSTrancheBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = creditIndexCurve)]
    pub fn credit_index_curve(mut self, credit_index_curve: &str) -> JsCDSTrancheBuilder {
        self.credit_index_curve = Some(credit_index_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: String) -> JsCDSTrancheBuilder {
        self.side = Some(side);
        self
    }

    #[wasm_bindgen(js_name = paymentsPerYear)]
    pub fn payments_per_year(mut self, payments_per_year: u32) -> JsCDSTrancheBuilder {
        self.payments_per_year = Some(payments_per_year);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsCDSTrancheBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCDSTranche, JsValue> {
        let index_name = self
            .index_name
            .as_deref()
            .ok_or_else(|| js_error("CDSTrancheBuilder: indexName is required".to_string()))?;
        let series = self
            .series
            .ok_or_else(|| js_error("CDSTrancheBuilder: series is required".to_string()))?;
        let attach_pct = self
            .attach_pct
            .ok_or_else(|| js_error("CDSTrancheBuilder: attachPct is required".to_string()))?;
        let detach_pct = self
            .detach_pct
            .ok_or_else(|| js_error("CDSTrancheBuilder: detachPct is required".to_string()))?;
        let notional = self.notional.ok_or_else(|| {
            js_error("CDSTrancheBuilder: notional (money) is required".to_string())
        })?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("CDSTrancheBuilder: maturity is required".to_string()))?;
        let running_coupon_bp = self.running_coupon_bp.ok_or_else(|| {
            js_error("CDSTrancheBuilder: runningCouponBp is required".to_string())
        })?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("CDSTrancheBuilder: discountCurve is required".to_string()))?;
        let credit_index_curve = self.credit_index_curve.as_deref().ok_or_else(|| {
            js_error("CDSTrancheBuilder: creditIndexCurve is required".to_string())
        })?;

        if attach_pct < 0.0 || detach_pct <= attach_pct {
            return Err(js_error(
                "detach_pct must be greater than attach_pct and both non-negative".to_string(),
            ));
        }

        let side_value = parse_optional_with_default(self.side, TrancheSide::BuyProtection)?;
        let freq = match self.payments_per_year {
            Some(ppy) => Tenor::from_payments_per_year(ppy)
                .map_err(|e| js_error(format!("Invalid payments per year: {}", e)))?,
            None => Tenor::quarterly(),
        };
        let dc = self.day_count.unwrap_or(DayCount::Act360);

        CDSTranche::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .index_name(index_name.to_string())
            .series(series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(notional)
            .maturity(maturity)
            .running_coupon_bp(running_coupon_bp)
            .payment_frequency(freq)
            .day_count(dc)
            .business_day_convention(BusinessDayConvention::Following)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .credit_index_id(curve_id_from_str(credit_index_curve))
            .side(side_value)
            .accumulated_loss(0.0)
            .standard_imm_dates(false)
            .attributes(Default::default())
            .build()
            .map(JsCDSTranche::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = CDSTranche)]
impl JsCDSTranche {
    /// Create a synthetic CDO tranche instrument (CDS tranche).
    ///
    /// Conventions:
    /// - `attach_pct` / `detach_pct` are tranche attachment/detachment in **percent** (0..100).
    /// - `running_coupon_bp` is in **basis points**.
    /// - Payment frequency defaults to quarterly when omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param index_name - Underlying index name (e.g. `"CDX.NA.IG"`)
    /// @param series - Series number
    /// @param attach_pct - Attachment point (percent)
    /// @param detach_pct - Detachment point (percent)
    /// @param notional - Tranche notional (currency-tagged)
    /// @param maturity - Maturity date
    /// @param running_coupon_bp - Running coupon in bps
    /// @param discount_curve - Discount curve ID
    /// @param credit_index_curve - Credit index curve ID
    /// @param side - Optional tranche side string
    /// @param payments_per_year - Optional payment frequency via payments/year
    /// @param day_count - Optional day count (defaults Act/360)
    /// @returns A new `CDSTranche`
    /// @throws {Error} If detach <= attach or inputs invalid
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
    ) -> Result<JsCDSTranche, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "CDSTranche constructor is deprecated; use CDSTrancheBuilder instead.",
        ));
        if attach_pct < 0.0 || detach_pct <= attach_pct {
            return Err(js_error(
                "detach_pct must be greater than attach_pct and both non-negative".to_string(),
            ));
        }

        let side_value = parse_optional_with_default(side, TrancheSide::BuyProtection)?;
        let freq = match payments_per_year {
            Some(ppy) => Tenor::from_payments_per_year(ppy)
                .map_err(|e| js_error(format!("Invalid payments per year: {}", e)))?,
            None => Tenor::quarterly(),
        };
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);

        let builder = CDSTranche::builder()
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
            .discount_curve_id(curve_id_from_str(discount_curve))
            .credit_index_id(curve_id_from_str(credit_index_curve))
            .side(side_value)
            .accumulated_loss(0.0) // Default: no accumulated losses
            .standard_imm_dates(false) // Default: not using IMM dates
            .attributes(Default::default());

        builder
            .build()
            .map(JsCDSTranche::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCDSTranche, JsValue> {
        from_js_value(value).map(JsCDSTranche::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get premium-leg cashflows for this CDS tranche.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use finstack_core::dates::{DayCountCtx, StubKind};
        use finstack_valuations::cashflow::builder::build_dates;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let start = self.inner.effective_date.unwrap_or(as_of);
        let sched = build_dates(
            start,
            self.inner.maturity,
            self.inner.payment_frequency,
            StubKind::None,
            self.inner.business_day_convention,
            false,
            0,
            self.inner
                .calendar_id
                .as_deref()
                .unwrap_or(finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
        )
        .map_err(|e| js_error(e.to_string()))?;

        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(Array::new());
        }

        let spread_decimal = self.inner.running_coupon_bp / 10000.0;
        let sign = match self.inner.side {
            TrancheSide::BuyProtection => -1.0,
            TrancheSide::SellProtection => 1.0,
        };

        let result = Array::new();
        let mut prev = dates[0];
        for &d in &dates[1..] {
            if d <= as_of {
                prev = d;
                continue;
            }
            let year_frac = self
                .inner
                .day_count
                .year_fraction(prev, d, DayCountCtx::default())
                .map_err(|e| js_error(e.to_string()))?;
            let amount = sign * self.inner.notional.amount() * spread_decimal * year_frac;
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
            "CDSTranche(id='{}', attach={:.2}%, detach={:.2}%)",
            self.inner.id, self.inner.attach_pct, self.inner.detach_pct
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCDSTranche {
        JsCDSTranche::from_inner(self.inner.clone())
    }
}
