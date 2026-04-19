"""Topology parity audit — validates Python package structure against parity_contract.toml.

Reads finstack-py/parity_contract.toml as the source of truth and checks:

1. Root package parity  — each [crates.*] entry has a Python package at python_package.
2. Module tree parity   — each [crates.*.modules.*] entry has a Python module.
3. Symbol diffing (--symbols) — for present modules, compares Python __all__ against
                          the corresponding Rust extension module's public names.
4. No audit command writes tracked repo files — output goes to .audit/ only.

Exit codes:
  0  — all checks pass (or only expected failures with status="missing")
  1  — unexpected failures or script errors

Usage:
  python scripts/audits/audit_topology.py
  python scripts/audits/audit_topology.py --strict    # fail on any missing item
  python scripts/audits/audit_topology.py --symbols   # also emit symbol_gaps.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ImportError:
        print(
            "Error: tomllib not available. Use Python 3.11+ or install tomli:\n  pip install tomli",
            file=sys.stderr,
        )
        sys.exit(1)


def _python_package_to_path(python_package: str, finstack_py_root: Path) -> Path:
    """Convert a dotted Python package name to a filesystem path under finstack-py."""
    parts = python_package.split(".")
    # e.g. finstack.core.currency -> finstack-py/finstack/core/currency
    pkg_path = finstack_py_root
    for part in parts:
        pkg_path = pkg_path / part
    return pkg_path


def _package_exists(python_package: str, finstack_py_root: Path) -> bool:
    """Check whether a Python package exists (has __init__.py or __init__.pyi)."""
    pkg_path = _python_package_to_path(python_package, finstack_py_root)
    return (pkg_path / "__init__.py").exists() or (pkg_path / "__init__.pyi").exists()


def _module_exists(python_module: str, finstack_py_root: Path) -> bool:
    """Check whether a Python module exists as a package or .py/.pyi file."""
    # First check if it's a package
    if _package_exists(python_module, finstack_py_root):
        return True
    # Then check if it's a .py or .pyi file
    parts = python_module.split(".")
    parent = finstack_py_root
    for part in parts[:-1]:
        parent = parent / part
    leaf = parts[-1]
    return (parent / f"{leaf}.py").exists() or (parent / f"{leaf}.pyi").exists()


def run_audit(
    contract_path: Path,
    finstack_py_root: Path,
    strict: bool = False,
) -> dict:
    """Run all topology checks and return a structured report."""
    with contract_path.open("rb") as f:
        contract = tomllib.load(f)

    crates = contract.get("crates", {})

    results: dict = {
        "meta": {
            "contract_version": contract.get("meta", {}).get("version", "unknown"),
            "contract_path": str(contract_path),
            "finstack_py_root": str(finstack_py_root),
            "strict": strict,
        },
        "root_packages": [],
        "modules": [],
        "summary": {
            "root_packages_total": 0,
            "root_packages_present": 0,
            "root_packages_missing": 0,
            "modules_total": 0,
            "modules_present": 0,
            "modules_missing": 0,
            "unexpected_failures": 0,
        },
    }

    # --- 1. Root package parity ---
    for crate_key, crate_cfg in crates.items():
        python_package = crate_cfg.get("python_package", "")
        declared_status = crate_cfg.get("status", "unknown")
        present = _package_exists(python_package, finstack_py_root)

        entry = {
            "crate": crate_key,
            "python_package": python_package,
            "declared_status": declared_status,
            "present": present,
            "check": "pass" if present else ("expected_missing" if declared_status == "missing" else "FAIL"),
        }
        results["root_packages"].append(entry)
        results["summary"]["root_packages_total"] += 1
        if present:
            results["summary"]["root_packages_present"] += 1
        else:
            results["summary"]["root_packages_missing"] += 1
            if declared_status != "missing":
                results["summary"]["unexpected_failures"] += 1

    # --- 2. Module tree parity ---
    for crate_key, crate_cfg in crates.items():
        modules = crate_cfg.get("modules", {})
        for mod_key, mod_cfg in modules.items():
            # Skip dotted sub-keys like "analysis.valuation" — those are sub-module entries
            python_module = mod_cfg.get("python", "")
            declared_status = mod_cfg.get("status", "unknown")
            note = mod_cfg.get("note", "")
            present = _module_exists(python_module, finstack_py_root)

            entry = {
                "crate": crate_key,
                "module_key": mod_key,
                "python_module": python_module,
                "declared_status": declared_status,
                "present": present,
                "note": note,
                "check": "pass" if present else ("expected_missing" if declared_status == "missing" else "FAIL"),
            }
            results["modules"].append(entry)
            results["summary"]["modules_total"] += 1
            if present:
                results["summary"]["modules_present"] += 1
            else:
                results["summary"]["modules_missing"] += 1
                if declared_status != "missing":
                    results["summary"]["unexpected_failures"] += 1

    return results


def print_report(report: dict) -> None:
    """Print a human-readable summary of the topology audit."""
    s = report["summary"]
    print("\n=== Topology Parity Audit ===")
    print(f"Contract:    {report['meta']['contract_path']}")
    print(f"Version:     {report['meta']['contract_version']}")
    print()

    print("Root Packages:")
    print(f"  Total:   {s['root_packages_total']}")
    print(f"  Present: {s['root_packages_present']}")
    print(f"  Missing: {s['root_packages_missing']}")
    for entry in report["root_packages"]:
        if entry["check"] == "FAIL":
            print(f"  [FAIL]            {entry['python_package']}  (crate: {entry['crate']})")
        elif entry["check"] == "expected_missing":
            print(f"  [expected_missing] {entry['python_package']}  (crate: {entry['crate']})")

    print()
    print("Modules:")
    print(f"  Total:   {s['modules_total']}")
    print(f"  Present: {s['modules_present']}")
    print(f"  Missing: {s['modules_missing']}")
    fails = [e for e in report["modules"] if e["check"] == "FAIL"]
    if fails:
        for entry in fails:
            print(f"  [FAIL] {entry['python_module']}  (crate: {entry['crate']}, key: {entry['module_key']})")

    print()
    unexpected = s["unexpected_failures"]
    if unexpected == 0:
        print("✓ No unexpected failures. All missing items are declared as expected.")
    else:
        print(f"✗ {unexpected} unexpected failure(s) — items in contract with status!='missing' are absent.")


def _rust_names_for_module(python_module: str) -> set[str] | None:
    """Return public names from the Rust extension layer for a Python module path.

    Navigates ``finstack.finstack`` to find the corresponding raw Rust module.
    Returns None if the module is not accessible in the Rust layer.
    """
    try:
        import types

        from finstack import finstack as _fs  # type: ignore[reportMissingModuleSource]

        parts = python_module.split(".")[1:]  # strip leading 'finstack'
        mod: object = _fs
        for part in parts:
            mod = getattr(mod, part, None)
            if mod is None:
                return None
        return {
            n for n in dir(mod) if not n.startswith("_") and not isinstance(getattr(mod, n, None), types.ModuleType)
        }
    except Exception:
        return None


def _python_all_for_module(python_module: str) -> set[str] | None:
    """Return the __all__ export set from an importable Python module.

    Falls back to dir()-derived names (excluding '_'-prefixed) when __all__
    is absent.  Returns None if the import fails.
    """
    try:
        import importlib
        import types

        m = importlib.import_module(python_module)
        if hasattr(m, "__all__"):
            exports = set()
            for name in m.__all__:
                if name.startswith("_"):
                    continue
                value = getattr(m, name, None)
                # Ignore wrapper-only submodule re-exports and absent lazy placeholders.
                if value is None or isinstance(value, types.ModuleType):
                    continue
                exports.add(name)
            return exports
        return {n for n in dir(m) if not n.startswith("_")}
    except Exception:
        return None


def run_symbol_audit(
    contract_path: Path,
    finstack_py_root: Path,
) -> list[dict]:
    """Diff Python __all__ vs Rust extension layer for each present module.

    Returns a list of per-module gap records:
    {
        "python_module": str,
        "rust_names": list[str] | None,   # None = not accessible in Rust layer
        "python_names": list[str] | None, # None = import failed
        "in_rust_not_python": list[str],  # exposed by Rust but absent from __all__
        "in_python_not_rust": list[str],  # in __all__ but not in Rust layer
        "rust_count": int,
        "python_count": int,
    }
    """
    try:
        import tomllib  # type: ignore[reportMissingModuleSource]
    except ImportError:
        import tomli as tomllib  # type: ignore[no-redef,reportMissingModuleSource]

    with contract_path.open("rb") as f:
        contract = tomllib.load(f)

    crates = contract.get("crates", {})
    gaps = []

    for crate_key, crate_cfg in crates.items():
        python_package = crate_cfg.get("python_package", "")
        declared_status = crate_cfg.get("status", "unknown")

        # Root package diff
        if declared_status == "exists" and _package_exists(python_package, finstack_py_root):
            rust_names = _rust_names_for_module(python_package)
            python_names = _python_all_for_module(python_package)
            if rust_names is not None and python_names is not None:
                in_rust_not_py = sorted(rust_names - python_names)
                in_py_not_rust = sorted(python_names - rust_names)
                if in_rust_not_py or in_py_not_rust:
                    gaps.append({
                        "python_module": python_package,
                        "crate": crate_key,
                        "rust_count": len(rust_names),
                        "python_count": len(python_names),
                        "in_rust_not_python": in_rust_not_py,
                        "in_python_not_rust": in_py_not_rust,
                    })

        # Module-level diffs
        for mod_key, mod_cfg in crate_cfg.get("modules", {}).items():
            python_module = mod_cfg.get("python", "")
            mod_status = mod_cfg.get("status", "unknown")
            if mod_status != "exists":
                continue
            if not _module_exists(python_module, finstack_py_root):
                continue

            rust_names = _rust_names_for_module(python_module)
            python_names = _python_all_for_module(python_module)
            if rust_names is None or python_names is None:
                continue
            in_rust_not_py = sorted(rust_names - python_names)
            in_py_not_rust = sorted(python_names - rust_names)
            if in_rust_not_py or in_py_not_rust:
                gaps.append({
                    "python_module": python_module,
                    "crate": crate_key,
                    "module_key": mod_key,
                    "rust_count": len(rust_names),
                    "python_count": len(python_names),
                    "in_rust_not_python": in_rust_not_py,
                    "in_python_not_rust": in_py_not_rust,
                })

    return gaps


def main() -> int:
    """Run the CLI entrypoint for the topology parity audit."""
    parser = argparse.ArgumentParser(description="Topology parity audit")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit 1 on any missing item, even those declared as expected_missing",
    )
    parser.add_argument(
        "--symbols",
        action="store_true",
        help="Also run per-module symbol diff (requires finstack importable on sys.path)",
    )
    args = parser.parse_args()

    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    contract_path = project_root / "finstack-py" / "parity_contract.toml"
    finstack_py_root = project_root / "finstack-py"

    if not contract_path.exists():
        print(f"Error: contract not found at {contract_path}", file=sys.stderr)
        return 1

    report = run_audit(contract_path, finstack_py_root, strict=args.strict)
    print_report(report)

    # Write JSON report to .audit/ — never write to tracked files
    audit_dir = project_root / ".audit"
    audit_dir.mkdir(exist_ok=True)
    report_file = audit_dir / "topology_report.json"
    with report_file.open("w") as f:
        json.dump(report, f, indent=2)
    print(f"\nJSON report written to {report_file}")

    # Optional symbol diff
    if args.symbols:
        gaps = run_symbol_audit(contract_path, finstack_py_root)
        gaps_file = audit_dir / "symbol_gaps.json"
        with gaps_file.open("w") as f:
            json.dump({"total_modules_with_gaps": len(gaps), "gaps": gaps}, f, indent=2)
        print(f"Symbol gaps written to {gaps_file}  ({len(gaps)} modules with gaps)")

    s = report["summary"]
    if args.strict:
        return 0 if (s["root_packages_missing"] == 0 and s["modules_missing"] == 0) else 1
    return 0 if s["unexpected_failures"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
