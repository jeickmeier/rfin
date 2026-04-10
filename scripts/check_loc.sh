#!/usr/bin/env bash
# Check for source files exceeding the LOC limit (default: 1000)
#
# Usage:
#   ./scripts/check_loc.sh          # list files > 1000 LOC
#   ./scripts/check_loc.sh 500      # list files > 500 LOC
#   ./scripts/check_loc.sh 1000 --ci # exit 1 if any violations found (for CI)
#
# Rust inline tests such as #[test], #[tokio::test], and #[cfg(test)] mod
# tests are excluded from the reported line counts.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PYTHON_BIN="$ROOT/.venv/bin/python"

if [ ! -x "$PYTHON_BIN" ]; then
	PYTHON_BIN="python3"
fi

exec "$PYTHON_BIN" "$ROOT/scripts/check_loc.py" "$@"
