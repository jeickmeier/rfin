//! WASM bindings for core phantom types (IDs, rates, credit ratings).

use crate::core::error::js_error;
use finstack_core::types::{CreditRating, NotchedRating, RatingNotch};
use finstack_core::types::{
    Bps as CoreBps, CurveId as CoreCurveId, IndexId as CoreIndexId,
    InstrumentId as CoreInstrumentId, Percentage as CorePercentage, PriceId as CorePriceId,
    Rate as CoreRate, UnderlyingId as CoreUnderlyingId,
};
use wasm_bindgen::prelude::*;

// ======================================================================
// ID wrappers
// ======================================================================

macro_rules! id_wrapper {
    ($js_name:ident, $inner:ty) => {
        #[wasm_bindgen(js_name = $js_name)]
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $js_name {
            inner: $inner,
        }

        #[wasm_bindgen(js_class = $js_name)]
        impl $js_name {
            #[wasm_bindgen(constructor)]
            pub fn new(id: &str) -> $js_name {
                Self {
                    inner: <$inner>::from(id),
                }
            }

            #[wasm_bindgen(getter, js_name = value)]
            pub fn value(&self) -> String {
                self.inner.as_str().to_string()
            }

            #[allow(clippy::inherent_to_string)]
            #[wasm_bindgen(js_name = toString)]
            pub fn to_string(&self) -> String {
                self.value()
            }

            #[allow(dead_code)]
            pub(crate) fn inner(&self) -> $inner {
                self.inner.clone()
            }
        }
    };
}

id_wrapper!(CurveId, CoreCurveId);
id_wrapper!(InstrumentId, CoreInstrumentId);
id_wrapper!(IndexId, CoreIndexId);
id_wrapper!(PriceId, CorePriceId);
id_wrapper!(UnderlyingId, CoreUnderlyingId);

// ======================================================================
// Rates
// ======================================================================

/// Financial rate stored as decimal (0.05 = 5%).
#[wasm_bindgen(js_name = Rate)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsRate {
    inner: CoreRate,
}

#[wasm_bindgen(js_class = Rate)]
impl JsRate {
    #[wasm_bindgen(js_name = fromDecimal)]
    pub fn from_decimal(decimal: f64) -> JsRate {
        JsRate {
            inner: CoreRate::from_decimal(decimal),
        }
    }

    #[wasm_bindgen(js_name = fromPercent)]
    pub fn from_percent(percent: f64) -> JsRate {
        JsRate {
            inner: CoreRate::from_percent(percent),
        }
    }

