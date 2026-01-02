#!/bin/bash
# Script to run all Rust examples with categorization

set -e

# Colors for output
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo -e "🚀 Running all Rust examples"
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo ""

if ! command -v jq >/dev/null 2>&1; then
    echo "❌ jq is required but not installed. Install with: brew install jq"
    exit 1
fi

# Get list of examples from cargo metadata
example_list=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "finstack") | .targets[] | select(.kind[] == "example") | .name')

last_category=""

for example in $example_list; do
    category="Valuations"
    if [[ "$example" =~ ^market_context ]]; then
        category="Core"
    elif [[ "$example" =~ portfolio ]]; then
        category="Portfolio"
    elif [[ "$example" =~ scenario ]]; then
        category="Scenarios"
    elif [[ "$example" =~ ^(statements|capital_structure|lbo_) ]]; then
        category="Statements"
    fi

    if [[ "$category" != "$last_category" ]]; then
        echo ""
        echo -e "📋 ${BLUE}$category Examples${NC}"
        echo "────────────────────────────────────────────────────────────────"
        last_category="$category"
    fi

    echo "Running $example..."
    CARGO_INCREMENTAL=1 cargo run --example "$example" --all-features || exit 1
    echo ""
done

echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo -e "🎉 All examples completed successfully!"
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
