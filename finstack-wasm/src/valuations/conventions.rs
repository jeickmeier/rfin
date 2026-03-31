//! WASM bindings for market conventions registry.
//!
//! Provides access to standard market conventions for various instrument types.
//! Convention properties that are complex types (DayCount, Tenor, BusinessDayConvention)
//! are exposed as string codes for simplicity in JavaScript.

use crate::core::error::js_error;
use finstack_valuations::market::conventions::{
    ids::{
        BondConventionId, CdsConventionKey, CdsDocClause, FxConventionId, FxOptionConventionId,
        IndexId, InflationSwapConventionId, IrFutureContractId, OptionConventionId,
        SwaptionConventionId, XccyConventionId,
    },
    CdsConventions, ConventionRegistry, InflationSwapConventions, IrFutureConventions,
    OptionConventions, RateIndexConventions, RateIndexKind, SwaptionConventions,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

// =============================================================================
// CdsDocClause Enum
// =============================================================================

/// CDS documentation clause (ISDA standard).
#[wasm_bindgen(js_name = CdsDocClause)]
#[derive(Clone, Copy)]
pub struct JsCdsDocClause {
    inner: CdsDocClause,
}

#[wasm_bindgen(js_class = CdsDocClause)]
impl JsCdsDocClause {
    /// Cum-Restructuring 2014 (CR14)
    #[wasm_bindgen(getter, js_name = CR14)]
    pub fn cr14() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::Cr14,
        }
    }

    /// Modified-Restructuring 2014 (MR14)
    #[wasm_bindgen(getter, js_name = MR14)]
    pub fn mr14() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::Mr14,
        }
    }

    /// Modified-Modified-Restructuring 2014 (MM14)
    #[wasm_bindgen(getter, js_name = MM14)]
    pub fn mm14() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::Mm14,
        }
    }

    /// No-Restructuring 2014 (XR14)
    #[wasm_bindgen(getter, js_name = XR14)]
    pub fn xr14() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::Xr14,
        }
    }

    /// ISDA North American Corporate
    #[wasm_bindgen(getter, js_name = ISDA_NA)]
    pub fn isda_na() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::IsdaNa,
        }
    }

    /// ISDA European Corporate
    #[wasm_bindgen(getter, js_name = ISDA_EU)]
    pub fn isda_eu() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::IsdaEu,
        }
    }

    /// ISDA Asia Corporate
    #[wasm_bindgen(getter, js_name = ISDA_AS)]
    pub fn isda_as() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::IsdaAs,
        }
    }

    /// ISDA Australia Corporate
    #[wasm_bindgen(getter, js_name = ISDA_AU)]
    pub fn isda_au() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::IsdaAu,
        }
    }

    /// ISDA New Zealand Corporate
    #[wasm_bindgen(getter, js_name = ISDA_NZ)]
    pub fn isda_nz() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::IsdaNz,
        }
    }

    /// Custom / Other
    #[wasm_bindgen(getter, js_name = CUSTOM)]
    pub fn custom() -> JsCdsDocClause {
        JsCdsDocClause {
            inner: CdsDocClause::Custom,
        }
    }

    /// Parse a CDS doc clause from string.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsCdsDocClause, JsValue> {
        CdsDocClause::from_str(name)
            .map(|inner| JsCdsDocClause { inner })
            .map_err(js_error)
    }

    /// Get the name of the doc clause.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// =============================================================================
// RateIndexKind Enum
// =============================================================================

/// Type of rate index.
#[wasm_bindgen(js_name = RateIndexKind)]
#[derive(Clone, Copy)]
pub struct JsRateIndexKind {
    inner: RateIndexKind,
}

#[wasm_bindgen(js_class = RateIndexKind)]
impl JsRateIndexKind {
    /// Overnight Risk-Free Rate (e.g., SOFR, SONIA)
    #[wasm_bindgen(getter, js_name = OVERNIGHT_RFR)]
    pub fn overnight_rfr() -> JsRateIndexKind {
        JsRateIndexKind {
            inner: RateIndexKind::OvernightRfr,
        }
    }

