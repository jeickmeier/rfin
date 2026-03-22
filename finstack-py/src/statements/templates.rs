//! Python bindings for real-estate financial statement templates.
//!
//! Wraps `finstack_statements::templates::real_estate` types and provides
//! extension methods on `PyModelBuilder` for constructing NOI/NCF buildups,
//! rent roll projections, and full property operating statements.

use crate::core::dates::periods::PyPeriodId;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use finstack_statements_analytics::templates::real_estate::{
    FreeRentWindowSpec, LeaseGrowthConvention, LeaseSpec, LeaseSpecV2, ManagementFeeBase,
    ManagementFeeSpec, PropertyTemplateNodes, RenewalSpec, RentRollOutputNodes, RentStepSpec,
};

// ---------------------------------------------------------------------------
// LeaseSpec (v1)
// ---------------------------------------------------------------------------

/// Lease specification for simple rent-roll modelling (v1).
///
/// Args:
///     node_id: Node identifier for this lease in the model.
///     start: Period when the lease begins.
///     end: Period when the lease ends (``None`` = model horizon).
///     base_rent: Starting annual rent.
///     growth_rate: Per-period rent escalation rate.
///     free_rent_periods: Number of initial periods with zero rent.
///     occupancy: Occupancy rate (0 to 1).
#[pyclass(
    module = "finstack.statements.templates",
    name = "LeaseSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyLeaseSpec {
    pub(crate) inner: LeaseSpec,
}

#[pymethods]
impl PyLeaseSpec {
    #[new]
    #[pyo3(signature = (node_id, start, base_rent, growth_rate=0.0, end=None, free_rent_periods=0, occupancy=1.0))]
    fn new(
        node_id: String,
        start: PyPeriodId,
        base_rent: f64,
        growth_rate: f64,
        end: Option<PyPeriodId>,
        free_rent_periods: u32,
        occupancy: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: LeaseSpec {
                node_id,
                start: start.inner,
                end: end.map(|e| e.inner),
                base_rent,
                growth_rate,
                free_rent_periods,
                occupancy,
            },
        })
    }

    #[getter]
    fn node_id(&self) -> &str {
        &self.inner.node_id
    }

    #[getter]
    fn start(&self) -> PyPeriodId {
        PyPeriodId::new(self.inner.start)
    }

    #[getter]
    fn base_rent(&self) -> f64 {
        self.inner.base_rent
    }

    #[getter]
    fn growth_rate(&self) -> f64 {
        self.inner.growth_rate
    }

    #[getter]
    fn occupancy(&self) -> f64 {
        self.inner.occupancy
    }

    fn __repr__(&self) -> String {
        format!(
            "LeaseSpec(node_id='{}', start={:?}, base_rent={:.2})",
            self.inner.node_id, self.inner.start, self.inner.base_rent
        )
    }

    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
    }
}

// ---------------------------------------------------------------------------
// RentStepSpec
// ---------------------------------------------------------------------------

/// Specifies a discrete rent step (absolute rent override at a given period).
///
/// Args:
///     start: Period when the step takes effect.
///     rent: Absolute rent amount from this period onward.
#[pyclass(
    module = "finstack.statements.templates",
    name = "RentStepSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRentStepSpec {
    pub(crate) inner: RentStepSpec,
}

#[pymethods]
impl PyRentStepSpec {
    #[new]
    fn new(start: PyPeriodId, rent: f64) -> Self {
        Self {
            inner: RentStepSpec {
                start: start.inner,
                rent,
            },
        }
    }

    #[getter]
    fn start(&self) -> PyPeriodId {
        PyPeriodId::new(self.inner.start)
    }

    #[getter]
    fn rent(&self) -> f64 {
        self.inner.rent
    }

    fn __repr__(&self) -> String {
        format!(
            "RentStepSpec(start={:?}, rent={:.2})",
            self.inner.start, self.inner.rent
        )
    }
}

// ---------------------------------------------------------------------------
// FreeRentWindowSpec
// ---------------------------------------------------------------------------

/// Free-rent window specification.
///
/// Args:
///     start: Period when the free-rent window begins.
///     periods: Number of consecutive free-rent periods.
#[pyclass(
    module = "finstack.statements.templates",
    name = "FreeRentWindowSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyFreeRentWindowSpec {
    pub(crate) inner: FreeRentWindowSpec,
}

#[pymethods]
impl PyFreeRentWindowSpec {
    #[new]
    fn new(start: PyPeriodId, periods: u32) -> Self {
        Self {
            inner: FreeRentWindowSpec {
                start: start.inner,
                periods,
            },
        }
    }

