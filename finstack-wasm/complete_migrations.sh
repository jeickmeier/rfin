#!/bin/bash
# Semi-automated completion script for remaining instrument migrations
# This handles the mechanical parts; struct/impl changes require manual editing

set -e

INST_DIR="finstack-wasm/src/valuations/instruments"

# List of remaining files
REMAINING=(
    "convertible.rs"
    "ir_future.rs"
    "irs.rs"
    "fra.rs"
    "basis_swap.rs"
    "cap_floor.rs"
    "swaption.rs"
    "equity_option.rs"
    "cds.rs"
    "cds_index.rs"
    "inflation_swap.rs"
    "fx.rs"
    "cds_tranche.rs"
    "cds_option.rs"
    "structured.rs"
    "private_markets_fund.rs"
    "trs.rs"
)

echo "=== Instrument Wrapper Migration - Automated Helper ==="
echo ""
echo "This script will perform the automated parts of migration:"
echo "  1. Add InstrumentWrapper import (if missing)"
echo "  2. Replace all self.inner with self.0"
echo ""
echo "You must MANUALLY:"
echo "  - Change struct definition: { inner: X } → (X)"
echo "  - Replace impl block with trait impl"
echo ""
echo "Press ENTER to continue, or Ctrl+C to cancel..."
read

for file in "${REMAINING[@]}"; do
    filepath="$INST_DIR/$file"
    
    if [ ! -f "$filepath" ]; then
        echo "⚠️  File not found: $file"
        continue
    fi
    
    echo "Processing: $file"
    
    # 1. Add import if not present
    if ! grep -q "use crate::valuations::instruments::InstrumentWrapper;" "$filepath"; then
        echo "  → Adding import..."
        # Insert after last use statement before wasm_bindgen
        awk '/^use / && !/wasm_bindgen/ { last=NR }
             NR==last+1 && !/^use / && !done { print "use crate::valuations::instruments::InstrumentWrapper;"; done=1 }
             { print }' "$filepath" > "$filepath.tmp"
        mv "$filepath.tmp" "$filepath"
    else
        echo "  ✓ Import already present"
    fi
    
    # 2. Replace self.inner with self.0
    if grep -q "self\.inner" "$filepath"; then
        echo "  → Replacing self.inner with self.0..."
        sed -i '' 's/self\.inner/self.0/g' "$filepath"
    else
        echo "  ✓ Field access already migrated"
    fi
    
    echo "  ✓ Automated steps complete for $file"
    echo "  ⚠️  MANUAL: Update struct definition and impl block"
    echo ""
done

echo "=== Automated Migration Complete ==="
echo ""
echo "Next steps for each file:"
echo "1. Open the file"
echo "2. Change: pub struct JsXxx { inner: Xxx } → pub struct JsXxx(Xxx);"
echo "3. Replace the impl block with trait impl (see QUICK_MIGRATION_REFERENCE.md)"
echo "4. Run: cargo check"
echo ""
echo "Files processed: ${#REMAINING[@]}"
echo "See MIGRATION_STATUS.md for tracking progress"

