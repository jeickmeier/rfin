//! Enum type bindings for scenarios.

use finstack_scenarios::{CurveKind, TenorMatchMode, VolSurfaceKind};
use wasm_bindgen::prelude::*;

/// Identifies which family of curve an operation targets.
///
/// Maps to the market data collections exposed by MarketContext.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsCurveKind {
    pub(crate) inner: CurveKind,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsCurveKind {
    /// Discount factor curve.
    #[wasm_bindgen(getter)]
    pub fn DISCOUNT() -> JsCurveKind {
        JsCurveKind {
            inner: CurveKind::Discount,
        }
    }

    /// Forward rate curve.
    #[wasm_bindgen(getter)]
    pub fn FORECAST() -> JsCurveKind {
        JsCurveKind {
            inner: CurveKind::Forecast,
        }
    }

    /// Credit hazard rate curve.
    #[wasm_bindgen(getter)]
    pub fn HAZARD() -> JsCurveKind {
        JsCurveKind {
            inner: CurveKind::Hazard,
        }
    }

    /// Inflation index curve.
    #[wasm_bindgen(getter)]
    pub fn INFLATION() -> JsCurveKind {
        JsCurveKind {
            inner: CurveKind::Inflation,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<CurveKind> for JsCurveKind {
    fn from(inner: CurveKind) -> Self {
        Self { inner }
    }
}

impl From<JsCurveKind> for CurveKind {
    fn from(js: JsCurveKind) -> Self {
        js.inner
    }
}

/// Identifies which category of volatility surface an operation targets.
///
/// Drives lookups into the relevant vol collections in market data.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsVolSurfaceKind {
    pub(crate) inner: VolSurfaceKind,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsVolSurfaceKind {
    /// Equity volatility surface.
    #[wasm_bindgen(getter)]
    pub fn EQUITY() -> JsVolSurfaceKind {
        JsVolSurfaceKind {
            inner: VolSurfaceKind::Equity,
        }
    }

    /// Credit volatility surface.
    #[wasm_bindgen(getter)]
    pub fn CREDIT() -> JsVolSurfaceKind {
        JsVolSurfaceKind {
            inner: VolSurfaceKind::Credit,
        }
    }

    /// Swaption volatility surface.
    #[wasm_bindgen(getter)]
    pub fn SWAPTION() -> JsVolSurfaceKind {
        JsVolSurfaceKind {
            inner: VolSurfaceKind::Swaption,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<VolSurfaceKind> for JsVolSurfaceKind {
    fn from(inner: VolSurfaceKind) -> Self {
        Self { inner }
    }
}

impl From<JsVolSurfaceKind> for VolSurfaceKind {
    fn from(js: JsVolSurfaceKind) -> Self {
        js.inner
    }
}

/// Strategy for aligning requested tenor bumps with curve pillars.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsTenorMatchMode {
    pub(crate) inner: TenorMatchMode,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsTenorMatchMode {
    /// Match exact pillar only (errors if not found).
    #[wasm_bindgen(getter)]
    pub fn EXACT() -> JsTenorMatchMode {
        JsTenorMatchMode {
            inner: TenorMatchMode::Exact,
        }
    }

    /// Use key-rate bump at interpolated time.
    #[wasm_bindgen(getter)]
    pub fn INTERPOLATE() -> JsTenorMatchMode {
        JsTenorMatchMode {
            inner: TenorMatchMode::Interpolate,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<TenorMatchMode> for JsTenorMatchMode {
    fn from(inner: TenorMatchMode) -> Self {
        Self { inner }
    }
}

impl From<JsTenorMatchMode> for TenorMatchMode {
    fn from(js: JsTenorMatchMode) -> Self {
        js.inner
    }
}

