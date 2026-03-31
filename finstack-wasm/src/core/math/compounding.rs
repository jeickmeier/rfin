use crate::core::error::js_error;
use finstack_core::math::compounding::Compounding as CoreCompounding;
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;

/// Compounding convention for interest rate calculations.
///
/// Different markets use different compounding conventions:
/// - **Continuous**: Internal calculations, curve construction
/// - **Annual**: Bond markets (UK, Europe)
/// - **SemiAnnual**: US Treasury, corporate bonds
/// - **Quarterly**: Some floating rate notes
/// - **Monthly**: Retail products
/// - **Simple**: Money market (< 1Y), deposits
///
/// @example
/// ```javascript
/// const cont = Compounding.Continuous();
/// const semi = Compounding.SemiAnnual();
///
/// // Discount factor for 5% rate at 1 year
/// const df = cont.dfFromRate(0.05, 1.0);
///
/// // Convert rate between conventions
/// const semiRate = cont.convertRate(0.05, 1.0, semi);
/// ```
#[wasm_bindgen(js_name = MathCompounding)]
#[derive(Clone, Copy, Debug)]
pub struct JsCompounding {
    inner: CoreCompounding,
}

impl JsCompounding {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CoreCompounding {
        self.inner
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CoreCompounding) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = MathCompounding)]
impl JsCompounding {
    /// Continuous compounding: DF = exp(-r × t).
    #[wasm_bindgen(js_name = Continuous)]
    pub fn continuous() -> JsCompounding {
        Self {
            inner: CoreCompounding::Continuous,
        }
    }

    /// Annual compounding: DF = (1 + r)^(-t).
    #[wasm_bindgen(js_name = Annual)]
    pub fn annual() -> JsCompounding {
        Self {
            inner: CoreCompounding::Annual,
        }
    }

    /// Semi-annual compounding (n=2): standard for US Treasury bonds.
    #[wasm_bindgen(js_name = SemiAnnual)]
    pub fn semi_annual() -> JsCompounding {
        Self {
            inner: CoreCompounding::SEMI_ANNUAL,
        }
    }

    /// Quarterly compounding (n=4).
    #[wasm_bindgen(js_name = Quarterly)]
    pub fn quarterly() -> JsCompounding {
        Self {
            inner: CoreCompounding::QUARTERLY,
        }
    }

    /// Monthly compounding (n=12).
    #[wasm_bindgen(js_name = Monthly)]
    pub fn monthly() -> JsCompounding {
        Self {
            inner: CoreCompounding::MONTHLY,
        }
    }

    /// Simple interest (no compounding): DF = 1 / (1 + r × t).
    #[wasm_bindgen(js_name = Simple)]
    pub fn simple() -> JsCompounding {
        Self {
            inner: CoreCompounding::Simple,
        }
    }

    /// Periodic compounding with n periods per year.
    ///
    /// @param {number} n - Periods per year (must be > 0)
    #[wasm_bindgen(js_name = Periodic)]
    pub fn periodic(n: u32) -> Result<JsCompounding, JsValue> {
        let nz = NonZeroU32::new(n)
            .ok_or_else(|| js_error("Periods per year must be greater than zero"))?;
        Ok(Self {
            inner: CoreCompounding::Periodic(nz),
        })
    }

    /// Parse a compounding convention from a string name.
    ///
    /// @param {string} name - One of: "continuous", "annual", "semi_annual", "quarterly", "monthly", "simple"
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsCompounding, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "continuous" => Ok(Self::continuous()),
            "annual" => Ok(Self::annual()),
            "semi_annual" | "semiannual" => Ok(Self::semi_annual()),
            "quarterly" => Ok(Self::quarterly()),
            "monthly" => Ok(Self::monthly()),
            "simple" => Ok(Self::simple()),
            other => Err(js_error(format!("Unknown compounding convention: {other}"))),
        }
    }

    /// Convert an interest rate to a discount factor for time `t` (years).
    ///
    /// @param {number} rate - Interest rate (decimal, e.g. 0.05 for 5%)
    /// @param {number} t - Time in years
    /// @returns {number} Discount factor
    #[wasm_bindgen(js_name = dfFromRate)]
    pub fn df_from_rate(&self, rate: f64, t: f64) -> f64 {
        self.inner.df_from_rate(rate, t)
    }

    /// Convert a discount factor to an interest rate for time `t` (years).
    ///
    /// @param {number} df - Discount factor
    /// @param {number} t - Time in years
    /// @returns {number} Interest rate (decimal)
    #[wasm_bindgen(js_name = rateFromDf)]
    pub fn rate_from_df(&self, df: f64, t: f64) -> f64 {
        self.inner.rate_from_df(df, t)
    }

    /// Fallible version of rateFromDf that returns an error for degenerate inputs.
    ///
    /// @param {number} df - Discount factor (must be positive and finite)
    /// @param {number} t - Time in years (must be finite)
    /// @returns {number} Interest rate (decimal)
    #[wasm_bindgen(js_name = tryRateFromDf)]
    pub fn try_rate_from_df(&self, df: f64, t: f64) -> Result<f64, JsValue> {
        self.inner
            .try_rate_from_df(df, t)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Convert a rate quoted under this convention to another convention.
    ///
    /// @param {number} rate - Rate under this convention
    /// @param {number} t - Time horizon in years
    /// @param {MathCompounding} to - Target compounding convention
    /// @returns {number} Equivalent rate under the target convention
    #[wasm_bindgen(js_name = convertRate)]
    pub fn convert_rate(&self, rate: f64, t: f64, to: &JsCompounding) -> f64 {
        self.inner.convert_rate(rate, t, &to.inner)
    }

    /// Number of compounding periods per year, if applicable.
    ///
    /// Returns null for Continuous and Simple conventions.
    #[wasm_bindgen(js_name = periodsPerYear)]
    pub fn periods_per_year(&self) -> Option<u32> {
        self.inner.periods_per_year()
    }

    /// Whether this is a periodic compounding convention (including Annual).
    #[wasm_bindgen(getter, js_name = isPeriodic)]
    pub fn is_periodic(&self) -> bool {
        self.inner.is_periodic()
    }

    /// String representation of the compounding convention.
    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}