    /// Term index (e.g., 3M LIBOR, 6M EURIBOR)
    #[wasm_bindgen(getter, js_name = TERM)]
    pub fn term() -> JsRateIndexKind {
        JsRateIndexKind {
            inner: RateIndexKind::Term,
        }
    }

    /// Get the name of the index kind.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        match self.inner {
            RateIndexKind::OvernightRfr => "OVERNIGHT_RFR".to_string(),
            RateIndexKind::Term => "TERM".to_string(),
        }
    }
}

// =============================================================================
// CdsConventionKey
// =============================================================================

/// Key for looking up CDS conventions (currency + doc clause).
#[wasm_bindgen(js_name = CdsConventionKey)]
pub struct JsCdsConventionKey {
    inner: CdsConventionKey,
}

#[wasm_bindgen(js_class = CdsConventionKey)]
impl JsCdsConventionKey {
    /// Create a new CDS convention key.
    #[wasm_bindgen(constructor)]
    pub fn new(currency: &str, doc_clause: JsCdsDocClause) -> Result<JsCdsConventionKey, JsValue> {
        let ccy = finstack_core::currency::Currency::from_str(currency)
            .map_err(|e| js_error(format!("Invalid currency: {}", e)))?;
        Ok(JsCdsConventionKey {
            inner: CdsConventionKey {
                currency: ccy,
                doc_clause: doc_clause.inner,
            },
        })
    }

    /// Get the currency code.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.currency.to_string()
    }

    /// Get the doc clause.
    #[wasm_bindgen(getter, js_name = docClause)]
    pub fn doc_clause(&self) -> JsCdsDocClause {
        JsCdsDocClause {
            inner: self.inner.doc_clause,
        }
    }
}

// =============================================================================
// RateIndexConventions
// =============================================================================

/// Conventions for rate index instruments.
#[wasm_bindgen(js_name = RateIndexConventions)]
pub struct JsRateIndexConventions {
    inner: RateIndexConventions,
}

#[wasm_bindgen(js_class = RateIndexConventions)]
impl JsRateIndexConventions {
    /// Currency code.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.currency.to_string()
    }

    /// Type of rate index (overnight RFR or term).
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> JsRateIndexKind {
        JsRateIndexKind {
            inner: self.inner.kind,
        }
    }

    /// Native tenor code (for term indices), or null for overnight.
    #[wasm_bindgen(getter)]
    pub fn tenor(&self) -> Option<String> {
        self.inner.tenor.map(|t| t.to_string())
    }

    /// Day count convention code.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    /// Default payment frequency code.
    #[wasm_bindgen(getter, js_name = defaultPaymentFrequency)]
    pub fn default_payment_frequency(&self) -> String {
        self.inner.default_payment_frequency.to_string()
    }

    /// Default payment delay in days.
    #[wasm_bindgen(getter, js_name = defaultPaymentDelayDays)]
    pub fn default_payment_lag_days(&self) -> i32 {
        self.inner.default_payment_lag_days
    }

    /// Default reset lag in days.
    #[wasm_bindgen(getter, js_name = defaultResetLagDays)]
    pub fn default_reset_lag_days(&self) -> i32 {
        self.inner.default_reset_lag_days
    }

    /// Market calendar identifier.
    #[wasm_bindgen(getter, js_name = marketCalendarId)]
    pub fn market_calendar_id(&self) -> String {
        self.inner.market_calendar_id.clone()
    }

    /// Market settlement days.
    #[wasm_bindgen(getter, js_name = marketSettlementDays)]
    pub fn market_settlement_days(&self) -> i32 {
        self.inner.market_settlement_days
    }

    /// Market business day convention code.
    #[wasm_bindgen(getter, js_name = marketBusinessDayConvention)]
    pub fn market_business_day_convention(&self) -> String {
        format!("{:?}", self.inner.market_business_day_convention)
    }

    /// Default fixed leg day count code.
    #[wasm_bindgen(getter, js_name = defaultFixedLegDayCount)]
    pub fn default_fixed_leg_day_count(&self) -> String {
        format!("{:?}", self.inner.default_fixed_leg_day_count)
    }

    /// Default fixed leg frequency code.
    #[wasm_bindgen(getter, js_name = defaultFixedLegFrequency)]
    pub fn default_fixed_leg_frequency(&self) -> String {
        self.inner.default_fixed_leg_frequency.to_string()
    }
}

