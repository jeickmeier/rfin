---
description: Rust-Python Bindings
alwaysApply: false
---
# Python Bindings Code Standards for rfin-python

## Core Principles

1. **Pythonic API** - Python bindings should feel natural to Python developers
2. **Zero-copy where possible** - Minimize data copying between Rust and Python
3. **Clear error mapping** - Convert Rust errors to appropriate Python exceptions
4. **Comprehensive docstrings** - Every public API must have Python-style documentation

## Module Structure

### Organization
```
rfin-python/
├── src/
│   ├── lib.rs          # Main module registration
│   ├── currency.rs     # Currency bindings
│   ├── money.rs        # Money bindings
│   ├── dates.rs        # Date bindings
│   ├── calendar.rs     # Calendar bindings
│   ├── daycount.rs     # DayCount bindings
│   ├── cashflow.rs     # CashFlow bindings
│   └── schedule.rs     # Schedule bindings
└── tests/
    └── test_*.py       # Python integration tests
```

### Module Registration Pattern
```rust
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // Create submodule
    let submodule = PyModule::new(m.py(), "submodule")?;
    submodule.add_class::<PyType>()?;
    m.add_submodule(&submodule)?;
    
    // Register in sys.modules for proper imports
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("rfin.submodule", &submodule)?;
    
    // Top-level re-export for convenience
    m.add_class::<PyType>()?;
    
    Ok(())
}
```

## Type Wrapping

### Basic Pattern
```rust
/// Python-visible documentation here.
///
/// Examples:
///     >>> from rfin import TypeName
///     >>> instance = TypeName()
#[pyclass(name = "TypeName", module = "rfin.module")]
#[derive(Clone)]
pub struct PyTypeName {
    inner: CoreType,
}
```

### Constructor Pattern
```rust
#[pymethods]
impl PyTypeName {
    /// Create a new TypeName.
    ///
    /// Args:
    ///     param (type): Description of parameter
    ///
    /// Returns:
    ///     TypeName: Description of return value
    ///
    /// Raises:
    ///     ValueError: When input is invalid
    ///
    /// Examples:
    ///     >>> obj = TypeName(param)
    #[new]
    #[pyo3(text_signature = "(param)")]
    fn new(param: Type) -> PyResult<Self> {
        // Validate and construct
        Ok(PyTypeName { inner: CoreType::new(param) })
    }
}
```

## Error Handling

### Error Conversion Pattern
```rust
// Convert Rust errors to Python exceptions
fn convert_error(err: rfin_core::Error) -> PyErr {
    use rfin_core::error::{Error, InputError};
    match err {
        Error::Input(InputError::InvalidDateRange) => {
            PyErr::new::<PyValueError, _>("Invalid date range: start must be before end")
        }
        Error::CurrencyMismatch { expected, actual } => {
            PyErr::new::<PyValueError, _>(format!(
                "Currency mismatch: expected {}, got {}", expected, actual
            ))
        }
        Error::InterpOutOfBounds => {
            PyErr::new::<PyIndexError, _>("Interpolation input out of bounds")
        }
        _ => PyErr::new::<PyRuntimeError, _>(format!("Operation failed: {}", err))
    }
}
```

### Method Error Handling
```rust
#[pymethods]
impl PyType {
    fn risky_operation(&self) -> PyResult<ReturnType> {
        self.inner
            .risky_operation()
            .map(|result| PyReturnType::from(result))
            .map_err(convert_error)
    }
}
```

## Documentation Standards

### Class Documentation
```rust
/// One-line summary of the class.
///
/// Longer description explaining the purpose, use cases, and behavior
/// of the class. Include information about:
/// - What the class represents
/// - When to use it
/// - Key features and capabilities
///
/// The class supports:
/// - Feature 1
/// - Feature 2
/// - Feature 3
///
/// Examples:
///     >>> from rfin import ClassName
///     >>> # Basic usage
///     >>> obj = ClassName(param1, param2)
///     >>> obj.method()
///     result
///     
///     >>> # Advanced usage
///     >>> obj2 = ClassName.from_alternative(data)
///     >>> obj2.property
///     value
```

