//! WASM bindings for real-estate financial statement templates.
//!
//! Wraps `finstack_statements_analytics::templates::real_estate` types for
//! constructing NOI/NCF buildups, rent roll projections, and property
//! operating statements.

use crate::core::error::js_error;
use finstack_core::dates::PeriodId;
use finstack_statements_analytics::templates::real_estate::{
    FreeRentWindowSpec, LeaseSpec, ManagementFeeBase, ManagementFeeSpec, PropertyTemplateNodes,
    RenewalSpec, RentRollOutputNodes, RentStepSpec, SimpleLeaseSpec,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

fn parse_pid(s: &str) -> Result<PeriodId, JsValue> {
    PeriodId::from_str(s).map_err(|e| js_error(format!("Invalid period ID '{s}': {e}")))
}

/// Lease specification for simple rent-roll modelling (v1).
#[wasm_bindgen(js_name = SimpleLeaseSpec)]
pub struct JsSimpleLeaseSpec {
    pub(crate) inner: SimpleLeaseSpec,
}

#[wasm_bindgen(js_class = SimpleLeaseSpec)]
impl JsSimpleLeaseSpec {
    /// Create a new lease specification.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier for this lease in the model
    /// * `start` - Period when the lease begins (e.g. "2025Q1")
    /// * `base_rent` - Starting annual rent
    /// * `growth_rate` - Per-period rent escalation rate (default 0.0)
    /// * `end` - Period when the lease ends (null = model horizon)
    /// * `free_rent_periods` - Number of initial periods with zero rent (default 0)
    /// * `occupancy` - Occupancy rate 0 to 1 (default 1.0)
    #[wasm_bindgen(constructor)]
    pub fn new(
        node_id: String,
        start: &str,
        base_rent: f64,
        growth_rate: Option<f64>,
        end: Option<String>,
        free_rent_periods: Option<u32>,
        occupancy: Option<f64>,
    ) -> Result<JsSimpleLeaseSpec, JsValue> {
        let start_pid = parse_pid(start)?;
        let end_pid = end.as_deref().map(parse_pid).transpose()?;
        Ok(JsSimpleLeaseSpec {
            inner: SimpleLeaseSpec {
                node_id,
                start: start_pid,
                end: end_pid,
                base_rent,
                growth_rate: growth_rate.unwrap_or(0.0),
                free_rent_periods: free_rent_periods.unwrap_or(0),
                occupancy: occupancy.unwrap_or(1.0),
            },
        })
    }

    /// Node identifier.
    #[wasm_bindgen(getter, js_name = nodeId)]
    pub fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    /// Start period string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// Starting annual rent.
    #[wasm_bindgen(getter, js_name = baseRent)]
    pub fn base_rent(&self) -> f64 {
        self.inner.base_rent
    }

    /// Per-period rent escalation rate.
    #[wasm_bindgen(getter, js_name = growthRate)]
    pub fn growth_rate(&self) -> f64 {
        self.inner.growth_rate
    }

    /// Occupancy rate.
    #[wasm_bindgen(getter)]
    pub fn occupancy(&self) -> f64 {
        self.inner.occupancy
    }

    /// Validate the lease specification.
    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| js_error(e.to_string()))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "SimpleLeaseSpec(nodeId='{}', start={}, baseRent={:.2})",
            self.inner.node_id, self.inner.start, self.inner.base_rent
        )
    }
}

/// Specifies a discrete rent step (absolute rent override at a given period).
#[wasm_bindgen(js_name = RentStepSpec)]
pub struct JsRentStepSpec {
    pub(crate) inner: RentStepSpec,
}

#[wasm_bindgen(js_class = RentStepSpec)]
impl JsRentStepSpec {
    /// Create a new rent step.
    ///
    /// # Arguments
    /// * `start` - Period when the step takes effect (e.g. "2025Q2")
    /// * `rent` - Absolute rent amount from this period onward
    #[wasm_bindgen(constructor)]
    pub fn new(start: &str, rent: f64) -> Result<JsRentStepSpec, JsValue> {
        Ok(JsRentStepSpec {
            inner: RentStepSpec {
                start: parse_pid(start)?,
                rent,
            },
        })
    }

    /// Start period string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// Rent amount.
    #[wasm_bindgen(getter)]
    pub fn rent(&self) -> f64 {
        self.inner.rent
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "RentStepSpec(start={}, rent={:.2})",
            self.inner.start, self.inner.rent
        )
    }
}

/// Free-rent window specification.
#[wasm_bindgen(js_name = FreeRentWindowSpec)]
pub struct JsFreeRentWindowSpec {
    pub(crate) inner: FreeRentWindowSpec,
}