    #[getter]
    fn start(&self) -> PyPeriodId {
        PyPeriodId::new(self.inner.start)
    }

    #[getter]
    fn periods(&self) -> u32 {
        self.inner.periods
    }

    fn __repr__(&self) -> String {
        format!(
            "FreeRentWindowSpec(start={:?}, periods={})",
            self.inner.start, self.inner.periods
        )
    }
}

// ---------------------------------------------------------------------------
// RenewalSpec
// ---------------------------------------------------------------------------

/// Lease renewal assumption.
///
/// Args:
///     downtime_periods: Vacancy periods between lease expiry and renewal.
///     term_periods: Duration of the renewed lease in periods.
///     probability: Probability the tenant renews (0 to 1).
///     rent_factor: Multiplier applied to the previous rent at renewal.
///     free_rent_periods: Free rent given at renewal start.
#[pyclass(
    module = "finstack.statements.templates",
    name = "RenewalSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRenewalSpec {
    pub(crate) inner: RenewalSpec,
}

#[pymethods]
impl PyRenewalSpec {
    #[new]
    #[pyo3(signature = (downtime_periods=0, term_periods=12, probability=1.0, rent_factor=1.0, free_rent_periods=0))]
    fn new(
        downtime_periods: u32,
        term_periods: u32,
        probability: f64,
        rent_factor: f64,
        free_rent_periods: u32,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RenewalSpec {
                downtime_periods,
                term_periods,
                probability,
                rent_factor,
                free_rent_periods,
            },
        })
    }

    #[getter]
    fn downtime_periods(&self) -> u32 {
        self.inner.downtime_periods
    }

    #[getter]
    fn term_periods(&self) -> u32 {
        self.inner.term_periods
    }

    #[getter]
    fn probability(&self) -> f64 {
        self.inner.probability
    }

    #[getter]
    fn rent_factor(&self) -> f64 {
        self.inner.rent_factor
    }

    fn __repr__(&self) -> String {
        format!(
            "RenewalSpec(downtime={}, term={}, prob={:.2}, factor={:.2})",
            self.inner.downtime_periods,
            self.inner.term_periods,
            self.inner.probability,
            self.inner.rent_factor
        )
    }

    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
    }
}

// ---------------------------------------------------------------------------
// LeaseGrowthConvention
// ---------------------------------------------------------------------------

/// Growth convention for lease escalation.
///
/// Variants:
///     PER_PERIOD: Growth applied each model period.
///     ANNUAL_ESCALATOR: Growth applied annually regardless of period frequency.
#[pyclass(
    module = "finstack.statements.templates",
    name = "LeaseGrowthConvention",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PyLeaseGrowthConvention {
    pub(crate) inner: LeaseGrowthConvention,
}

#[pymethods]
impl PyLeaseGrowthConvention {
    #[classattr]
    const PER_PERIOD: Self = Self {
        inner: LeaseGrowthConvention::PerPeriod,
    };

    #[classattr]
    const ANNUAL_ESCALATOR: Self = Self {
        inner: LeaseGrowthConvention::AnnualEscalator,
    };

    fn __repr__(&self) -> &'static str {
        match self.inner {
            LeaseGrowthConvention::PerPeriod => "LeaseGrowthConvention.PER_PERIOD",
            LeaseGrowthConvention::AnnualEscalator => "LeaseGrowthConvention.ANNUAL_ESCALATOR",
        }
    }
}

// ---------------------------------------------------------------------------
// ManagementFeeBase
// ---------------------------------------------------------------------------

/// Base metric for management fee calculation.
///
/// Variants:
///     EGI: Fee calculated as a percentage of Effective Gross Income.
///     EFFECTIVE_RENT: Fee calculated as a percentage of effective rent.
#[pyclass(
    module = "finstack.statements.templates",
    name = "ManagementFeeBase",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PyManagementFeeBase {
    pub(crate) inner: ManagementFeeBase,
}

#[pymethods]
impl PyManagementFeeBase {
    #[classattr]
    const EGI: Self = Self {
        inner: ManagementFeeBase::Egi,
    };

    #[classattr]
    const EFFECTIVE_RENT: Self = Self {
        inner: ManagementFeeBase::EffectiveRent,
    };

    fn __repr__(&self) -> &'static str {
        match self.inner {
            ManagementFeeBase::Egi => "ManagementFeeBase.EGI",
            ManagementFeeBase::EffectiveRent => "ManagementFeeBase.EFFECTIVE_RENT",
        }
    }
}

