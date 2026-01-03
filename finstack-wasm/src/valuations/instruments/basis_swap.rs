use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::rates::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = BasisSwapLeg)]
#[derive(Clone, Debug)]
pub struct JsBasisSwapLeg {
    pub(crate) inner: BasisSwapLeg,
}

#[wasm_bindgen(js_class = BasisSwapLeg)]
impl JsBasisSwapLeg {
    /// Create a basis swap floating leg specification.
    ///
    /// Conventions:
    /// - `spread` is a **decimal rate** (not bps) applied to the index rate.
    /// - `frequency` and `day_count` are parsed from strings (e.g. `"3M"`, `"act_360"`).
    ///
    /// @param forward_curve - Forward curve ID (e.g. `"USD-SOFR-3M"`)
    /// @param frequency - Optional payment/reset frequency (e.g. `"3M"`)
    /// @param day_count - Optional day count name (e.g. `"act_360"`)
    /// @param spread - Optional spread (decimal) added to the forward rate
    /// @param business_day_convention - Optional BDC name (e.g. `"modified_following"`)
    /// @returns A `BasisSwapLeg`
    /// @throws {Error} If parsing fails
    #[wasm_bindgen(constructor)]
    pub fn new(
        forward_curve: &str,
        frequency: Option<String>,
        day_count: Option<String>,
        spread: Option<f64>,
        business_day_convention: Option<String>,
    ) -> Result<JsBasisSwapLeg, JsValue> {
        let freq = parse_optional_with_default(frequency, Tenor::quarterly())?;
        let dc = parse_optional_with_default(day_count, DayCount::Act360)?;
        let bdc = parse_optional_with_default(
            business_day_convention,
            BusinessDayConvention::ModifiedFollowing,
        )?;

        Ok(JsBasisSwapLeg {
            inner: BasisSwapLeg {
                forward_curve_id: curve_id_from_str(forward_curve),
                frequency: freq,
                day_count: dc,
                bdc,
                spread: spread.unwrap_or(0.0),
                payment_lag_days: 0,
                reset_lag_days: 0,
            },
        })
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn spread(&self) -> f64 {
        self.inner.spread
    }
}

#[wasm_bindgen(js_name = BasisSwap)]
#[derive(Clone, Debug)]
pub struct JsBasisSwap {
    pub(crate) inner: BasisSwap,
}

impl InstrumentWrapper for JsBasisSwap {
    type Inner = BasisSwap;
    fn from_inner(inner: BasisSwap) -> Self {
        JsBasisSwap { inner }
    }
    fn inner(&self) -> BasisSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = BasisSwap)]
impl JsBasisSwap {
    /// Create a float/float basis swap.
    ///
    /// Conventions:
    /// - The two legs reference forward curve IDs; discounting uses `discount_curve`.
    /// - Stub and calendar settings affect schedule generation.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Swap notional (currency-tagged)
    /// @param start_date - Swap start date
    /// @param maturity - Swap end/maturity date
    /// @param primary_leg - Primary floating leg specification
    /// @param reference_leg - Reference floating leg specification
    /// @param discount_curve - Discount curve ID
    /// @param calendar - Optional calendar code (e.g. `"usny"`)
    /// @param stub - Optional stub kind string
    /// @returns A new `BasisSwap`
    /// @throws {Error} If inputs are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { BasisSwap, BasisSwapLeg, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const leg3m = new BasisSwapLeg("USD-SOFR-3M", "3M", "act_360", 0.0, "modified_following");
    /// const leg1m = new BasisSwapLeg("USD-SOFR-1M", "1M", "act_360", 0.0, "modified_following");
    /// const swap = new BasisSwap(
    ///   "basis_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   leg3m,
    ///   leg1m,
    ///   "USD-OIS"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        start_date: &JsDate,
        maturity: &JsDate,
        primary_leg: &JsBasisSwapLeg,
        reference_leg: &JsBasisSwapLeg,
        discount_curve: &str,
        calendar: Option<String>,
        stub: Option<String>,
    ) -> Result<JsBasisSwap, JsValue> {
        let stub_kind = parse_optional_with_default(stub, StubKind::None)?;

        let builder = BasisSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .start_date(start_date.inner())
            .maturity_date(maturity.inner())
            .primary_leg(primary_leg.inner.clone())
            .reference_leg(reference_leg.inner.clone())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .stub_kind(stub_kind)
            .calendar_id_opt(calendar)
            .attributes(Default::default());

        builder
            .build()
            .map(JsBasisSwap::from_inner)
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

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::BasisSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("BasisSwap(id='{}')", self.inner.id)
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasisSwap {
        JsBasisSwap::from_inner(self.inner.clone())
    }
}