// =============================================================================
// CdsConventions
// =============================================================================

/// Conventions for Credit Default Swaps.
#[wasm_bindgen(js_name = CdsConventions)]
pub struct JsCdsConventions {
    inner: CdsConventions,
}

#[wasm_bindgen(js_class = CdsConventions)]
impl JsCdsConventions {
    /// Calendar identifier.
    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Day count convention code.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    /// Business day convention code.
    #[wasm_bindgen(getter, js_name = businessDayConvention)]
    pub fn business_day_convention(&self) -> String {
        format!("{:?}", self.inner.bdc)
    }

    /// Settlement days.
    #[wasm_bindgen(getter, js_name = settlementDays)]
    pub fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    /// Payment frequency code.
    #[wasm_bindgen(getter, js_name = paymentFrequency)]
    pub fn payment_frequency(&self) -> String {
        self.inner.frequency.to_string()
    }
}

// =============================================================================
// SwaptionConventions
// =============================================================================

/// Conventions for Swaptions.
#[wasm_bindgen(js_name = SwaptionConventions)]
pub struct JsSwaptionConventions {
    inner: SwaptionConventions,
}

#[wasm_bindgen(js_class = SwaptionConventions)]
impl JsSwaptionConventions {
    /// Calendar identifier.
    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Settlement days.
    #[wasm_bindgen(getter, js_name = settlementDays)]
    pub fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    /// Business day convention code.
    #[wasm_bindgen(getter, js_name = businessDayConvention)]
    pub fn business_day_convention(&self) -> String {
        format!("{:?}", self.inner.business_day_convention)
    }

    /// Fixed leg frequency code.
    #[wasm_bindgen(getter, js_name = fixedLegFrequency)]
    pub fn fixed_leg_frequency(&self) -> String {
        self.inner.fixed_leg_frequency.to_string()
    }

    /// Fixed leg day count code.
    #[wasm_bindgen(getter, js_name = fixedLegDayCount)]
    pub fn fixed_leg_day_count(&self) -> String {
        format!("{:?}", self.inner.fixed_leg_day_count)
    }

    /// Float leg index ID.
    #[wasm_bindgen(getter, js_name = floatLegIndex)]
    pub fn float_leg_index(&self) -> String {
        self.inner.float_leg_index.clone()
    }
}

// =============================================================================
// InflationSwapConventions
// =============================================================================

/// Conventions for Inflation Swaps.
#[wasm_bindgen(js_name = InflationSwapConventions)]
pub struct JsInflationSwapConventions {
    inner: InflationSwapConventions,
}

#[wasm_bindgen(js_class = InflationSwapConventions)]
impl JsInflationSwapConventions {
    /// Calendar identifier.
    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Settlement days.
    #[wasm_bindgen(getter, js_name = settlementDays)]
    pub fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    /// Business day convention code.
    #[wasm_bindgen(getter, js_name = businessDayConvention)]
    pub fn business_day_convention(&self) -> String {
        format!("{:?}", self.inner.business_day_convention)
    }

    /// Day count convention code.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    /// Inflation lag (observation delay) code.
    #[wasm_bindgen(getter, js_name = inflationLag)]
    pub fn inflation_lag(&self) -> String {
        self.inner.inflation_lag.to_string()
    }
}

// =============================================================================
// OptionConventions
// =============================================================================

/// Conventions for Options (Equity/FX/Commodity).
#[wasm_bindgen(js_name = OptionConventions)]
pub struct JsOptionConventions {
    inner: OptionConventions,
}

#[wasm_bindgen(js_class = OptionConventions)]
impl JsOptionConventions {
    /// Calendar identifier.
    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Settlement days.
    #[wasm_bindgen(getter, js_name = settlementDays)]
    pub fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    /// Business day convention code.
    #[wasm_bindgen(getter, js_name = businessDayConvention)]
    pub fn business_day_convention(&self) -> String {
        format!("{:?}", self.inner.business_day_convention)
    }
}