// ---------------------------------------------------------------------------
// ManagementFeeSpec
// ---------------------------------------------------------------------------

/// Management fee specification.
///
/// Args:
///     rate: Fee rate (e.g. 0.03 for 3%).
///     base: :class:`ManagementFeeBase` determining the fee calculation basis.
#[pyclass(
    module = "finstack.statements.templates",
    name = "ManagementFeeSpec",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyManagementFeeSpec {
    pub(crate) inner: ManagementFeeSpec,
}

#[pymethods]
impl PyManagementFeeSpec {
    #[new]
    #[pyo3(signature = (rate, base=None))]
    fn new(rate: f64, base: Option<PyManagementFeeBase>) -> Self {
        Self {
            inner: ManagementFeeSpec {
                rate,
                base: base.map(|b| b.inner).unwrap_or_default(),
            },
        }
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    fn __repr__(&self) -> String {
        format!(
            "ManagementFeeSpec(rate={:.4}, base={:?})",
            self.inner.rate, self.inner.base
        )
    }
}

// ---------------------------------------------------------------------------
// LeaseSpecV2
// ---------------------------------------------------------------------------

/// Enhanced lease specification with discrete rent steps, free-rent windows,
/// and renewal assumptions (v2).
///
/// Args:
///     node_id: Node identifier for this lease in the model.
///     start: Period when the lease begins.
///     base_rent: Starting annual rent.
///     growth_rate: Per-period rent escalation rate.
///     growth_convention: :class:`LeaseGrowthConvention` (default per-period).
///     end: Period when the lease ends (``None`` = model horizon).
///     rent_steps: List of :class:`RentStepSpec` for discrete overrides.
///     free_rent_periods: Number of initial periods with zero rent.
///     free_rent_windows: List of :class:`FreeRentWindowSpec`.
///     occupancy: Occupancy rate (0 to 1).
///     renewal: Optional :class:`RenewalSpec`.
#[pyclass(
    module = "finstack.statements.templates",
    name = "LeaseSpecV2",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyLeaseSpecV2 {
    pub(crate) inner: LeaseSpecV2,
}

#[pymethods]
impl PyLeaseSpecV2 {
    #[new]
    #[pyo3(signature = (
        node_id, start, base_rent,
        growth_rate=0.0,
        growth_convention=None,
        end=None,
        rent_steps=None,
        free_rent_periods=0,
        free_rent_windows=None,
        occupancy=1.0,
        renewal=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        node_id: String,
        start: PyPeriodId,
        base_rent: f64,
        growth_rate: f64,
        growth_convention: Option<PyLeaseGrowthConvention>,
        end: Option<PyPeriodId>,
        rent_steps: Option<Vec<PyRentStepSpec>>,
        free_rent_periods: u32,
        free_rent_windows: Option<Vec<PyFreeRentWindowSpec>>,
        occupancy: f64,
        renewal: Option<PyRenewalSpec>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: LeaseSpecV2 {
                node_id,
                start: start.inner,
                end: end.map(|e| e.inner),
                base_rent,
                growth_rate,
                growth_convention: growth_convention.map(|g| g.inner).unwrap_or_default(),
                rent_steps: rent_steps
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.inner)
                    .collect(),
                free_rent_periods,
                free_rent_windows: free_rent_windows
                    .unwrap_or_default()
                    .into_iter()
                    .map(|w| w.inner)
                    .collect(),
                occupancy,
                renewal: renewal.map(|r| r.inner),
            },
        })
    }

    #[getter]
    fn node_id(&self) -> &str {
        &self.inner.node_id
    }

    #[getter]
    fn start(&self) -> PyPeriodId {
        PyPeriodId::new(self.inner.start)
    }

    #[getter]
    fn base_rent(&self) -> f64 {
        self.inner.base_rent
    }

    #[getter]
    fn growth_rate(&self) -> f64 {
        self.inner.growth_rate
    }

    #[getter]
    fn occupancy(&self) -> f64 {
        self.inner.occupancy
    }

    fn __repr__(&self) -> String {
        format!(
            "LeaseSpecV2(node_id='{}', start={:?}, base_rent={:.2})",
            self.inner.node_id, self.inner.start, self.inner.base_rent
        )
    }

    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
    }
}

// ---------------------------------------------------------------------------
// RentRollOutputNodes
// ---------------------------------------------------------------------------

