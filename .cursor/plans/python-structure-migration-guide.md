# Python Bindings Structure Migration Guide

## Overview

This guide provides step-by-step instructions for migrating the Python bindings to match the Rust crate structure. It includes concrete examples, file templates, and validation steps.

---

## Quick Reference: Module Mapping

| Rust Path | Current Python | Target Python |
|-----------|---------------|---------------|
| `valuations/src/instruments/bond/` | `src/valuations/instruments/bond.rs` | `src/valuations/instruments/bond/` |
| `valuations/src/calibration/methods/` | `src/valuations/calibration/methods.rs` | `src/valuations/calibration/methods/` |
| `statements/src/capital_structure/` | *(missing)* | `src/statements/capital_structure/` |
| `statements/src/dsl/` | *(missing)* | `src/statements/dsl/` |

---

## Part 1: Valuations - Instruments Restructuring

### Example: Bond Instrument Migration

#### Current Structure
```
finstack-py/src/valuations/instruments/
└── bond.rs                 (all bond bindings in one file)

finstack-py/finstack/valuations/instruments/
└── bond.pyi                (all bond stubs in one file)
```

#### Target Structure
```
finstack-py/src/valuations/instruments/bond/
├── mod.rs                  (module root, re-exports)
├── types.rs                (Bond struct binding)
├── cashflows.rs            (cashflow generation bindings)
├── metrics.rs              (bond-specific metrics)
└── pricing.rs              (pricing method bindings)

finstack-py/finstack/valuations/instruments/bond/
├── __init__.pyi            (re-exports)
├── types.pyi               (Bond type stubs)
├── cashflows.pyi
├── metrics.pyi
└── pricing.pyi
```

#### Step-by-Step Migration

##### Step 1: Create Directory Structure
```bash
mkdir -p finstack-py/src/valuations/instruments/bond
mkdir -p finstack-py/finstack/valuations/instruments/bond
```

##### Step 2: Split `bond.rs` into Submodules

**Original:** `src/valuations/instruments/bond.rs` (example content)
```rust
use pyo3::prelude::*;

#[pyclass(name = "Bond")]
pub struct PyBond { /* ... */ }

#[pymethods]
impl PyBond {
    #[new]
    pub fn new(/* ... */) -> Self { /* ... */ }
    
    pub fn generate_cashflows(&self) -> PyResult<Vec<PyCashFlow>> { /* ... */ }
    
    pub fn calculate_yield(&self) -> PyResult<f64> { /* ... */ }
}

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    // ... registration logic
}
```

**New:** `src/valuations/instruments/bond/types.rs`
```rust
use pyo3::prelude::*;
use finstack_valuations::instruments::Bond;

/// Python wrapper for the Bond instrument.
#[pyclass(name = "Bond", module = "finstack.valuations.instruments.bond")]
#[derive(Clone)]
pub struct PyBond {
    pub(crate) inner: Bond,
}

#[pymethods]
impl PyBond {
    #[new]
    #[pyo3(signature = (
        instrument_id,
        notional,
        coupon_rate,
        issue_date,
        maturity_date,
        discount_curve_id,
        // ... other args
    ))]
    pub fn new(
        instrument_id: &str,
        notional: PyMoney,
        coupon_rate: f64,
        // ... other params
    ) -> PyResult<Self> {
        // Implementation
        Ok(PyBond { inner: /* ... */ })
    }
}

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyBond>()?;
    Ok(vec!["Bond"])
}
```

**New:** `src/valuations/instruments/bond/cashflows.rs`
```rust
use pyo3::prelude::*;
use super::PyBond;

#[pymethods]
impl PyBond {
    /// Generate the bond's cashflow schedule.
    pub fn generate_cashflows(&self, py: Python<'_>) -> PyResult<Vec<PyCashFlow>> {
        // Implementation
    }
}

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    // Cashflow-related functions if any
    Ok(vec![])
}
```

**New:** `src/valuations/instruments/bond/metrics.rs`
```rust
use pyo3::prelude::*;
use super::PyBond;

#[pymethods]
impl PyBond {
    /// Calculate yield to maturity.
    pub fn calculate_yield(&self, /* ... */) -> PyResult<f64> {
        // Implementation
    }
    
    /// Calculate duration.
    pub fn calculate_duration(&self, /* ... */) -> PyResult<f64> {
        // Implementation
    }
}

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    // Metric-related functions if any
    Ok(vec![])
}
```