// =============================================================================
// IrFutureConventions
// =============================================================================

/// Conventions for Interest Rate Futures.
#[wasm_bindgen(js_name = IrFutureConventions)]
pub struct JsIrFutureConventions {
    inner: IrFutureConventions,
}

#[wasm_bindgen(js_class = IrFutureConventions)]
impl JsIrFutureConventions {
    /// Rate index identifier.
    #[wasm_bindgen(getter, js_name = indexId)]
    pub fn index_id(&self) -> String {
        self.inner.index_id.as_str().to_string()
    }

    /// Calendar identifier.
    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Settlement days.
    #[wasm_bindgen(getter, js_name = settlementDays)]
    pub fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    /// Delivery months.
    #[wasm_bindgen(getter, js_name = deliveryMonths)]
    pub fn delivery_months(&self) -> u8 {
        self.inner.delivery_months
    }

    /// Face value of the contract.
    #[wasm_bindgen(getter, js_name = faceValue)]
    pub fn face_value(&self) -> f64 {
        self.inner.face_value
    }

    /// Minimum price movement.
    #[wasm_bindgen(getter, js_name = tickSize)]
    pub fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    /// Dollar value of one tick.
    #[wasm_bindgen(getter, js_name = tickValue)]
    pub fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    /// Default convexity adjustment (if any).
    #[wasm_bindgen(getter, js_name = convexityAdjustment)]
    pub fn convexity_adjustment(&self) -> Option<f64> {
        self.inner.convexity_adjustment
    }
}

// =============================================================================
// ConventionRegistry
// =============================================================================

/// Global registry of market conventions.
#[wasm_bindgen(js_name = ConventionRegistry)]
pub struct JsConventionRegistry;

#[wasm_bindgen(js_class = ConventionRegistry)]
impl JsConventionRegistry {
    /// Get the global convention registry instance.
    #[wasm_bindgen(js_name = globalInstance)]
    pub fn global_instance() -> Result<JsConventionRegistry, JsValue> {
        Ok(JsConventionRegistry)
    }