/// Node names for rent-roll output allocation.
///
/// Args:
///     rent_pgi_node: Potential gross income node (default ``"rent_pgi"``).
///     free_rent_node: Free rent adjustment node (default ``"free_rent"``).
///     vacancy_loss_node: Vacancy loss node (default ``"vacancy_loss"``).
///     rent_effective_node: Effective rent node (default ``"rent_effective"``).
#[pyclass(
    module = "finstack.statements.templates",
    name = "RentRollOutputNodes",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRentRollOutputNodes {
    pub(crate) inner: RentRollOutputNodes,
}

#[pymethods]
impl PyRentRollOutputNodes {
    #[new]
    #[pyo3(signature = (
        rent_pgi_node="rent_pgi".to_string(),
        free_rent_node="free_rent".to_string(),
        vacancy_loss_node="vacancy_loss".to_string(),
        rent_effective_node="rent_effective".to_string(),
    ))]
    fn new(
        rent_pgi_node: String,
        free_rent_node: String,
        vacancy_loss_node: String,
        rent_effective_node: String,
    ) -> Self {
        Self {
            inner: RentRollOutputNodes {
                rent_pgi_node,
                free_rent_node,
                vacancy_loss_node,
                rent_effective_node,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RentRollOutputNodes(pgi='{}', free='{}', vacancy='{}', effective='{}')",
            self.inner.rent_pgi_node,
            self.inner.free_rent_node,
            self.inner.vacancy_loss_node,
            self.inner.rent_effective_node
        )
    }
}

// ---------------------------------------------------------------------------
// PropertyTemplateNodes
// ---------------------------------------------------------------------------

/// Node names for a full property operating statement template.
///
/// Args:
///     rent_roll: :class:`RentRollOutputNodes` for rent decomposition.
///     other_income_total_node: Other income aggregation node.
///     egi_node: Effective gross income node.
///     management_fee_node: Management fee node.
///     opex_total_node: Operating expenses total node.
///     noi_node: Net operating income node.
///     capex_total_node: Capital expenditures total node.
///     ncf_node: Net cash flow node.
#[pyclass(
    module = "finstack.statements.templates",
    name = "PropertyTemplateNodes",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyPropertyTemplateNodes {
    pub(crate) inner: PropertyTemplateNodes,
}

#[pymethods]
impl PyPropertyTemplateNodes {
    #[new]
    #[pyo3(signature = (
        rent_roll=None,
        other_income_total_node="other_income_total".to_string(),
        egi_node="egi".to_string(),
        management_fee_node="management_fee".to_string(),
        opex_total_node="opex_total".to_string(),
        noi_node="noi".to_string(),
        capex_total_node="capex_total".to_string(),
        ncf_node="ncf".to_string(),
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        rent_roll: Option<PyRentRollOutputNodes>,
        other_income_total_node: String,
        egi_node: String,
        management_fee_node: String,
        opex_total_node: String,
        noi_node: String,
        capex_total_node: String,
        ncf_node: String,
    ) -> Self {
        Self {
            inner: PropertyTemplateNodes {
                rent_roll: rent_roll.map(|r| r.inner).unwrap_or_default(),
                other_income_total_node,
                egi_node,
                management_fee_node,
                opex_total_node,
                noi_node,
                capex_total_node,
                ncf_node,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PropertyTemplateNodes(noi='{}', ncf='{}')",
            self.inner.noi_node, self.inner.ncf_node
        )
    }
}

// ---------------------------------------------------------------------------
// Module Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "templates")?;
    module.setattr(
        "__doc__",
        "Real-estate financial statement templates: lease specs, rent rolls, NOI/NCF buildups, and property operating statements.",
    )?;

    module.add_class::<PyLeaseSpec>()?;
    module.add_class::<PyRentStepSpec>()?;
    module.add_class::<PyFreeRentWindowSpec>()?;
    module.add_class::<PyRenewalSpec>()?;
    module.add_class::<PyLeaseGrowthConvention>()?;
    module.add_class::<PyManagementFeeBase>()?;
    module.add_class::<PyManagementFeeSpec>()?;
    module.add_class::<PyLeaseSpecV2>()?;
    module.add_class::<PyRentRollOutputNodes>()?;
    module.add_class::<PyPropertyTemplateNodes>()?;

    let exports = vec![
        "LeaseSpec",
        "RentStepSpec",
        "FreeRentWindowSpec",
        "RenewalSpec",
        "LeaseGrowthConvention",
        "ManagementFeeBase",
        "ManagementFeeSpec",
        "LeaseSpecV2",
        "RentRollOutputNodes",
        "PropertyTemplateNodes",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
