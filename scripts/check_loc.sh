#!/usr/bin/env bash
# Check for source files exceeding the LOC limit (default: 1000)
#
# Usage:
#   ./scripts/check_loc.sh          # list files > 1000 LOC
#   ./scripts/check_loc.sh 500      # list files > 500 LOC
#   ./scripts/check_loc.sh 1000 --ci # exit 1 if any violations found (for CI)

set -euo pipefail

LIMIT="${1:-1000}"
CI_MODE="${2:-}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

VIOLATIONS=$(
  find "$ROOT" -type f \( -name "*.rs" -o -name "*.py" -o -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \) \
    ! -path "*/target/*" \
    ! -path "*/node_modules/*" \
    ! -path "*/.git/*" \
    ! -path "*/dist/*" \
    ! -path "*/build/*" \
    ! -path "*/.venv/*" \
    ! -path "*/vendor/*" \
    ! -path "*/pkg/*" \
    ! -path "*/pkg-node/*" \
    ! -path "*/examples/*" \
    ! -path "*/scripts/*" \
    ! -path "*/notebooks/*" \
    -exec wc -l {} + \
  | awk -v limit="$LIMIT" -v root="$ROOT/" '$1 > limit && !/total$/ { sub(root, "", $2); printf "%6d  %s\n", $1, $2 }' \
  | sort -rn
)

COUNT=$(echo "$VIOLATIONS" | grep -c '[^ ]' || true)

if [ "$COUNT" -eq 0 ]; then
  echo "All source files are within the ${LIMIT}-line limit."
  exit 0
fi

echo "Found ${COUNT} file(s) exceeding ${LIMIT} lines:"
echo ""
printf "%6s  %s\n" "LINES" "FILE"
printf "%6s  %s\n" "-----" "----"
echo "$VIOLATIONS"

if [ "$CI_MODE" = "--ci" ]; then
  exit 1
fi