#[wasm_bindgen(js_class = FreeRentWindowSpec)]
impl JsFreeRentWindowSpec {
    /// Create a free-rent window.
    ///
    /// # Arguments
    /// * `start` - Period when the free-rent window begins (e.g. "2025Q1")
    /// * `periods` - Number of consecutive free-rent periods
    #[wasm_bindgen(constructor)]
    pub fn new(start: &str, periods: u32) -> Result<JsFreeRentWindowSpec, JsValue> {
        Ok(JsFreeRentWindowSpec {
            inner: FreeRentWindowSpec {
                start: parse_pid(start)?,
                periods,
            },
        })
    }

    /// Start period string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// Number of free-rent periods.
    #[wasm_bindgen(getter)]
    pub fn periods(&self) -> u32 {
        self.inner.periods
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FreeRentWindowSpec(start={}, periods={})",
            self.inner.start, self.inner.periods
        )
    }
}

/// Lease renewal assumption.
#[wasm_bindgen(js_name = RenewalSpec)]
pub struct JsRenewalSpec {
    pub(crate) inner: RenewalSpec,
}

#[wasm_bindgen(js_class = RenewalSpec)]
impl JsRenewalSpec {
    /// Create a renewal specification.
    ///
    /// # Arguments
    /// * `downtime_periods` - Vacancy periods between expiry and renewal (default 0)
    /// * `term_periods` - Duration of renewed lease in periods (default 12)
    /// * `probability` - Renewal probability 0 to 1 (default 1.0)
    /// * `rent_factor` - Multiplier applied to previous rent at renewal (default 1.0)
    /// * `free_rent_periods` - Free rent at renewal start (default 0)
    #[wasm_bindgen(constructor)]
    pub fn new(
        downtime_periods: Option<u32>,
        term_periods: Option<u32>,
        probability: Option<f64>,
        rent_factor: Option<f64>,
        free_rent_periods: Option<u32>,
    ) -> JsRenewalSpec {
        JsRenewalSpec {
            inner: RenewalSpec {
                downtime_periods: downtime_periods.unwrap_or(0),
                term_periods: term_periods.unwrap_or(12),
                probability: probability.unwrap_or(1.0),
                rent_factor: rent_factor.unwrap_or(1.0),
                free_rent_periods: free_rent_periods.unwrap_or(0),
            },
        }
    }

    /// Vacancy periods between expiry and renewal.
    #[wasm_bindgen(getter, js_name = downtimePeriods)]
    pub fn downtime_periods(&self) -> u32 {
        self.inner.downtime_periods
    }

    /// Duration of renewed lease in periods.
    #[wasm_bindgen(getter, js_name = termPeriods)]
    pub fn term_periods(&self) -> u32 {
        self.inner.term_periods
    }

    /// Renewal probability.
    #[wasm_bindgen(getter)]
    pub fn probability(&self) -> f64 {
        self.inner.probability
    }

    /// Rent factor at renewal.
    #[wasm_bindgen(getter, js_name = rentFactor)]
    pub fn rent_factor(&self) -> f64 {
        self.inner.rent_factor
    }

    /// Validate the renewal specification.
    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| js_error(e.to_string()))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "RenewalSpec(downtime={}, term={}, prob={:.2}, factor={:.2})",
            self.inner.downtime_periods,
            self.inner.term_periods,
            self.inner.probability,
            self.inner.rent_factor
        )
    }
}

/// Growth convention for lease escalation.
#[wasm_bindgen(js_name = LeaseGrowthConvention)]
pub enum JsLeaseGrowthConvention {
    /// Growth applied each model period.
    PerPeriod = "per_period",
    /// Growth applied annually regardless of period frequency.
    AnnualEscalator = "annual_escalator",
}

/// Base metric for management fee calculation.
#[wasm_bindgen(js_name = ManagementFeeBase)]
pub enum JsManagementFeeBase {
    /// Fee calculated as a percentage of Effective Gross Income.
    Egi = "egi",
    /// Fee calculated as a percentage of effective rent.
    EffectiveRent = "effective_rent",
}

impl JsManagementFeeBase {
    fn to_core(&self) -> ManagementFeeBase {
        match self {
            JsManagementFeeBase::Egi => ManagementFeeBase::Egi,
            JsManagementFeeBase::EffectiveRent => ManagementFeeBase::EffectiveRent,
            _ => ManagementFeeBase::Egi,
        }
    }
}

/// Management fee specification.
#[wasm_bindgen(js_name = ManagementFeeSpec)]
pub struct JsManagementFeeSpec {
    pub(crate) inner: ManagementFeeSpec,
}

