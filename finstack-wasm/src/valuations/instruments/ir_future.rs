use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::ir_future::{
    FutureContractSpecs, InterestRateFuture, Position,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateFuture)]
#[derive(Clone, Debug)]
pub struct JsInterestRateFuture {
    pub(crate) inner: InterestRateFuture,
}

impl InstrumentWrapper for JsInterestRateFuture {
    type Inner = InterestRateFuture;
    fn from_inner(inner: InterestRateFuture) -> Self {
        JsInterestRateFuture { inner }
    }
    fn inner(&self) -> InterestRateFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateFuture)]
impl JsInterestRateFuture {
    /// Create an interest rate future (e.g., Eurodollar/SOFR future style).
    ///
    /// Conventions:
    /// - `quoted_price` is typically quoted as `100 - implied_rate*100` (market convention varies by contract).
    /// - By default, convexity adjustment is set to 0.0 to avoid requiring a vol surface.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Contract notional (currency-tagged)
    /// @param quoted_price - Quoted futures price
    /// @param expiry - Expiry date
    /// @param fixing_date - Fixing date
    /// @param period_start - Underlying accrual period start
    /// @param period_end - Underlying accrual period end
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID
    /// @param position - Optional position string (e.g. `"long"`)
    /// @param day_count - Optional day count (defaults Act/360)
    /// @returns A new `InterestRateFuture`
    /// @throws {Error} If inputs are invalid or parsing fails
    ///
    /// @example
    /// ```javascript
    /// import init, { InterestRateFuture, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const fut = new InterestRateFuture(
    ///   "fut_1",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   95.25,
    ///   new FsDate(2024, 12, 18),
    ///   new FsDate(2024, 12, 18),
    ///   new FsDate(2024, 12, 18),
    ///   new FsDate(2025, 3, 19),
    ///   "USD-OIS",
    ///   "USD-SOFR-3M"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        quoted_price: f64,
        expiry: &JsDate,
        fixing_date: &JsDate,
        period_start: &JsDate,
        period_end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        position: Option<String>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateFuture, JsValue> {
        let position_value = parse_optional_with_default(position, Position::Long)?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);

        // Use default contract specs with convexity adjustment set to 0.0
        // to avoid requiring a volatility surface for simple pricing
        let contract_specs = FutureContractSpecs {
            convexity_adjustment: Some(0.0),
            ..Default::default()
        };

        let builder = InterestRateFuture::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .quoted_price(quoted_price)
            .expiry_date(expiry.inner())
            .fixing_date(fixing_date.inner())
            .period_start(period_start.inner())
            .period_end(period_end.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .forward_id(curve_id_from_str(forward_curve))
            .day_count(dc)
            .position(position_value)
            .contract_specs(contract_specs)
            .attributes(Default::default());

        builder
            .build()
            .map(JsInterestRateFuture::from_inner)
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

    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InterestRateFuture as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateFuture {
        JsInterestRateFuture::from_inner(self.inner.clone())
    }
}