**New:** `src/valuations/instruments/bond/mod.rs`
```rust
//! Bond instrument Python bindings.
//!
//! This module mirrors the structure of `finstack_valuations::instruments::bond`.

mod types;
mod cashflows;
mod metrics;
mod pricing;

use pyo3::prelude::*;

// Re-export the main type
pub use types::PyBond;

/// Register the bond submodule with Python.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let bond_module = PyModule::new(py, "bond")?;
    bond_module.setattr(
        "__doc__",
        "Bond instrument types, cashflows, and pricing methods.",
    )?;
    
    let mut exports = Vec::new();
    
    // Register types
    exports.extend(types::register(&bond_module)?);
    
    // Register other submodules
    exports.extend(cashflows::register(&bond_module)?);
    exports.extend(metrics::register(&bond_module)?);
    exports.extend(pricing::register(&bond_module)?);
    
    // Add as submodule
    parent.add_submodule(&bond_module)?;
    parent.setattr("bond", &bond_module)?;
    
    Ok(exports)
}

// For backward compatibility: re-export Bond at instruments level
pub(crate) fn register_legacy(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyBond>()?;
    Ok(())
}
```

##### Step 3: Update `instruments/mod.rs`

**Before:**
```rust
pub(crate) mod bond;
pub(crate) mod cds;
// ... other instruments

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "instruments")?;
    
    // Register each instrument directly
    module.add_class::<bond::PyBond>()?;
    module.add_class::<cds::PyCDS>()?;
    // ...
    
    parent.add_submodule(&module)?;
    Ok(exports)
}
```

**After:**
```rust
pub(crate) mod bond;
pub(crate) mod cds;
// ... other instruments

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "instruments")?;
    module.setattr(
        "__doc__",
        "Financial instrument definitions organized by type.",
    )?;
    
    let mut exports = Vec::new();
    
    // NEW: Register each instrument as a submodule
    exports.extend(bond::register(py, &module)?);
    exports.extend(cds::register(py, &module)?);
    // ... etc
    
    // BACKWARD COMPATIBILITY: Also expose classes at instruments level
    bond::register_legacy(&module)?;
    cds::register_legacy(&module)?;
    // ... etc
    
    parent.add_submodule(&module)?;
    parent.setattr("instruments", &module)?;
    
    Ok(exports)
}
```

##### Step 4: Create Python Stub Files

**New:** `finstack/valuations/instruments/bond/__init__.pyi`
```python
"""Bond instrument types and methods."""

from .types import Bond
from . import cashflows
from . import metrics
from . import pricing

__all__ = ["Bond", "cashflows", "metrics", "pricing"]
```

**New:** `finstack/valuations/instruments/bond/types.pyi`
```python
"""Bond type definitions."""

from finstack.core.money import Money
from finstack.core.dates import FsDate
from typing import Optional

class Bond:
    """Fixed income bond instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        coupon_rate: float,
        issue_date: FsDate,
        maturity_date: FsDate,
        discount_curve_id: str,
        *,
        frequency: Optional[str] = None,
        day_count: Optional[str] = None,
    ) -> None: ...
    
    @property
    def instrument_id(self) -> str: ...
    
    @property
    def notional(self) -> Money: ...
    
    # ... other properties
```

**New:** `finstack/valuations/instruments/bond/cashflows.pyi`
```python
"""Bond cashflow generation methods."""

# This can be empty if all methods are on the Bond class
```

**New:** `finstack/valuations/instruments/bond/metrics.pyi`
```python
"""Bond metric calculation methods."""

# This can be empty if all methods are on the Bond class
```

##### Step 5: Update Top-Level `instruments/__init__.pyi`

**Before:**
```python
from finstack.valuations.instruments.bond import Bond
from finstack.valuations.instruments.cds import CreditDefaultSwap
# ... all instruments

__all__ = ["Bond", "CreditDefaultSwap", ...]
```

**After (with backward compat):**
```python
"""Financial instruments module."""

# NEW: Import submodules
from . import bond
from . import cds
# ... other instrument submodules

# BACKWARD COMPATIBILITY: Re-export main classes
from .bond import Bond
from .cds import CreditDefaultSwap
# ... etc

__all__ = [
    # Submodules
    "bond",
    "cds",
    # ... etc
    
    # Classes (for backward compat)
    "Bond",
    "CreditDefaultSwap",
    # ... etc
]
```