#[wasm_bindgen(js_class = ManagementFeeSpec)]
impl JsManagementFeeSpec {
    /// Create a management fee specification.
    ///
    /// # Arguments
    /// * `rate` - Fee rate (e.g. 0.03 for 3%)
    /// * `base` - Fee calculation basis (default: EGI)
    #[wasm_bindgen(constructor)]
    pub fn new(rate: f64, base: Option<JsManagementFeeBase>) -> JsManagementFeeSpec {
        JsManagementFeeSpec {
            inner: ManagementFeeSpec {
                rate,
                base: base.map(|b| b.to_core()).unwrap_or_default(),
            },
        }
    }

    /// Fee rate.
    #[wasm_bindgen(getter)]
    pub fn rate(&self) -> f64 {
        self.inner.rate
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ManagementFeeSpec(rate={:.4}, base={:?})",
            self.inner.rate, self.inner.base
        )
    }
}

/// Enhanced lease specification with rent steps, free-rent windows, and renewals.
#[wasm_bindgen(js_name = LeaseSpec)]
pub struct JsLeaseSpec {
    pub(crate) inner: LeaseSpec,
}

#[wasm_bindgen(js_class = LeaseSpec)]
impl JsLeaseSpec {
    /// Create from JSON representation.
    ///
    /// Accepts a JS object with fields: `nodeId`, `start`, `baseRent`,
    /// `growthRate`, `growthConvention`, `end`, `rentSteps`, `freeRentPeriods`,
    /// `freeRentWindows`, `occupancy`, `renewal`.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsLeaseSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsLeaseSpec { inner })
            .map_err(|e| js_error(format!("Failed to deserialize LeaseSpec: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize LeaseSpec: {e}")))
    }

    /// Node identifier.
    #[wasm_bindgen(getter, js_name = nodeId)]
    pub fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    /// Start period string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// Starting annual rent.
    #[wasm_bindgen(getter, js_name = baseRent)]
    pub fn base_rent(&self) -> f64 {
        self.inner.base_rent
    }

    /// Per-period rent escalation rate.
    #[wasm_bindgen(getter, js_name = growthRate)]
    pub fn growth_rate(&self) -> f64 {
        self.inner.growth_rate
    }

    /// Occupancy rate.
    #[wasm_bindgen(getter)]
    pub fn occupancy(&self) -> f64 {
        self.inner.occupancy
    }

    /// Validate the lease specification.
    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| js_error(e.to_string()))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "LeaseSpec(nodeId='{}', start={}, baseRent={:.2})",
            self.inner.node_id, self.inner.start, self.inner.base_rent
        )
    }
}

/// Node names for rent-roll output allocation.
#[wasm_bindgen(js_name = RentRollOutputNodes)]
pub struct JsRentRollOutputNodes {
    pub(crate) inner: RentRollOutputNodes,
}

#[wasm_bindgen(js_class = RentRollOutputNodes)]
impl JsRentRollOutputNodes {
    /// Create with optional overrides for node names.
    #[wasm_bindgen(constructor)]
    pub fn new(
        rent_pgi_node: Option<String>,
        free_rent_node: Option<String>,
        vacancy_loss_node: Option<String>,
        rent_effective_node: Option<String>,
    ) -> JsRentRollOutputNodes {
        JsRentRollOutputNodes {
            inner: RentRollOutputNodes {
                rent_pgi_node: rent_pgi_node.unwrap_or_else(|| "rent_pgi".to_string()),
                free_rent_node: free_rent_node.unwrap_or_else(|| "free_rent".to_string()),
                vacancy_loss_node: vacancy_loss_node.unwrap_or_else(|| "vacancy_loss".to_string()),
                rent_effective_node: rent_effective_node
                    .unwrap_or_else(|| "rent_effective".to_string()),
            },
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "RentRollOutputNodes(pgi='{}', free='{}', vacancy='{}', effective='{}')",
            self.inner.rent_pgi_node,
            self.inner.free_rent_node,
            self.inner.vacancy_loss_node,
            self.inner.rent_effective_node
        )
    }
}

/// Node names for a full property operating statement template.
#[wasm_bindgen(js_name = PropertyTemplateNodes)]
pub struct JsPropertyTemplateNodes {
    pub(crate) inner: PropertyTemplateNodes,
}

#[wasm_bindgen(js_class = PropertyTemplateNodes)]
impl JsPropertyTemplateNodes {
    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPropertyTemplateNodes, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPropertyTemplateNodes { inner })
            .map_err(|e| js_error(format!("Failed to deserialize PropertyTemplateNodes: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize PropertyTemplateNodes: {e}")))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "PropertyTemplateNodes(noi='{}', ncf='{}')",
            self.inner.noi_node, self.inner.ncf_node
        )
    }
}