    /// Look up conventions for a rate index.
    #[wasm_bindgen(js_name = requireRateIndex)]
    pub fn require_rate_index(&self, index_id: &str) -> Result<JsRateIndexConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_rate_index(&IndexId::new(index_id))
            .map(|conv| JsRateIndexConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Look up conventions for a CDS.
    #[wasm_bindgen(js_name = requireCds)]
    pub fn require_cds(&self, key: &JsCdsConventionKey) -> Result<JsCdsConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_cds(&key.inner)
            .map(|conv| JsCdsConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Look up conventions for a swaption.
    #[wasm_bindgen(js_name = requireSwaption)]
    pub fn require_swaption(&self, convention_id: &str) -> Result<JsSwaptionConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_swaption(&SwaptionConventionId::new(convention_id))
            .map(|conv| JsSwaptionConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Look up conventions for an inflation swap.
    #[wasm_bindgen(js_name = requireInflationSwap)]
    pub fn require_inflation_swap(
        &self,
        convention_id: &str,
    ) -> Result<JsInflationSwapConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_inflation_swap(&InflationSwapConventionId::new(convention_id))
            .map(|conv| JsInflationSwapConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Look up conventions for an option.
    #[wasm_bindgen(js_name = requireOption)]
    pub fn require_option(&self, convention_id: &str) -> Result<JsOptionConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_option(&OptionConventionId::new(convention_id))
            .map(|conv| JsOptionConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Look up conventions for an IR future contract.
    #[wasm_bindgen(js_name = requireIrFuture)]
    pub fn require_ir_future(&self, contract_id: &str) -> Result<JsIrFutureConventions, JsValue> {
        ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_ir_future(&IrFutureContractId::new(contract_id))
            .map(|conv| JsIrFutureConventions {
                inner: conv.clone(),
            })
            .map_err(|e| js_error(e.to_string()))
    }

    // -------------------------------------------------------------------------
    // Existence check helpers
    // -------------------------------------------------------------------------

    /// Check if a rate index exists in the registry.
    #[wasm_bindgen(js_name = hasRateIndex)]
    pub fn has_rate_index(&self, index_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| r.require_rate_index(&IndexId::new(index_id)).is_ok())
            .unwrap_or(false)
    }

    /// Check if a CDS convention exists in the registry.
    #[wasm_bindgen(js_name = hasCds)]
    pub fn has_cds(&self, key: &JsCdsConventionKey) -> bool {
        ConventionRegistry::try_global()
            .map(|r| r.require_cds(&key.inner).is_ok())
            .unwrap_or(false)
    }

    /// Check if a swaption convention exists in the registry.
    #[wasm_bindgen(js_name = hasSwaption)]
    pub fn has_swaption(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_swaption(&SwaptionConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    /// Check if an inflation swap convention exists in the registry.
    #[wasm_bindgen(js_name = hasInflationSwap)]
    pub fn has_inflation_swap(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_inflation_swap(&InflationSwapConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    /// Check if an option convention exists in the registry.
    #[wasm_bindgen(js_name = hasOption)]
    pub fn has_option(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_option(&OptionConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    /// Check if an IR future convention exists in the registry.
    #[wasm_bindgen(js_name = hasIrFuture)]
    pub fn has_ir_future(&self, contract_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_ir_future(&IrFutureContractId::new(contract_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------------
    // Bond conventions
    // -------------------------------------------------------------------------

    /// Look up conventions for a bond.
    #[wasm_bindgen(js_name = requireBond)]
    pub fn require_bond(&self, convention_id: &str) -> Result<JsValue, JsValue> {
        let conv = ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_bond(&BondConventionId::new(convention_id))
            .map_err(|e| js_error(e.to_string()))?;
        serde_wasm_bindgen::to_value(conv)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Check if a bond convention exists in the registry.
    #[wasm_bindgen(js_name = hasBond)]
    pub fn has_bond(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_bond(&BondConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------------
    // FX conventions
    // -------------------------------------------------------------------------

    /// Look up conventions for an FX pair.
    #[wasm_bindgen(js_name = requireFx)]
    pub fn require_fx(&self, convention_id: &str) -> Result<JsValue, JsValue> {
        let conv = ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_fx(&FxConventionId::new(convention_id))
            .map_err(|e| js_error(e.to_string()))?;
        serde_wasm_bindgen::to_value(conv)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Check if an FX convention exists.
    #[wasm_bindgen(js_name = hasFx)]
    pub fn has_fx(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_fx(&FxConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------------
    // FX option conventions
    // -------------------------------------------------------------------------

    /// Look up conventions for an FX option.
    #[wasm_bindgen(js_name = requireFxOption)]
    pub fn require_fx_option(&self, convention_id: &str) -> Result<JsValue, JsValue> {
        let conv = ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_fx_option(&FxOptionConventionId::new(convention_id))
            .map_err(|e| js_error(e.to_string()))?;
        serde_wasm_bindgen::to_value(conv)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Check if an FX option convention exists.
    #[wasm_bindgen(js_name = hasFxOption)]
    pub fn has_fx_option(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_fx_option(&FxOptionConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------------
    // Cross-currency swap conventions
    // -------------------------------------------------------------------------

    /// Look up conventions for a cross-currency swap.
    #[wasm_bindgen(js_name = requireXccy)]
    pub fn require_xccy(&self, convention_id: &str) -> Result<JsValue, JsValue> {
        let conv = ConventionRegistry::try_global()
            .map_err(|e| js_error(e.to_string()))?
            .require_xccy(&XccyConventionId::new(convention_id))
            .map_err(|e| js_error(e.to_string()))?;
        serde_wasm_bindgen::to_value(conv)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Check if a cross-currency swap convention exists.
    #[wasm_bindgen(js_name = hasXccy)]
    pub fn has_xccy(&self, convention_id: &str) -> bool {
        ConventionRegistry::try_global()
            .map(|r| {
                r.require_xccy(&XccyConventionId::new(convention_id))
                    .is_ok()
            })
            .unwrap_or(false)
    }
}