##### Step 6: Validation

Run these commands to validate the migration:

```bash
# Build Python bindings
cd finstack-py
maturin develop --release

# Test both import styles work
python -c "from finstack.valuations.instruments.bond import Bond; print('✓ New import works')"
python -c "from finstack.valuations.instruments import Bond; print('✓ Legacy import works')"

# Run tests
uv run pytest tests/

# Check stubs
uv run mypy finstack/ --strict
```

---

## Part 2: Calibration Module Restructuring

### Example: Methods Submodule

#### Current Structure
```
finstack-py/src/valuations/calibration/
└── methods.rs              (all calibration methods)
```

#### Target Structure
```
finstack-py/src/valuations/calibration/methods/
├── mod.rs
├── discount.rs             (discount curve calibration)
├── forward_curve.rs        (forward curve calibration)
├── hazard_curve.rs         (hazard curve calibration)
└── sabr_surface.rs         (SABR surface calibration)
```

#### Migration Steps

##### Step 1: Create Submodule Structure

```bash
mkdir -p finstack-py/src/valuations/calibration/methods
mkdir -p finstack-py/finstack/valuations/calibration/methods
```

##### Step 2: Split `methods.rs`

**New:** `src/valuations/calibration/methods/discount.rs`
```rust
use pyo3::prelude::*;
use finstack_valuations::calibration::methods::bootstrap_discount_curve;

#[pyfunction]
#[pyo3(name = "bootstrap_discount_curve")]
pub fn py_bootstrap_discount_curve(
    // ... parameters
) -> PyResult<PyDiscountCurve> {
    // Implementation
}

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    parent.add_function(wrap_pyfunction!(py_bootstrap_discount_curve, parent)?)?;
    Ok(vec!["bootstrap_discount_curve"])
}
```

**New:** `src/valuations/calibration/methods/mod.rs`
```rust
//! Calibration methods submodule.

mod discount;
mod forward_curve;
mod hazard_curve;
mod sabr_surface;

use pyo3::prelude::*;

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let methods_module = PyModule::new(py, "methods")?;
    methods_module.setattr(
        "__doc__",
        "Calibration methods for curves and surfaces.",
    )?;
    
    let mut exports = Vec::new();
    exports.extend(discount::register(&methods_module)?);
    exports.extend(forward_curve::register(&methods_module)?);
    exports.extend(hazard_curve::register(&methods_module)?);
    exports.extend(sabr_surface::register(&methods_module)?);
    
    parent.add_submodule(&methods_module)?;
    parent.setattr("methods", &methods_module)?;
    
    Ok(exports)
}
```

##### Step 3: Update `calibration/mod.rs`

```rust
pub(crate) mod config;
pub(crate) mod methods;      // Now a directory
pub(crate) mod derivatives;  // NEW
pub(crate) mod quote;
pub(crate) mod report;
pub(crate) mod simple;
pub(crate) mod validation;

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let calibration_module = PyModule::new(py, "calibration")?;
    
    let mut exports = Vec::new();
    
    // Register submodules
    exports.extend(methods::register(py, &calibration_module)?);
    exports.extend(derivatives::register(py, &calibration_module)?);
    // ... other submodules
    
    parent.add_submodule(&calibration_module)?;
    Ok(exports)
}
```

---

## Part 3: Statements Module - Adding Missing Modules

### Adding `capital_structure` Module

#### Step 1: Create Module Structure

```bash
mkdir -p finstack-py/src/statements/capital_structure
mkdir -p finstack-py/finstack/statements/capital_structure
```

#### Step 2: Create Bindings

**New:** `src/statements/capital_structure/mod.rs`
```rust
//! Capital structure Python bindings.

mod builder;
mod integration;
mod types;

use pyo3::prelude::*;

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    let cap_structure_module = PyModule::new(py, "capital_structure")?;
    cap_structure_module.setattr(
        "__doc__",
        "Capital structure modeling and integration with financial statements.",
    )?;
    
    let mut exports = Vec::new();
    exports.extend(types::register(&cap_structure_module)?);
    exports.extend(builder::register(&cap_structure_module)?);
    exports.extend(integration::register(&cap_structure_module)?);
    
    parent.add_submodule(&cap_structure_module)?;
    parent.setattr("capital_structure", &cap_structure_module)?;
    
    Ok(exports)
}
```

