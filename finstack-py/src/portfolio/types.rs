//! Python bindings for portfolio core types.

use crate::core::currency::PyCurrency;
use crate::valuations::instruments::extract_instrument;
use finstack_portfolio::{Entity, Position, PositionUnit};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;
use std::sync::Arc;

/// An entity that can hold positions.
///
/// Entities represent companies, funds, or other legal entities that own instruments.
/// For standalone instruments, use the dummy entity via :meth:`Entity.dummy`.
///
/// Examples:
///     >>> entity = Entity("ACME_CORP")
///     >>> entity = entity.with_name("Acme Corporation")
///     >>> entity = entity.with_tag("sector", "Technology")
#[pyclass(module = "finstack.portfolio", name = "Entity")]
#[derive(Clone)]
pub struct PyEntity {
    pub(crate) inner: Entity,
}

impl PyEntity {
    pub(crate) fn new(inner: Entity) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEntity {
    #[new]
    #[pyo3(text_signature = "(id)")]
    /// Create a new entity with the given ID.
    ///
    /// Args:
    ///     id: Unique entity identifier.
    ///
    /// Returns:
    ///     Entity: New entity instance.
    ///
    /// Examples:
    ///     >>> entity = Entity("ACME_CORP")
    ///     >>> entity.id
    ///     'ACME_CORP'
    fn new_py(id: String) -> Self {
        Self::new(Entity::new(id))
    }

    #[pyo3(text_signature = "($self, name)")]
    /// Set the entity name.
    ///
    /// Args:
    ///     name: Human-readable name.
    ///
    /// Returns:
    ///     Entity: Entity with updated name (builder pattern).
    ///
    /// Examples:
    ///     >>> entity = Entity("ACME").with_name("Acme Corporation")
    ///     >>> entity.name
    ///     'Acme Corporation'
    fn with_name(&self, name: String) -> Self {
        Self::new(self.inner.clone().with_name(name))
    }

    #[pyo3(text_signature = "($self, key, value)")]
    /// Add a tag to the entity.
    ///
    /// Args:
    ///     key: Tag key.
    ///     value: Tag value.
    ///
    /// Returns:
    ///     Entity: Entity with added tag (builder pattern).
    ///
    /// Examples:
    ///     >>> entity = Entity("ACME").with_tag("sector", "Technology")
    ///     >>> entity.tags["sector"]
    ///     'Technology'
    fn with_tag(&self, key: String, value: String) -> Self {
        Self::new(self.inner.clone().with_tag(key, value))
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    /// Create the dummy entity for standalone instruments.
    ///
    /// Returns:
    ///     Entity: Dummy entity with special identifier.
    ///
    /// Examples:
    ///     >>> dummy = Entity.dummy()
    ///     >>> dummy.id
    ///     '_standalone'
    fn dummy() -> Self {
        Self::new(Entity::dummy())
    }

    #[getter]
    /// Get the entity identifier.
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    /// Get the entity name.
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    /// Get the entity tags.
    fn tags(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.tags {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get entity metadata.
    fn meta(&self, py: Python<'_>) -> PyResult<PyObject> {
        let bound = pythonize::pythonize(py, &self.inner.meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;
        Ok(bound.unbind())
    }

    fn __repr__(&self) -> String {
        if let Some(name) = &self.inner.name {
            format!("Entity(id='{}', name='{}')", self.inner.id, name)
        } else {
            format!("Entity(id='{}')", self.inner.id)
        }
    }

    fn __str__(&self) -> String {
        self.inner.name.clone().unwrap_or_else(|| self.inner.id.clone())
    }
}

/// Unit of position measurement.
///
/// Describes how the quantity on a position should be interpreted.
///
/// Variants:
///     UNITS: Number of units/shares (for equities, baskets)
///     NOTIONAL: Notional amount, optionally in a specific currency (for derivatives, FX)
///     FACE_VALUE: Face value of debt instruments (for bonds, loans)
///     PERCENTAGE: Percentage of ownership
///
/// Examples:
///     >>> unit = PositionUnit.UNITS
///     >>> unit = PositionUnit.notional_with_ccy(Currency.USD)
///     >>> unit = PositionUnit.FACE_VALUE
#[pyclass(module = "finstack.portfolio", name = "PositionUnit")]
#[derive(Clone, Copy)]
pub struct PyPositionUnit {
    pub(crate) inner: PositionUnit,
}

impl PyPositionUnit {
    pub(crate) fn new(inner: PositionUnit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionUnit {
    #[classattr]
    const UNITS: Self = Self {
        inner: PositionUnit::Units,
    };

    #[classattr]
    const FACE_VALUE: Self = Self {
        inner: PositionUnit::FaceValue,
    };

    #[classattr]
    const PERCENTAGE: Self = Self {
        inner: PositionUnit::Percentage,
    };

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    /// Create a notional position unit without specific currency.
    ///
    /// Returns:
    ///     PositionUnit: Notional unit.
    fn notional() -> Self {
        Self::new(PositionUnit::Notional(None))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(currency)")]
    /// Create a notional position unit with specific currency.
    ///
    /// Args:
    ///     currency: Currency for the notional amount.
    ///
    /// Returns:
    ///     PositionUnit: Notional unit with currency.
    fn notional_with_ccy(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = if let Ok(py_ccy) = currency.extract::<PyRef<PyCurrency>>() {
            py_ccy.inner
        } else if let Ok(s) = currency.extract::<String>() {
            s.parse().map_err(|e| PyValueError::new_err(format!("Invalid currency: {}", e)))?
        } else {
            return Err(PyTypeError::new_err("Expected Currency or string"));
        };
        Ok(Self::new(PositionUnit::Notional(Some(ccy))))
    }

    fn __repr__(&self) -> String {
        match self.inner {
            PositionUnit::Units => "PositionUnit.UNITS".to_string(),
            PositionUnit::Notional(None) => "PositionUnit.notional()".to_string(),
            PositionUnit::Notional(Some(ccy)) => format!("PositionUnit.notional_with_ccy(Currency.{})", ccy),
            PositionUnit::FaceValue => "PositionUnit.FACE_VALUE".to_string(),
            PositionUnit::Percentage => "PositionUnit.PERCENTAGE".to_string(),
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            PositionUnit::Units => "units".to_string(),
            PositionUnit::Notional(None) => "notional".to_string(),
            PositionUnit::Notional(Some(ccy)) => format!("notional({})", ccy),
            PositionUnit::FaceValue => "face_value".to_string(),
            PositionUnit::Percentage => "percentage".to_string(),
        }
    }
}

/// A position in an instrument.
///
/// Represents a holding of a specific quantity of an instrument, belonging to an entity.
/// Positions track the instrument reference, quantity, unit, and metadata for aggregation.
///
/// Examples:
///     >>> from finstack.valuations.instruments import Deposit
///     >>> from finstack.core import Money, Currency
///     >>> deposit = Deposit.fixed("DEP_1M", Money(Currency.USD, 1_000_000), ...)
///     >>> position = Position("POS_001", "ENTITY_A", "DEP_1M", deposit, 1.0, PositionUnit.UNITS)
///     >>> position.is_long()
///     True
#[pyclass(module = "finstack.portfolio", name = "Position")]
pub struct PyPosition {
    pub(crate) inner: Position,
}

impl PyPosition {
    pub(crate) fn new(inner: Position) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPosition {
    #[new]
    #[pyo3(signature = (position_id, entity_id, instrument_id, instrument, quantity, unit))]
    #[pyo3(text_signature = "(position_id, entity_id, instrument_id, instrument, quantity, unit)")]
    /// Create a new position.
    ///
    /// Args:
    ///     position_id: Unique identifier for the position.
    ///     entity_id: Owning entity identifier.
    ///     instrument_id: Instrument identifier (for reference/lookup).
    ///     instrument: The actual instrument being held.
    ///     quantity: Signed quantity (positive=long, negative=short).
    ///     unit: Unit of measurement for the quantity.
    ///
    /// Returns:
    ///     Position: New position instance.
    ///
    /// Raises:
    ///     TypeError: If instrument is not a valid instrument type.
    fn new_py(
        position_id: String,
        entity_id: String,
        instrument_id: String,
        instrument: &Bound<'_, PyAny>,
        quantity: f64,
        unit: PyPositionUnit,
    ) -> PyResult<Self> {
        let handle = extract_instrument(instrument)?;
        let position = Position::new(
            position_id,
            entity_id,
            instrument_id,
            Arc::from(handle.instrument),
            quantity,
            unit.inner,
        );
        Ok(Self::new(position))
    }

    #[pyo3(text_signature = "($self)")]
    /// Check if the position is long (positive quantity).
    ///
    /// Returns:
    ///     bool: True if quantity is positive.
    fn is_long(&self) -> bool {
        self.inner.is_long()
    }

    #[pyo3(text_signature = "($self)")]
    /// Check if the position is short (negative quantity).
    ///
    /// Returns:
    ///     bool: True if quantity is negative.
    fn is_short(&self) -> bool {
        self.inner.is_short()
    }

    #[getter]
    /// Get the position identifier.
    fn position_id(&self) -> String {
        self.inner.position_id.clone()
    }

    #[getter]
    /// Get the entity identifier.
    fn entity_id(&self) -> String {
        self.inner.entity_id.clone()
    }

    #[getter]
    /// Get the instrument identifier.
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[getter]
    /// Get the quantity.
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[getter]
    /// Get the position unit.
    fn unit(&self) -> PyPositionUnit {
        PyPositionUnit::new(self.inner.unit)
    }

    #[getter]
    /// Get position tags.
    fn tags(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.tags {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get position metadata.
    fn meta(&self, py: Python<'_>) -> PyResult<PyObject> {
        let bound = pythonize::pythonize(py, &self.inner.meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;
        Ok(bound.unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "Position(id='{}', entity='{}', instrument='{}', qty={}, unit={})",
            self.inner.position_id,
            self.inner.entity_id,
            self.inner.instrument_id,
            self.inner.quantity,
            match self.inner.unit {
                PositionUnit::Units => "UNITS",
                PositionUnit::Notional(_) => "NOTIONAL",
                PositionUnit::FaceValue => "FACE_VALUE",
                PositionUnit::Percentage => "PERCENTAGE",
            }
        )
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.inner.position_id, self.inner.instrument_id)
    }
}

/// Extract an Entity from Python object.
pub(crate) fn extract_entity(value: &Bound<'_, PyAny>) -> PyResult<Entity> {
    if let Ok(py_entity) = value.extract::<PyRef<PyEntity>>() {
        Ok(py_entity.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected Entity"))
    }
}

/// Extract a Position from Python object.
pub(crate) fn extract_position(value: &Bound<'_, PyAny>) -> PyResult<Position> {
    if let Ok(py_pos) = value.extract::<PyRef<PyPosition>>() {
        Ok(py_pos.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected Position"))
    }
}

/// Register types module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyEntity>()?;
    parent.add_class::<PyPositionUnit>()?;
    parent.add_class::<PyPosition>()?;

    Ok(vec![
        "Entity".to_string(),
        "PositionUnit".to_string(),
        "Position".to_string(),
    ])
}

