//! WASM bindings for core phantom types (IDs, rates, credit ratings, attributes).

use crate::core::error::js_error;
use finstack_core::types::{
    Attributes as CoreAttributes, CreditRating, NotchedRating, RatingLabel as CoreRatingLabel,
    RatingNotch,
};
use finstack_core::types::{
    Bps as CoreBps, CalendarId as CoreCalendarId, CurveId as CoreCurveId, DealId as CoreDealId,
    IndexId as CoreIndexId, InstrumentId as CoreInstrumentId, Percentage as CorePercentage,
    PoolId as CorePoolId, PriceId as CorePriceId, Rate as CoreRate,
    UnderlyingId as CoreUnderlyingId,
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
id_wrapper!(PoolId, CorePoolId);
id_wrapper!(DealId, CoreDealId);
id_wrapper!(CalendarId, CoreCalendarId);

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

// ======================================================================
// Rating Label
// ======================================================================

/// Stable, display-ready rating label sourced from a NotchedRating.
#[wasm_bindgen(js_name = RatingLabel)]
#[derive(Clone, Debug)]
pub struct JsRatingLabel {
    inner: CoreRatingLabel,
}

#[wasm_bindgen(js_class = RatingLabel)]
impl JsRatingLabel {
    /// Create a rating label from a notched rating string (e.g. "AA-", "Baa1").
    #[wasm_bindgen(constructor)]
    pub fn new(label: &str) -> Result<JsRatingLabel, JsValue> {
        label
            .parse::<NotchedRating>()
            .map(|nr| JsRatingLabel {
                inner: CoreRatingLabel::from(nr),
            })
            .map_err(|e| js_error(format!("Invalid rating label: {e}")))
    }

    /// The display string for this label.
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

// ======================================================================
// Rating Factor Table
// ======================================================================

/// Get the Moody's WARF (Weighted Average Rating Factor) for a credit rating.
///
/// @param {CreditRating} rating - Credit rating bucket
/// @returns {number} WARF factor
#[wasm_bindgen(js_name = moodysWarfFactor)]
pub fn moodys_warf_factor_js(rating: JsCreditRating) -> Result<f64, JsValue> {
    let core_rating: CreditRating = rating.into();
    finstack_core::types::moodys_warf_factor(core_rating).map_err(|e| js_error(format!("{e}")))
}

// ======================================================================
// Attributes
// ======================================================================

/// User-defined tags and key-value metadata for instrument classification.
///
/// @example
/// ```javascript
/// const attrs = new Attributes();
/// attrs.addTag("energy");
/// attrs.setMeta("region", "NA");
/// console.log(attrs.hasTag("energy"));  // true
/// console.log(attrs.getMeta("region")); // "NA"
/// ```
#[wasm_bindgen(js_name = Attributes)]
#[derive(Clone, Debug)]
pub struct JsAttributes {
    inner: CoreAttributes,
}

impl JsAttributes {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &CoreAttributes {
        &self.inner
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CoreAttributes) -> Self {
        Self { inner }
    }
}

impl Default for JsAttributes {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = Attributes)]
impl JsAttributes {
    /// Create an empty set of attributes.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsAttributes {
        JsAttributes {
            inner: CoreAttributes::new(),
        }
    }

    /// Add a tag to the attribute set.
    #[wasm_bindgen(js_name = addTag)]
    pub fn add_tag(&mut self, tag: &str) {
        self.inner.tags.insert(tag.to_string());
    }

    /// Check if a tag is present.
    #[wasm_bindgen(js_name = hasTag)]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.inner.has_tag(tag)
    }

    /// Set a metadata key-value pair.
    #[wasm_bindgen(js_name = setMeta)]
    pub fn set_meta(&mut self, key: &str, value: &str) {
        self.inner.set(key, value);
    }

    /// Get a metadata value by key.
    #[wasm_bindgen(js_name = getMeta)]
    pub fn get_meta(&self, key: &str) -> Option<String> {
        self.inner.get_meta(key).map(|s| s.to_string())
    }

    /// Match against a selector string (e.g. "tag:energy", "meta:region=NA", "*").
    #[wasm_bindgen(js_name = matchesSelector)]
    pub fn matches_selector(&self, selector: &str) -> bool {
        self.inner.matches_selector(selector)
    }

    /// Get all tags as a string array.
    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Vec<String> {
        self.inner.tags.iter().cloned().collect()
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize attributes: {e}")))
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsAttributes, JsValue> {
        let inner: CoreAttributes = serde_wasm_bindgen::from_value(value)
            .map_err(|e| js_error(format!("Failed to deserialize attributes: {e}")))?;
        Ok(JsAttributes { inner })
    }
}
