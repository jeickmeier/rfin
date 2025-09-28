#!/usr/bin/env bash
# -----------------------------------------------------------------------------
# Regenerate *.pyi stub files for the finstack Python bindings.
#
# 1. Ensures dev dependencies are installed via uv (maturin, pyo3-stubgen).
# 2. Builds the finstack extension into the current uv environment (release mode)
#    so that the compiled module can be imported by `pyo3-stubgen`.
# 3. Invokes `pyo3-stubgen` to generate fresh stub files.
# 3. Copies / syncs those stubs into the finstack-py package tree and ensures
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

PACKAGE_CRATE_PATH="finstack-py"
PY_PACKAGE_DIR="$PACKAGE_CRATE_PATH/finstack"

# Ensure uv dev deps are installed (maturin, pyo3-stubgen)
if command -v uv >/dev/null 2>&1; then
    echo "ℹ️  Ensuring dev dependencies via uv …"
    uv sync --dev >/dev/null
else
    echo "⚠️  uv not found. Please install uv or activate an environment with maturin and pyo3-stubgen."
fi

# 1. Re-install the extension into the current uv environment (release build)
if command -v uv >/dev/null 2>&1; then
    uv run maturin develop --release -m "$PACKAGE_CRATE_PATH/Cargo.toml"
else
    # Fallback to python -m maturin if uv is unavailable
    pushd "$PACKAGE_CRATE_PATH" >/dev/null
    python -m maturin develop --release
    popd >/dev/null
fi

echo "✅  Rust extension built. Generating stubs …"

# 2. Generate fresh stubs into a temporary directory
TMP_STUB_DIR=$(mktemp -d)

MODULES=(
  "finstack"
  "finstack.core"
  "finstack.core.currency"
  "finstack.core.config"
  "finstack.core.cashflow"
  "finstack.core.money"
  "finstack.core.dates"
  "finstack.core.market_data"
  "finstack.core.market_data.term_structures"
  "finstack.core.market_data.fx"
  "finstack.core.market_data.scalars"
  "finstack.core.market_data.surfaces"
  "finstack.core.market_data.dividends"
  "finstack.core.market_data.context"
  "finstack.core.market_data.interp"
  "finstack.core.math"
  "finstack.core.expr"
)

for MOD in "${MODULES[@]}"; do
  if command -v uv >/dev/null 2>&1; then
    uv run pyo3-stubgen "$MOD" "$TMP_STUB_DIR"
  else
    pyo3-stubgen "$MOD" "$TMP_STUB_DIR"
  fi
done

# 3. Copy stubs
# Root stub (__init__.pyi)
if [ -f "$TMP_STUB_DIR/finstack.pyi" ]; then
  cp "$TMP_STUB_DIR/finstack.pyi" "$PY_PACKAGE_DIR/__init__.pyi"
fi

# Package sub-stubs – mirror directory structure to capture nested modules
if [ -d "$TMP_STUB_DIR/finstack" ]; then
  find "$TMP_STUB_DIR/finstack" -name "*.pyi" -print0 |
    while IFS= read -r -d '' FILE; do
      REL_PATH=${FILE#"$TMP_STUB_DIR/finstack/"}
      DEST_DIR="$PY_PACKAGE_DIR/$(dirname "$REL_PATH")"
      mkdir -p "$DEST_DIR"
      cp "$FILE" "$DEST_DIR/$(basename "$FILE")"
    done
fi

# Ensure PEP 561 marker
touch "$PY_PACKAGE_DIR/py.typed"

rm -rf "$TMP_STUB_DIR"

echo "✨  Stub files updated in $PACKAGE_CRATE_PATH"