### Method Documentation
```rust
/// One-line summary of what the method does.
///
/// Longer description if needed, explaining details of behavior,
/// edge cases, or important notes.
///
/// Args:
///     param1 (type): Description of first parameter
///     param2 (type, optional): Description of optional parameter.
///                              Defaults to value.
///
/// Returns:
///     ReturnType: Description of what is returned
///
/// Raises:
///     ValueError: When and why this error is raised
///     RuntimeError: When and why this error is raised
///
/// Examples:
///     >>> obj.method(param1, param2)
///     expected_result
///     
///     >>> # Edge case example
///     >>> obj.method(edge_case_input)
///     edge_case_result
```

## Property and Method Patterns

### Properties (Getters)
```rust
/// The property description.
///
/// Returns:
///     type: What this property represents
///
/// Examples:
///     >>> obj.property_name
///     value
#[getter]
fn property_name(&self) -> ReturnType {
    self.inner.property()
}
```

### Class Methods
```rust
/// Create an instance using an alternative constructor.
///
/// Args:
///     cls: The class (automatically provided)
///     param: Parameter description
///
/// Returns:
///     ClassName: New instance created via this method
#[classmethod]
fn from_alternative(_cls: &Bound<'_, PyType>, param: Type) -> PyResult<Self> {
    // Implementation
}
```

### Static Methods
```rust
/// Utility function that doesn't need an instance.
///
/// Args:
///     param: Parameter description
///
/// Returns:
///     type: Return value description
#[staticmethod]
fn utility_function(param: Type) -> ReturnType {
    // Implementation
}
```

## Magic Methods

### String Representation
```rust
fn __str__(&self) -> String {
    // User-friendly representation
    format!("{}", self.inner)
}

fn __repr__(&self) -> String {
    // Developer-friendly representation that could recreate the object
    format!("ClassName('{}')", self.inner)
}
```

### Comparison
```rust
fn __eq__(&self, other: &Self) -> bool {
    self.inner == other.inner
}

fn __hash__(&self) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    self.inner.hash(&mut hasher);
    hasher.finish()
}
```

### Arithmetic Operations
```rust
fn __add__(&self, other: &Self) -> PyResult<Self> {
    match self.inner.checked_add(other.inner) {
        Ok(result) => Ok(Self { inner: result }),
        Err(e) => Err(convert_error(e)),
    }
}
```

## Enum Handling

### Simple Enums
```rust
#[pyclass(name = "EnumName", module = "rfin.module", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEnumName {
    /// Variant documentation
    Variant1,
    /// Another variant documentation
    Variant2,
}

// Conversion to/from Rust enum
impl From<PyEnumName> for CoreEnum {
    fn from(py_enum: PyEnumName) -> Self {
        match py_enum {
            PyEnumName::Variant1 => CoreEnum::Variant1,
            PyEnumName::Variant2 => CoreEnum::Variant2,
        }
    }
}
```

## Function Patterns

### Module-Level Functions
```rust
/// Function description with examples.
///
/// Args:
///     param1: Description
///     param2: Description
///
/// Returns:
///     type: Description
///
/// Examples:
///     >>> from rfin import function_name
///     >>> function_name(arg1, arg2)
///     result
#[pyfunction(name = "function_name", text_signature = "(param1, param2)")]
pub fn py_function_name(param1: Type1, param2: Type2) -> PyResult<ReturnType> {
    // Implementation
}
```

## Performance Considerations

### Zero-Copy Access
```rust
// Expose references when possible
#[getter]
fn data(&self, py: Python) -> &PyArray1<f64> {
    self.cached_array.as_ref(py)
}
```

### Lazy Computation
```rust
#[pyclass]
struct PyType {
    inner: CoreType,
    cached_result: Option<Py<PyAny>>,
}

#[pymethods]
impl PyType {
    #[getter]
    fn expensive_property(&mut self, py: Python) -> PyResult<PyObject> {
        if let Some(cached) = &self.cached_result {
            Ok(cached.clone_ref(py))
        } else {
            let result = compute_expensive();
            let py_result = result.into_py(py);
            self.cached_result = Some(py_result.clone_ref(py));
            Ok(py_result)
        }
    }
}
```

## Testing

### Python Test Structure
```python
import pytest
from rfin import ClassName

class TestClassName:
    """Test suite for ClassName."""
    
    def test_construction(self):
        """Test object construction."""
        obj = ClassName(param)
        assert obj.property == expected_value
    
    def test_error_handling(self):
        """Test that errors are properly raised."""
        with pytest.raises(ValueError, match="expected error message"):
            ClassName(invalid_param)
    
    @pytest.mark.parametrize("input,expected", [
        (input1, expected1),
        (input2, expected2),
    ])
    def test_parametrized(self, input, expected):
        """Test with multiple inputs."""
        result = ClassName(input).method()
        assert result == expected
```

