#!/usr/bin/env bash
# -----------------------------------------------------------------------------
# Regenerate *.pyi stub files for the rfin Python bindings.
#
# 1. Builds the rfin extension into the current virtual environment (release
#    mode) so that the compiled module can be imported by `pyo3-stubgen`.
# 2. Invokes `pyo3-stubgen` to generate fresh stub files.
# 3. Copies / syncs those stubs into the rfin-python package tree and ensures
#    the mandatory `py.typed` marker exists so that type checkers recognise the
#    package as "typed" (PEP 561).
#
# Run this script from the project root, *after activating your virtualenv*.
# -----------------------------------------------------------------------------
set -euo pipefail

# Make sure we run from project root regardless of invocation place
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
PROJECT_ROOT="$SCRIPT_DIR/.."
cd "$PROJECT_ROOT"

PACKAGE_CRATE_PATH="rfin-python"
PY_PACKAGE_DIR="$PACKAGE_CRATE_PATH/rfin"

# Ensure pyo3-stubgen is available
if ! command -v pyo3-stubgen >/dev/null 2>&1; then
    echo "ℹ️  pyo3-stubgen not found – installing into current environment …"
    pip install --quiet --upgrade pyo3-stubgen
fi

# 1. Re-install the extension into the current venv (release build)
#    This guarantees that the just-compiled shared library is importable so that
#    pyo3-stubgen can introspect it. We run maturin from within the crate
#    directory to avoid the unrelated top-level pyproject.toml (which does not
#    contain a `[project]` table and would make maturin abort).
pushd "$PACKAGE_CRATE_PATH" >/dev/null
python -m maturin develop --release
popd >/dev/null

echo "✅  Rust extension built. Generating stubs …"

# 2. Generate fresh stubs into a temporary directory
TMP_STUB_DIR=$(mktemp -d)
pyo3-stubgen rfin "$TMP_STUB_DIR"

# 3. Copy stubs
# Root stub (rfin.pyi)
cp "$TMP_STUB_DIR/rfin.pyi" "$PACKAGE_CRATE_PATH/"

# Package sub-stubs – use rsync to mirror directory structure
if [ -d "$TMP_STUB_DIR/rfin" ]; then
  rsync -a --delete "$TMP_STUB_DIR/rfin/" "$PY_PACKAGE_DIR/"
fi

# Ensure PEP 561 marker
touch "$PY_PACKAGE_DIR/py.typed"

echo "✨  Stub files updated in $PACKAGE_CRATE_PATH" 