    #[wasm_bindgen(js_name = fromBps)]
    pub fn from_bps(bps: i32) -> JsRate {
        JsRate {
            inner: CoreRate::from_bps(bps),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[wasm_bindgen(getter)]
    pub fn percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[wasm_bindgen(getter)]
    pub fn bps(&self) -> i32 {
        self.inner.as_bps()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CoreRate {
        self.inner
    }
}

/// Basis points helper (1 bp = 0.01%).
#[wasm_bindgen(js_name = Bps)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsBps {
    inner: CoreBps,
}

#[wasm_bindgen(js_class = Bps)]
impl JsBps {
    #[wasm_bindgen(constructor)]
    pub fn new(bps: i32) -> JsBps {
        JsBps {
            inner: CoreBps::new(bps),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> i32 {
        self.inner.as_bps()
    }

    #[wasm_bindgen(js_name = asDecimal)]
    pub fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[wasm_bindgen(js_name = asPercent)]
    pub fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[wasm_bindgen(js_name = asRate)]
    pub fn as_rate(&self) -> JsRate {
        JsRate {
            inner: self.inner.as_rate(),
        }
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

/// Percentage helper (5.0 = 5%).
#[wasm_bindgen(js_name = Percentage)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsPercentage {
    inner: CorePercentage,
}

#[wasm_bindgen(js_class = Percentage)]
impl JsPercentage {
    #[wasm_bindgen(constructor)]
    pub fn new(percent: f64) -> JsPercentage {
        JsPercentage {
            inner: CorePercentage::new(percent),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[wasm_bindgen(js_name = asDecimal)]
    pub fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[wasm_bindgen(js_name = asRate)]
    pub fn as_rate(&self) -> JsRate {
        JsRate {
            inner: self.inner.as_rate(),
        }
    }

    #[wasm_bindgen(js_name = asBps)]
    pub fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

// ======================================================================
// Credit ratings
// ======================================================================

/// Rating notch (+ / flat / -) helper.
#[wasm_bindgen(js_name = RatingNotch)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JsRatingNotch {
    Plus,
    Flat,
    Minus,
}

impl From<JsRatingNotch> for RatingNotch {
    fn from(value: JsRatingNotch) -> Self {
        match value {
            JsRatingNotch::Plus => RatingNotch::Plus,
            JsRatingNotch::Flat => RatingNotch::Flat,
            JsRatingNotch::Minus => RatingNotch::Minus,
        }
    }
}

impl From<RatingNotch> for JsRatingNotch {
    fn from(value: RatingNotch) -> Self {
        match value {
            RatingNotch::Plus => JsRatingNotch::Plus,
            RatingNotch::Flat => JsRatingNotch::Flat,
            RatingNotch::Minus => JsRatingNotch::Minus,
        }
    }
}

/// Credit rating helper (S&P-style buckets).
#[wasm_bindgen(js_name = CreditRating)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JsCreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    CC,
    C,
    D,
    NR,
}

impl From<JsCreditRating> for CreditRating {
    fn from(value: JsCreditRating) -> Self {
        match value {
            JsCreditRating::AAA => CreditRating::AAA,
            JsCreditRating::AA => CreditRating::AA,
            JsCreditRating::A => CreditRating::A,
            JsCreditRating::BBB => CreditRating::BBB,
            JsCreditRating::BB => CreditRating::BB,
            JsCreditRating::B => CreditRating::B,
            JsCreditRating::CCC => CreditRating::CCC,
            JsCreditRating::CC => CreditRating::CC,
            JsCreditRating::C => CreditRating::C,
            JsCreditRating::D => CreditRating::D,
            JsCreditRating::NR => CreditRating::NR,
        }
    }
}

impl From<CreditRating> for JsCreditRating {
    fn from(value: CreditRating) -> Self {
        match value {
            CreditRating::AAA => JsCreditRating::AAA,
            CreditRating::AA => JsCreditRating::AA,
            CreditRating::A => JsCreditRating::A,
            CreditRating::BBB => JsCreditRating::BBB,
            CreditRating::BB => JsCreditRating::BB,
            CreditRating::B => JsCreditRating::B,
            CreditRating::CCC => JsCreditRating::CCC,
            CreditRating::CC => JsCreditRating::CC,
            CreditRating::C => JsCreditRating::C,
            CreditRating::D => JsCreditRating::D,
            CreditRating::NR => JsCreditRating::NR,
        }
    }
}

#[wasm_bindgen(js_name = NotchedRating)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JsNotchedRating {
    inner: NotchedRating,
}

#[wasm_bindgen(js_class = NotchedRating)]
impl JsNotchedRating {
    /// Create a notched rating from base rating and notch.
    #[wasm_bindgen(constructor)]
    pub fn new(base: JsCreditRating, notch: JsRatingNotch) -> JsNotchedRating {
        JsNotchedRating {
            inner: NotchedRating::new(base.into(), notch.into()),
        }
    }

    /// Parse a rating label (e.g., "AA-", "Baa1", "NR").
    #[wasm_bindgen(js_name = fromLabel)]
    pub fn from_label(label: &str) -> Result<JsNotchedRating, JsValue> {
        label
            .parse::<NotchedRating>()
            .map(|inner| JsNotchedRating { inner })
            .map_err(|e| js_error(format!("Invalid rating label '{label}': {e}")))
    }

    /// Rating symbol (e.g., "AA-").
    #[wasm_bindgen(getter)]
    pub fn symbol(&self) -> String {
        self.inner.to_string()
    }

    /// Base bucket without notch.
    #[wasm_bindgen(getter)]
    pub fn base(&self) -> JsCreditRating {
        JsCreditRating::from(self.inner.base())
    }

    /// Notch indicator.
    #[wasm_bindgen(getter)]
    pub fn notch(&self) -> JsRatingNotch {
        JsRatingNotch::from(self.inner.notch())
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> NotchedRating {
        self.inner
    }
}