## Type Hints and Stubs

### Generate .pyi Files
```python
# In .pyi stub files
from typing import Optional, List, Union

class ClassName:
    def __init__(self, param: str) -> None: ...
    @property
    def property_name(self) -> float: ...
    def method(self, param: int) -> Optional[str]: ...
    @classmethod
    def from_alternative(cls, data: List[float]) -> 'ClassName': ...
```

## Import Structure

### Recommended Import Pattern
```python
# Top-level imports for common types
from rfin import Currency, Money, Date

# Submodule imports for specialized functionality
from rfin.dates import Calendar, DayCount, Frequency
from rfin.cashflow import FixedRateLeg, CashFlow

# Function imports
from rfin.dates import generate_schedule, available_calendars
```

## Versioning and Compatibility

### Version Exposure
```rust
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // ...
}
```

### Deprecation Pattern
```rust
/// Deprecated: Use new_method instead.
///
/// .. deprecated:: 0.4.0
///    Use :meth:`new_method` instead.
#[pyo3(text_signature = "...")]
fn old_method(&self) -> PyResult<()> {
    PyErr::warn(
        py,
        PyDeprecationWarning::type_object(py),
        "old_method is deprecated, use new_method instead",
        1
    )?;
    self.new_method()
}
```

## Memory Management

### Clone vs Reference
```rust
// Prefer cloning for small Copy types
#[getter]
fn currency(&self) -> PyCurrency {
    PyCurrency::from_inner(self.inner.currency())
}

// Use Arc for large shared data
#[pyclass]
struct PyLargeData {
    inner: Arc<LargeData>,
}
```

### Lifetime Considerations
```rust
// Be explicit about Python lifetime when storing references
#[pyclass]
struct PyWrapper {
    data: Py<PyAny>,
}

impl PyWrapper {
    fn new(py: Python, data: PyObject) -> Self {
        Self { data: data.into() }
    }
}
``` # Python Bindings Code Standards for rfin-python

## Core Principles

1. **Pythonic API** - Python bindings should feel natural to Python developers
2. **Zero-copy where possible** - Minimize data copying between Rust and Python
3. **Clear error mapping** - Convert Rust errors to appropriate Python exceptions
4. **Comprehensive docstrings** - Every public API must have Python-style documentation

## Module Structure

### Organization
```
rfin-python/
├── src/
│   ├── lib.rs          # Main module registration
│   ├── currency.rs     # Currency bindings
│   ├── money.rs        # Money bindings
│   ├── dates.rs        # Date bindings
│   ├── calendar.rs     # Calendar bindings
│   ├── daycount.rs     # DayCount bindings
│   ├── cashflow.rs     # CashFlow bindings
│   └── schedule.rs     # Schedule bindings
└── tests/
    └── test_*.py       # Python integration tests
```

### Module Registration Pattern
```rust
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // Create submodule
    let submodule = PyModule::new(m.py(), "submodule")?;
    submodule.add_class::<PyType>()?;
    m.add_submodule(&submodule)?;
    
    // Register in sys.modules for proper imports
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("rfin.submodule", &submodule)?;
    
    // Top-level re-export for convenience
    m.add_class::<PyType>()?;
    
    Ok(())
}
```

## Type Wrapping

### Basic Pattern
```rust
/// Python-visible documentation here.
///
/// Examples:
///     >>> from rfin import TypeName
///     >>> instance = TypeName()
#[pyclass(name = "TypeName", module = "rfin.module")]
#[derive(Clone)]
pub struct PyTypeName {
    inner: CoreType,
}
```

### Constructor Pattern
```rust
#[pymethods]
impl PyTypeName {
    /// Create a new TypeName.
    ///
    /// Args:
    ///     param (type): Description of parameter
    ///
    /// Returns:
    ///     TypeName: Description of return value
    ///
    /// Raises:
    ///     ValueError: When input is invalid
    ///
    /// Examples:
    ///     >>> obj = TypeName(param)
    #[new]
    #[pyo3(text_signature = "(param)")]
    fn new(param: Type) -> PyResult<Self> {
        // Validate and construct
        Ok(PyTypeName { inner: CoreType::new(param) })
    }
}
```

