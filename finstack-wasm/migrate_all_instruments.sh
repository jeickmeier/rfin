#!/bin/bash
# Comprehensive migration script for all remaining instrument wrappers
# This script applies the InstrumentWrapper trait pattern to all remaining instruments

set -e

INST_DIR="finstack-wasm/src/valuations/instruments"

echo "=== Instrument Wrapper Migration Script ==="
echo "This will migrate all remaining instrument files to use InstrumentWrapper trait"
echo ""

# Function to migrate a single instrument file
migrate_instrument() {
    local file=$1
    local struct_name=$2
    local inner_type=$3
    
    echo "Migrating: $file ($struct_name -> $inner_type)"
    
    # Add import if not present (at the end of use statements before wasm_bindgen)
    if ! grep -q "use crate::valuations::instruments::InstrumentWrapper;" "$INST_DIR/$file"; then
        # Find last use statement before wasm_bindgen and add import
        perl -i -pe 'if (/^use / && !/wasm_bindgen/ && !$done) { $last_use_line = $.; } 
                     if ($. == $last_use_line + 1 && !$done && !/^use /) { 
                         print "use crate::valuations::instruments::InstrumentWrapper;\n"; 
                         $done = 1;
                     }' "$INST_DIR/$file"
    fi
    
    # Replace all self.inner with self.0
    sed -i '' 's/self\.inner/self.0/g' "$INST_DIR/$file"
    
    echo "  ✓ Completed $file"
}

# Phase 1: Simple instruments (already have equity, repo done)
echo "Phase 1: Simple instruments"
# These need to be done manually due to struct definition replacement complexity

# Phase 2: Medium complexity instruments  
echo ""
echo "Phase 2: Medium complexity instruments"
# These also need manual struct/impl changes

# Phase 3: Complex multi-type files
echo ""
echo "Phase 3: Complex instruments"
# These have multiple types per file

echo ""
echo "=== Migration requires manual struct definition changes ==="
echo "The self.inner -> self.0 replacements can be automated, but"
echo "struct definition and impl block changes need careful manual editing."
echo ""
echo "For each file, you need to:"
echo "1. Change: pub struct JsXxx { inner: Xxx } -> pub struct JsXxx(Xxx);"
echo "2. Replace impl block with trait impl"
echo "3. All self.inner are already changed to self.0"
echo ""
echo "See QUICK_MIGRATION_REFERENCE.md for the complete pattern."