**New:** `src/statements/capital_structure/types.rs`
```rust
use pyo3::prelude::*;
use finstack_statements::capital_structure::{CapitalStructure, Tranche};

#[pyclass(name = "CapitalStructure", module = "finstack.statements.capital_structure")]
pub struct PyCapitalStructure {
    pub(crate) inner: CapitalStructure,
}

#[pyclass(name = "Tranche", module = "finstack.statements.capital_structure")]
pub struct PyTranche {
    pub(crate) inner: Tranche,
}

// Implementation...

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyCapitalStructure>()?;
    parent.add_class::<PyTranche>()?;
    Ok(vec!["CapitalStructure", "Tranche"])
}
```

#### Step 3: Update Statements Module

**Update:** `src/statements/mod.rs`
```rust
pub(crate) mod builder;
pub(crate) mod capital_structure;  // NEW
pub(crate) mod dsl;                // NEW
pub(crate) mod evaluator;
pub(crate) mod extensions;
pub(crate) mod forecast;           // NEW
pub(crate) mod registry;
pub(crate) mod types;
pub(crate) mod utils;

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let statements_module = PyModule::new(py, "statements")?;
    
    // Register all submodules
    builder::register(py, &statements_module)?;
    capital_structure::register(py, &statements_module)?;  // NEW
    dsl::register(py, &statements_module)?;                // NEW
    evaluator::register(py, &statements_module)?;
    extensions::register(py, &statements_module)?;
    forecast::register(py, &statements_module)?;           // NEW
    registry::register(py, &statements_module)?;
    types::register(py, &statements_module)?;
    
    parent.add_submodule(&statements_module)?;
    parent.setattr("statements", &statements_module)?;
    
    Ok(())
}
```

#### Step 4: Create Stub Files

**New:** `finstack/statements/capital_structure/__init__.pyi`
```python
"""Capital structure modeling for financial statements."""

from .types import CapitalStructure, Tranche
from .builder import CapitalStructureBuilder
from .integration import integrate_capital_structure

__all__ = [
    "CapitalStructure",
    "Tranche",
    "CapitalStructureBuilder",
    "integrate_capital_structure",
]
```

---

## Part 4: Automated Refactoring Script

### Script: `scripts/migrate_python_structure.py`

```python
#!/usr/bin/env python3
"""
Automated migration script for Python bindings restructuring.

Usage:
    python scripts/migrate_python_structure.py --module valuations --instrument bond --dry-run
    python scripts/migrate_python_structure.py --module valuations --instrument bond --execute
"""

import argparse
import shutil
from pathlib import Path
import re


def migrate_instrument(module: str, instrument: str, dry_run: bool = True):
    """Migrate a single instrument to nested structure."""
    
    src_base = Path("finstack-py/src") / module / "instruments"
    stub_base = Path("finstack-py/finstack") / module / "instruments"
    
    src_file = src_base / f"{instrument}.rs"
    src_dir = src_base / instrument
    stub_file = stub_base / f"{instrument}.pyi"
    stub_dir = stub_base / instrument
    
    if not src_file.exists():
        print(f"❌ Source file {src_file} not found")
        return
    
    print(f"{'[DRY RUN] ' if dry_run else ''}Migrating {instrument}...")
    
    # Create directories
    if not dry_run:
        src_dir.mkdir(exist_ok=True)
        stub_dir.mkdir(exist_ok=True)
    else:
        print(f"  Would create: {src_dir}")
        print(f"  Would create: {stub_dir}")
    
    # Read original file
    with open(src_file, 'r') as f:
        content = f.read()
    
    # Parse content and split into sections
    # This is a simplified example - real implementation would be more sophisticated
    sections = {
        'types': [],
        'cashflows': [],
        'metrics': [],
        'pricing': [],
    }
    
    # Basic heuristic: split by #[pymethods] blocks and function names
    # In practice, you'd want more sophisticated parsing
    
    # Create mod.rs
    mod_content = f'''//! {instrument.title()} instrument Python bindings.

mod types;
mod cashflows;
mod metrics;
mod pricing;

use pyo3::prelude::*;
pub use types::Py{instrument.title()};

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {{
    let module = PyModule::new(py, "{instrument}")?;
    
    let mut exports = Vec::new();
    exports.extend(types::register(&module)?);
    exports.extend(cashflows::register(&module)?);
    exports.extend(metrics::register(&module)?);
    exports.extend(pricing::register(&module)?);
    
    parent.add_submodule(&module)?;
    parent.setattr("{instrument}", &module)?;
    
    Ok(exports)
}}

pub(crate) fn register_legacy(parent: &Bound<'_, PyModule>) -> PyResult<()> {{
    parent.add_class::<Py{instrument.title()}>()?;
    Ok(())
}}
'''
    
    if not dry_run:
        (src_dir / "mod.rs").write_text(mod_content)
        # TODO: Split content into types.rs, cashflows.rs, etc.
        # This requires AST parsing or sophisticated regex
    else:
        print(f"  Would create: {src_dir}/mod.rs")
        print(f"  Would split {src_file} into submodules")
    
    # Create stub files
    stub_init_content = f'''"""
{instrument.title()} instrument types and methods.
"""

from .types import {instrument.title()}

__all__ = ["{instrument.title()}"]
'''
    
    if not dry_run:
        (stub_dir / "__init__.pyi").write_text(stub_init_content)
        # TODO: Split stub content
    else:
        print(f"  Would create: {stub_dir}/__init__.pyi")
        print(f"  Would split {stub_file} into submodules")
    
    print(f"✅ Migration {'would be' if dry_run else 'is'} complete for {instrument}")


def main():
    parser = argparse.ArgumentParser(description="Migrate Python bindings structure")
    parser.add_argument("--module", required=True, choices=["valuations", "statements"])
    parser.add_argument("--instrument", required=True, help="Instrument name (e.g., bond, cds)")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be done")
    parser.add_argument("--execute", action="store_true", help="Actually perform migration")
    
    args = parser.parse_args()
    
    if not args.dry_run and not args.execute:
        print("❌ Must specify either --dry-run or --execute")
        return
    
    migrate_instrument(args.module, args.instrument, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
```