## Error Handling

### Error Conversion Pattern
```rust
// Convert Rust errors to Python exceptions
fn convert_error(err: rfin_core::Error) -> PyErr {
    use rfin_core::error::{Error, InputError};
    match err {
        Error::Input(InputError::InvalidDateRange) => {
            PyErr::new::<PyValueError, _>("Invalid date range: start must be before end")
        }
        Error::CurrencyMismatch { expected, actual } => {
            PyErr::new::<PyValueError, _>(format!(
                "Currency mismatch: expected {}, got {}", expected, actual
            ))
        }
        Error::InterpOutOfBounds => {
            PyErr::new::<PyIndexError, _>("Interpolation input out of bounds")
        }
        _ => PyErr::new::<PyRuntimeError, _>(format!("Operation failed: {}", err))
    }
}
```

### Method Error Handling
```rust
#[pymethods]
impl PyType {
    fn risky_operation(&self) -> PyResult<ReturnType> {
        self.inner
            .risky_operation()
            .map(|result| PyReturnType::from(result))
            .map_err(convert_error)
    }
}
```

## Documentation Standards

### Class Documentation
```rust
/// One-line summary of the class.
///
/// Longer description explaining the purpose, use cases, and behavior
/// of the class. Include information about:
/// - What the class represents
/// - When to use it
/// - Key features and capabilities
///
/// The class supports:
/// - Feature 1
/// - Feature 2
/// - Feature 3
///
/// Examples:
///     >>> from rfin import ClassName
///     >>> # Basic usage
///     >>> obj = ClassName(param1, param2)
///     >>> obj.method()
///     result
///     
///     >>> # Advanced usage
///     >>> obj2 = ClassName.from_alternative(data)
///     >>> obj2.property
///     value
```

### Method Documentation
```rust
/// One-line summary of what the method does.
///
/// Longer description if needed, explaining details of behavior,
/// edge cases, or important notes.
///
/// Args:
///     param1 (type): Description of first parameter
///     param2 (type, optional): Description of optional parameter.
///                              Defaults to value.
///
/// Returns:
///     ReturnType: Description of what is returned
///
/// Raises:
///     ValueError: When and why this error is raised
///     RuntimeError: When and why this error is raised
///
/// Examples:
///     >>> obj.method(param1, param2)
///     expected_result
///     
///     >>> # Edge case example
///     >>> obj.method(edge_case_input)
///     edge_case_result
```

## Property and Method Patterns

### Properties (Getters)
```rust
/// The property description.
///
/// Returns:
///     type: What this property represents
///
/// Examples:
///     >>> obj.property_name
///     value
#[getter]
fn property_name(&self) -> ReturnType {
    self.inner.property()
}
```

### Class Methods
```rust
/// Create an instance using an alternative constructor.
///
/// Args:
///     cls: The class (automatically provided)
///     param: Parameter description
///
/// Returns:
///     ClassName: New instance created via this method
#[classmethod]
fn from_alternative(_cls: &Bound<'_, PyType>, param: Type) -> PyResult<Self> {
    // Implementation
}
```

### Static Methods
```rust
/// Utility function that doesn't need an instance.
///
/// Args:
///     param: Parameter description
///
/// Returns:
///     type: Return value description
#[staticmethod]
fn utility_function(param: Type) -> ReturnType {
    // Implementation
}
```

## Magic Methods

### String Representation
```rust
fn __str__(&self) -> String {
    // User-friendly representation
    format!("{}", self.inner)
}

fn __repr__(&self) -> String {
    // Developer-friendly representation that could recreate the object
    format!("ClassName('{}')", self.inner)
}
```

### Comparison
```rust
fn __eq__(&self, other: &Self) -> bool {
    self.inner == other.inner
}

fn __hash__(&self) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    self.inner.hash(&mut hasher);
    hasher.finish()
}
```

### Arithmetic Operations
```rust
fn __add__(&self, other: &Self) -> PyResult<Self> {
    match self.inner.checked_add(other.inner) {
        Ok(result) => Ok(Self { inner: result }),
        Err(e) => Err(convert_error(e)),
    }
}
```

## Enum Handling

