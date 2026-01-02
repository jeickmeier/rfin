# Stub maintenance (Python `.pyi`)

Finstack’s Python API is implemented in Rust (PyO3). For a great IDE experience
in VS Code + Pylance/pyright, we ship **hand-maintained** type stubs under
`finstack-py/finstack/**/*.pyi`.

This document explains how to keep those stubs correct and useful.

## Principles

- **Stubs are the IDE contract**: autocomplete, hover docs, and call signatures
  should come from `.pyi` files, not from runtime introspection.
- **Runtime stays authoritative for behavior**: parity tests ensure Python and
  Rust compute the same results.
- **Stubs must not drift**: signature mismatches should be caught automatically
  (pyright + stubtest).

## What to update when Rust changes

Update stubs after any change to:

- Exported classes/functions in `finstack-py/src/**`
- Method signatures (`#[pyo3(signature = ...)]`, `#[pyo3(text_signature = ...)]`)
- Module structure / re-exports (`register(...)`, `m.add_submodule(...)`, `__all__`)

### Common changes

- **New class or function**: add to the relevant `.pyi` module and to its
  `__all__` list if present.
- **Renamed/moved symbol**: update import paths in stubs and examples.
- **Signature change**: update parameter names, defaults, overloads, and return
  types in the stub.
- **Docstring updates**: keep examples runnable; they are tested.

## Docstrings in stubs (Pylance gotcha)

To show hover docs in VS Code/Pylance, docstrings must be **inside** the
function body in `.pyi`:

```python
def foo(x: int) -> int:
    """Adds one."""
    ...
```

This is **not** attached and will not show in hovers:

```python
def foo(x: int) -> int: ...
"""Adds one."""
```

If you suspect drift here, run:

```bash
uv run python finstack-py/tools/fix_pyi_docstrings.py finstack-py/finstack
```

## Verification (recommended)

These checks are intended to run in CI and locally:

- **pyright**: ensures the shipped stubs form a coherent public API for users.
- **mypy stubtest**: compares runtime objects against stubs to catch drift.

See also:
- `finstack-py/tests/test_doc_examples.py` (runs `>>>` examples extracted from `.pyi` docstrings)
- `finstack-py/tests/parity/` (behavior parity tests)