---

## Part 5: Testing Strategy

### Test Checklist for Each Module Migration

```bash
#!/bin/bash
# scripts/validate_migration.sh

MODULE=$1  # e.g., "bond"
PARENT=$2  # e.g., "valuations.instruments"

echo "Validating migration of ${PARENT}.${MODULE}..."

# 1. Build succeeds
echo "→ Building..."
cd finstack-py
maturin develop --release || exit 1

# 2. New import path works
echo "→ Testing new import path..."
python -c "from finstack.${PARENT}.${MODULE} import *" || exit 1

# 3. Legacy import path works (if maintaining backward compat)
echo "→ Testing legacy import path..."
python -c "from finstack.${PARENT} import ${MODULE^}" || exit 1

# 4. Stubs are valid
echo "→ Checking type stubs..."
uv run mypy -c "from finstack.${PARENT}.${MODULE} import *" || exit 1

# 5. Tests pass
echo "→ Running tests..."
uv run pytest tests/ -k ${MODULE} || exit 1

# 6. No performance regression
echo "→ Running benchmarks..."
# TODO: Add benchmark comparison

echo "✅ Migration validated for ${MODULE}"
```

---

## Part 6: Rollback Strategy

### If Migration Fails

1. **Git branches**: Do all work in feature branches
   ```bash
   git checkout -b feat/migrate-bond-structure
   # ... make changes
   git checkout -b feat/migrate-cds-structure
   # ... etc
   ```

2. **Revert script**:
   ```bash
   #!/bin/bash
   # scripts/rollback_migration.sh
   
   INSTRUMENT=$1
   
   echo "Rolling back migration for ${INSTRUMENT}..."
   
   # Restore original files from git
   git checkout HEAD -- \
     finstack-py/src/valuations/instruments/${INSTRUMENT}.rs \
     finstack-py/finstack/valuations/instruments/${INSTRUMENT}.pyi
   
   # Remove new directories
   rm -rf finstack-py/src/valuations/instruments/${INSTRUMENT}/
   rm -rf finstack-py/finstack/valuations/instruments/${INSTRUMENT}/
   
   echo "✅ Rollback complete"
   ```

---

## Part 7: Documentation Updates

### Update Examples

**Before:**
```python
# examples/scripts/bond_pricing.py
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import CalibrationConfig
```

**After:**
```python
# examples/scripts/bond_pricing.py

# New structured imports (recommended)
from finstack.valuations.instruments.bond import Bond
from finstack.valuations.calibration.config import CalibrationConfig

# Legacy imports (deprecated, will be removed in v2.0)
# from finstack.valuations.instruments import Bond
# from finstack.valuations.calibration import CalibrationConfig
```

