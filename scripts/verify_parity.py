#!/usr/bin/env python3
"""Parity verification script for finstack Python and WASM bindings.

This script verifies that:
1. All instruments in core Rust crate are bound in both Python and WASM
2. All common parameter types are bound in both Python and WASM
3. All MC generator components are present in both bindings
4. Performance utilities are available in both bindings
5. DataFrame support exists in both bindings

Usage:
    python scripts/verify_parity.py
"""

from pathlib import Path
import sys

# Add project root to path
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))

# Colors for terminal output
GREEN = "\033[32m"
RED = "\033[31m"
YELLOW = "\033[33m"
RESET = "\033[0m"


def get_instruments_from_rust() -> set[str]:
    """Extract instrument types from core Rust crate."""
    instruments_dir = project_root / "finstack" / "valuations" / "src" / "instruments"
    instruments = set()

    # Exclude non-instrument helper files
    exclude = {"mod.rs", "wrapper.rs", "pricing_overrides.rs", "common"}

    # Get all instrument modules
    for file in instruments_dir.glob("*.rs"):
        if file.name not in exclude and not file.name.startswith("_"):
            name = file.stem
            instruments.add(name)

    return instruments


def get_instruments_from_python() -> set[str]:
    """Extract instrument types from Python bindings."""
    py_bindings_dir = project_root / "finstack-py" / "src" / "valuations" / "instruments"
    instruments = set()

    exclude = {"mod.rs", "wrapper.rs", "pricing_overrides.rs"}

    for file in py_bindings_dir.glob("*.rs"):
        if file.name not in exclude and not file.name.startswith("_"):
            name = file.stem
            instruments.add(name)

    return instruments


def get_instruments_from_wasm() -> set[str]:
    """Extract instrument types from WASM bindings."""
    wasm_bindings_dir = project_root / "finstack-wasm" / "src" / "valuations" / "instruments"
    instruments = set()

    exclude = {"mod.rs", "wrapper.rs", "pricing_overrides.rs"}

    for file in wasm_bindings_dir.glob("*.rs"):
        if file.name not in exclude and not file.name.startswith("_"):
            name = file.stem
            instruments.add(name)

    return instruments


def check_common_params() -> list[tuple[str, bool, bool]]:
    """Check if common parameter types are bound in both Python and WASM."""
    params = ["OptionType", "ExerciseStyle", "SettlementType", "PayReceive", "BarrierType"]
    results = []

    # Check Python
    py_params_file = project_root / "finstack-py" / "src" / "valuations" / "common" / "parameters.rs"
    py_has_params = py_params_file.exists()

    # Check WASM
    wasm_params_file = project_root / "finstack-wasm" / "src" / "valuations" / "common" / "parameters.rs"
    wasm_has_params = wasm_params_file.exists()

    for param in params:
        py_has = False
        wasm_has = False

        if py_has_params:
            content = py_params_file.read_text()
            py_has = f"Py{param}" in content or f"struct Py{param}" in content

        if wasm_has_params:
            content = wasm_params_file.read_text()
            wasm_has = f"Js{param}" in content or f"struct Js{param}" in content

        results.append((param, py_has, wasm_has))

    return results


def check_mc_components() -> list[tuple[str, bool, bool]]:
    """Check if MC generator components are present in both bindings."""
    components = ["mc_generator", "mc_paths", "mc_params", "mc_result"]
    results = []

    for comp in components:
        py_file = project_root / "finstack-py" / "src" / "valuations" / f"{comp}.rs"
        wasm_file = project_root / "finstack-wasm" / "src" / "valuations" / f"{comp}.rs"

        results.append((comp, py_file.exists(), wasm_file.exists()))

    return results


def check_performance_utils() -> list[tuple[str, bool, bool]]:
    """Check if performance utilities are in core and bound in both."""
    results = []

    # Check xirr (in xirr.rs, not performance.rs)
    xirr_core = (project_root / "finstack" / "core" / "src" / "cashflow" / "xirr.rs").exists()
    xirr_py = (project_root / "finstack-py" / "src" / "core" / "cashflow" / "xirr.rs").exists()
    xirr_wasm = (project_root / "finstack-wasm" / "src" / "valuations" / "performance.rs").exists()

    xirr_wasm_content = ""
    if xirr_wasm:
        xirr_wasm_content = (project_root / "finstack-wasm" / "src" / "valuations" / "performance.rs").read_text()

    results.append(("xirr", xirr_core, xirr_py and ("xirr_wasm" in xirr_wasm_content or "xirr" in xirr_wasm_content)))

    # Check irr_periodic and npv (in performance.rs)
    core_file = project_root / "finstack" / "core" / "src" / "cashflow" / "performance.rs"
    py_file = project_root / "finstack-py" / "src" / "core" / "cashflow" / "performance.rs"
    wasm_file = project_root / "finstack-wasm" / "src" / "valuations" / "performance.rs"

    for util in ["irr_periodic", "npv"]:
        util_in_core = core_file.exists() and (util in core_file.read_text() if core_file.exists() else False)
        util_in_py = py_file.exists() and (f"py_{util}" in py_file.read_text() if py_file.exists() else False)
        util_in_wasm = wasm_file.exists() and (f"{util}_wasm" in wasm_file.read_text() if wasm_file.exists() else False)

        results.append((util, util_in_core, util_in_py and util_in_wasm))

    return results


