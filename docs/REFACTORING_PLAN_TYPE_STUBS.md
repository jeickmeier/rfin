# Type Stub Integration Strategy

## Current Issues

- Type stubs exist but don't match dynamic module behavior
- IDE can't provide accurate completions
- Type checkers fail on dynamically registered modules

## Proposed Solution

### 1. Static Type Stub Generation

```python
# New build script: /finstack-py/scripts/generate_type_stubs.py
"""Generate type stubs that match actual runtime structure."""

import subprocess
import sys
from pathlib import Path

def generate_stubs():
    """Generate type stubs from the actual built module."""
    # Use stubgen from mypy to generate initial stubs
    subprocess.run([
        sys.executable, "-m", "mypy.stubgen",
        "--include-private",
        "--output", "finstack",
        "finstack"
    ])

    # Post-process to fix dynamic imports
    fix_dynamic_imports()

def fix_dynamic_imports():
    """Fix stubs to match static module structure."""
    # Replace dynamic registrations with explicit imports
    # Add proper type annotations for wrapped methods
    pass
```

### 2. Synchronized Module Structure

```python
# Ensure runtime modules match stub structure
# /finstack-py/finstack/core/__init__.py (runtime)

# Must match /finstack-py/finstack/core/__init__.pyi (stubs)

# Runtime
from finstack import finstack as _finstack
_rust_core = _finstack.core

# Explicit exports that match stubs
currency = _rust_core.currency
money = _rust_core.money

# Stub file
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from . import currency as currency
    from . import money as money
```

### 3. Validation CI Check

```yaml
# .github/workflows/validate-types.yml
name: Validate Type Stubs

on: [push, pull_request]

jobs:
  validate-types:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: "3.11"

      - name: Install dependencies
        run: |
          pip install -e .
          pip install mypy

      - name: Generate stubs from runtime
        run: python scripts/generate_type_stubs.py

      - name: Check stubs match
        run: |
          # Compare generated stubs with checked-in stubs
          python scripts/compare_stubs.py

      - name: Type check with mypy
        run: mypy finstack --strict
```

### 4. IDE Integration

```json
// .vscode/settings.json
{
    "python.analysis.typeCheckingMode": "strict",
    "python.analysis.autoImportCompletions": true,
    "python.analysis.stubPath": "./finstack-py",
    "python.linting.mypyEnabled": true,
    "python.linting.enabled": true
}
```

## Implementation Steps

1. Generate initial stubs from built module
2. Manually curate stubs for complex cases
3. Add stub generation to build process
4. Add CI validation
5. Update all examples to pass type checking

## Benefits

- Full IDE autocomplete support
- Catch type errors before runtime
- Better documentation generation
- Improved developer experience