### Update README

Add migration guide section:

```markdown
## Import Path Changes (v1.5+)

Starting in v1.5, we've restructured the Python bindings to mirror the Rust crate
organization more closely. This provides better discoverability and consistency.

### New Import Paths

```python
# Instruments - now organized by type
from finstack.valuations.instruments.bond import Bond, BondMetrics
from finstack.valuations.instruments.cds import CreditDefaultSwap

# Calibration - now organized by method type
from finstack.valuations.calibration.methods import DiscountCurveCalibrator
from finstack.valuations.calibration.derivatives import SABRDerivatives
```

### Backward Compatibility

Legacy import paths are still supported but deprecated:

```python
# Still works, but will be removed in v2.0
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import CalibrationConfig
```

To update your code, run:
```bash
python scripts/migrate_imports.py --path your_code/
```
```

---

## Part 8: CI/CD Integration

### Add Structure Validation to CI

```yaml
# .github/workflows/python-structure-check.yml
name: Python Structure Validation

on: [push, pull_request]

jobs:
  validate-structure:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.12'
      
      - name: Install dependencies
        run: |
          pip install uv
          uv pip install mypy pytest
      
      - name: Build Python bindings
        run: |
          cd finstack-py
          pip install maturin
          maturin develop --release
      
      - name: Validate module structure
        run: |
          python scripts/validate_module_structure.py
      
      - name: Check type stubs
        run: |
          cd finstack-py
          mypy finstack/ --strict
      
      - name: Run import tests
        run: |
          python scripts/test_all_import_paths.py
```

---

## Appendix: File Templates

### Template: `mod.rs` for Instrument

```rust
//! {{ INSTRUMENT_NAME }} instrument Python bindings.
//!
//! Mirrors the structure of `finstack_valuations::instruments::{{ instrument_name }}`.

mod types;
{% if has_cashflows %}mod cashflows;{% endif %}
{% if has_metrics %}mod metrics;{% endif %}
{% if has_pricing %}mod pricing;{% endif %}

use pyo3::prelude::*;

pub use types::Py{{ InstrumentName }};

/// Register the {{ instrument_name }} submodule.
pub(crate) fn register(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "{{ instrument_name }}")?;
    module.setattr(
        "__doc__",
        "{{ INSTRUMENT_NAME }} instrument types and methods.",
    )?;
    
    let mut exports = Vec::new();
    exports.extend(types::register(&module)?);
    {% if has_cashflows %}exports.extend(cashflows::register(&module)?);{% endif %}
    {% if has_metrics %}exports.extend(metrics::register(&module)?);{% endif %}
    {% if has_pricing %}exports.extend(pricing::register(&module)?);{% endif %}
    
    parent.add_submodule(&module)?;
    parent.setattr("{{ instrument_name }}", &module)?;
    
    Ok(exports)
}

/// Register at parent level for backward compatibility.
#[allow(dead_code)]
pub(crate) fn register_legacy(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<Py{{ InstrumentName }}>()?;
    Ok(())
}
```

### Template: `__init__.pyi` for Instrument

```python
"""
{{ INSTRUMENT_NAME }} instrument types and methods.

This module provides Python bindings for the {{ instrument_name }} instrument
from the Rust `finstack_valuations` crate.
"""

from .types import {{ InstrumentName }}
{% if has_metrics %}from . import metrics{% endif %}
{% if has_cashflows %}from . import cashflows{% endif %}
{% if has_pricing %}from . import pricing{% endif %}

__all__ = [
    "{{ InstrumentName }}",
    {% if has_metrics %}"metrics",{% endif %}
    {% if has_cashflows %}"cashflows",{% endif %}
    {% if has_pricing %}"pricing",{% endif %}
]
```

---

## Summary Checklist

- [ ] Plan reviewed and approved
- [ ] Migration script created and tested
- [ ] Rollback procedure documented
- [ ] Test suite updated
- [ ] CI/CD validation added
- [ ] Documentation updated
- [ ] Examples updated
- [ ] Backward compatibility maintained
- [ ] Performance benchmarks run
- [ ] Team trained on new structure

---

**Document Version:** 1.0  
**Created:** 2025-11-03  
**Last Updated:** 2025-11-03  
**Status:** Draft - Implementation Guide