def check_dataframe_support() -> tuple[bool, bool]:
    """Check if DataFrame support exists in both bindings."""
    py_file = project_root / "finstack-py" / "src" / "valuations" / "dataframe.rs"
    wasm_file = project_root / "finstack-wasm" / "src" / "valuations" / "dataframe.rs"

    return (py_file.exists(), wasm_file.exists())


def main() -> None:
    """Main entry point for parity verification."""
    print("=" * 80)
    print("Finstack Bindings Parity Verification")
    print("=" * 80)
    print()

    all_passed = True

    # 1. Check instruments
    print("1. Checking Instrument Parity")
    print("-" * 80)
    rust_instruments = get_instruments_from_rust()
    py_instruments = get_instruments_from_python()
    wasm_instruments = get_instruments_from_wasm()

    missing_in_py = rust_instruments - py_instruments
    missing_in_wasm = rust_instruments - wasm_instruments

    if missing_in_py:
        print(f"{RED}✗ Python missing: {missing_in_py}{RESET}")
        all_passed = False
    else:
        print(f"{GREEN}✓ All instruments present in Python{RESET}")

    if missing_in_wasm:
        print(f"{RED}✗ WASM missing: {missing_in_wasm}{RESET}")
        all_passed = False
    else:
        print(f"{GREEN}✓ All instruments present in WASM{RESET}")

    print()

    # 2. Check common parameters
    print("2. Checking Common Parameter Types")
    print("-" * 80)
    param_results = check_common_params()
    for param, py_has, wasm_has in param_results:
        if py_has and wasm_has:
            print(f"{GREEN}✓ {param}: Python ✓, WASM ✓{RESET}")
        else:
            print(f"{RED}✗ {param}: Python {'✓' if py_has else '✗'}, WASM {'✓' if wasm_has else '✗'}{RESET}")
            all_passed = False
    print()

    # 3. Check MC components
    print("3. Checking MC Generator Components")
    print("-" * 80)
    mc_results = check_mc_components()
    for comp, py_has, wasm_has in mc_results:
        if py_has and wasm_has:
            print(f"{GREEN}✓ {comp}: Python ✓, WASM ✓{RESET}")
        else:
            print(f"{RED}✗ {comp}: Python {'✓' if py_has else '✗'}, WASM {'✓' if wasm_has else '✗'}{RESET}")
            all_passed = False
    print()

    # 4. Check performance utilities
    print("4. Checking Performance Utilities")
    print("-" * 80)
    perf_results = check_performance_utils()
    for util, in_core, in_bindings in perf_results:
        if in_core and in_bindings:
            print(f"{GREEN}✓ {util}: Core ✓, Bindings ✓{RESET}")
        else:
            print(f"{RED}✗ {util}: Core {'✓' if in_core else '✗'}, Bindings {'✓' if in_bindings else '✗'}{RESET}")
            all_passed = False
    print()

    # 5. Check DataFrame support
    print("5. Checking DataFrame Support")
    print("-" * 80)
    py_df, wasm_df = check_dataframe_support()
    if py_df and wasm_df:
        print(f"{GREEN}✓ DataFrame support: Python ✓, WASM ✓{RESET}")
    else:
        print(f"{RED}✗ DataFrame support: Python {'✓' if py_df else '✗'}, WASM {'✓' if wasm_df else '✗'}{RESET}")
        all_passed = False
    print()

    # Summary
    print("=" * 80)
    if all_passed:
        print(f"{GREEN}✓ All parity checks passed!{RESET}")
        sys.exit(0)
    else:
        print(f"{RED}✗ Some parity checks failed. See details above.{RESET}")
        sys.exit(1)


if __name__ == "__main__":
    sys.exit(main())