### Simple Enums
```rust
#[pyclass(name = "EnumName", module = "rfin.module", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEnumName {
    /// Variant documentation
    Variant1,
    /// Another variant documentation
    Variant2,
}

// Conversion to/from Rust enum
impl From<PyEnumName> for CoreEnum {
    fn from(py_enum: PyEnumName) -> Self {
        match py_enum {
            PyEnumName::Variant1 => CoreEnum::Variant1,
            PyEnumName::Variant2 => CoreEnum::Variant2,
        }
    }
}
```

## Function Patterns

### Module-Level Functions
```rust
/// Function description with examples.
///
/// Args:
///     param1: Description
///     param2: Description
///
/// Returns:
///     type: Description
///
/// Examples:
///     >>> from rfin import function_name
///     >>> function_name(arg1, arg2)
///     result
#[pyfunction(name = "function_name", text_signature = "(param1, param2)")]
pub fn py_function_name(param1: Type1, param2: Type2) -> PyResult<ReturnType> {
    // Implementation
}
```

## Performance Considerations

### Zero-Copy Access
```rust
// Expose references when possible
#[getter]
fn data(&self, py: Python) -> &PyArray1<f64> {
    self.cached_array.as_ref(py)
}
```

### Lazy Computation
```rust
#[pyclass]
struct PyType {
    inner: CoreType,
    cached_result: Option<Py<PyAny>>,
}

#[pymethods]
impl PyType {
    #[getter]
    fn expensive_property(&mut self, py: Python) -> PyResult<PyObject> {
        if let Some(cached) = &self.cached_result {
            Ok(cached.clone_ref(py))
        } else {
            let result = compute_expensive();
            let py_result = result.into_py(py);
            self.cached_result = Some(py_result.clone_ref(py));
            Ok(py_result)
        }
    }
}
```

## Testing

### Python Test Structure
```python
import pytest
from rfin import ClassName

class TestClassName:
    """Test suite for ClassName."""
    
    def test_construction(self):
        """Test object construction."""
        obj = ClassName(param)
        assert obj.property == expected_value
    
    def test_error_handling(self):
        """Test that errors are properly raised."""
        with pytest.raises(ValueError, match="expected error message"):
            ClassName(invalid_param)
    
    @pytest.mark.parametrize("input,expected", [
        (input1, expected1),
        (input2, expected2),
    ])
    def test_parametrized(self, input, expected):
        """Test with multiple inputs."""
        result = ClassName(input).method()
        assert result == expected
```

## Type Hints and Stubs

### Generate .pyi Files
```python
# In .pyi stub files
from typing import Optional, List, Union

class ClassName:
    def __init__(self, param: str) -> None: ...
    @property
    def property_name(self) -> float: ...
    def method(self, param: int) -> Optional[str]: ...
    @classmethod
    def from_alternative(cls, data: List[float]) -> 'ClassName': ...
```

## Import Structure

### Recommended Import Pattern
```python
# Top-level imports for common types
from rfin import Currency, Money, Date

# Submodule imports for specialized functionality
from rfin.dates import Calendar, DayCount, Frequency
from rfin.cashflow import FixedRateLeg, CashFlow

# Function imports
from rfin.dates import generate_schedule, available_calendars
```

## Versioning and Compatibility

### Version Exposure
```rust
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // ...
}
```

### Deprecation Pattern
```rust
/// Deprecated: Use new_method instead.
///
/// .. deprecated:: 0.4.0
///    Use :meth:`new_method` instead.
#[pyo3(text_signature = "...")]
fn old_method(&self) -> PyResult<()> {
    PyErr::warn(
        py,
        PyDeprecationWarning::type_object(py),
        "old_method is deprecated, use new_method instead",
        1
    )?;
    self.new_method()
}
```

## Memory Management

### Clone vs Reference
```rust
// Prefer cloning for small Copy types
#[getter]
fn currency(&self) -> PyCurrency {
    PyCurrency::from_inner(self.inner.currency())
}

// Use Arc for large shared data
#[pyclass]
struct PyLargeData {
    inner: Arc<LargeData>,
}
```

### Lifetime Considerations
```rust
// Be explicit about Python lifetime when storing references
#[pyclass]
struct PyWrapper {
    data: Py<PyAny>,
}

impl PyWrapper {
    fn new(py: Python, data: PyObject) -> Self {
        Self { data: data.into() }
    }
}
``` 