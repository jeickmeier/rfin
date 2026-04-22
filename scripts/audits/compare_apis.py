#!/usr/bin/env python3
"""Compare Rust, Python, and WASM APIs to generate a parity audit report.

This script:
1. Loads the extracted API data from Rust library and both bindings
2. Compares classes, methods, and functions
3. Identifies gaps and mismatches
4. Generates a comprehensive markdown report
"""

from dataclasses import dataclass
import json
from pathlib import Path
import sys
from typing import Any


@dataclass
class ParityIssue:
    """Represents a parity issue between bindings."""

    category: str  # "missing_in_wasm", "missing_in_python", "name_mismatch", etc.
    item_type: str  # "class", "method", "function"
    item_name: str
    details: str
    module: str = ""


class APIComparator:
    """Compare APIs between Rust, Python, and WASM bindings."""

    def __init__(self, rust_api: dict, python_api: dict, wasm_api: dict) -> None:
        """Initialize the comparator with Rust, Python, and WASM API data."""
        self.rust_api = rust_api
        self.python_api = python_api
        self.wasm_api = wasm_api
        self.issues: list[ParityIssue] = []

    def snake_to_camel(self, snake_str: str) -> str:
        """Convert snake_case to camelCase."""
        components = snake_str.split("_")
        return components[0] + "".join(x.title() for x in components[1:])

    def _collect_rust_types(self) -> set[str]:
        """Collect all public types from Rust API (full walk, dupes removed)."""
        rust_types: set[str] = set()
        seen: set[int] = set()

        def collect_from_module(module_data: dict[str, Any]) -> None:
            """Recursively collect types from module tree with cycle protection."""
            pid = id(module_data)
            if pid in seen:
                return
            seen.add(pid)
            rust_types.update(module_data.get("types", []))
            rust_types.update(module_data.get("functions", []))
            for items in module_data.get("re_exports", {}).values():
                rust_types.update(items)
            for submod_data in module_data.get("modules", {}).values():
                collect_from_module(submod_data)

        for crate_data in self.rust_api.get("api", {}).values():
            exports = crate_data.get("exports", {})
            rust_types.update(exports.get("types", []))
            rust_types.update(exports.get("functions", []))
            for items in exports.get("re_exports", {}).values():
                rust_types.update(items)
            for mod_info in crate_data.get("modules", {}).values():
                collect_from_module(mod_info)

        return rust_types

    def _collect_python_functions(self) -> set[str]:
        """Collect all top-level pyfunction names from Python API."""
        names: set[str] = set()
        for crate_data in self.python_api.get("api", {}).values():
            for mod_info in crate_data.get("modules", {}).values():
                for fn in mod_info.get("functions", []):
                    names.add(fn.get("name", "") if isinstance(fn, dict) else fn)
        names.discard("")
        return names

    def _collect_wasm_functions(self) -> set[str]:
        """Collect all top-level wasm_bindgen function names."""
        names: set[str] = set()
        for crate_data in self.wasm_api.get("api", {}).values():
            for mod_info in crate_data.get("modules", {}).values():
                for fn in mod_info.get("functions", []):
                    if isinstance(fn, dict):
                        names.add(fn.get("js_name") or fn.get("name", ""))
                    else:
                        names.add(fn)
        names.discard("")
        return names

    def _collect_python_classes(self) -> set[str]:
        """Collect all classes from Python API."""
        python_classes = set()
        for module_data in self.python_api.get("api", {}).values():
            for mod_info in module_data.get("modules", {}).values():
                for cls in mod_info.get("classes", []):
                    python_classes.add(cls.get("name", ""))
        return python_classes

    def _collect_wasm_classes(self) -> set[str]:
        """Collect all classes from WASM API."""
        wasm_classes = set()
        for module_data in self.wasm_api.get("api", {}).values():
            for mod_info in module_data.get("modules", {}).values():
                for cls in mod_info.get("classes", []):
                    wasm_classes.add(cls.get("js_name", cls.get("name", "")))
        # Also get from exports
        wasm_exports = set(self.wasm_api.get("exports", {}).get("types", []))
        wasm_classes.update(wasm_exports)
        return wasm_classes

    def compare_classes(self) -> dict[str, Any]:
        """Compare classes/types between all three APIs.

        Returns:
            Dictionary with comparison results
        """
        rust_types = self._collect_rust_types()
        python_classes = self._collect_python_classes()
        wasm_classes = self._collect_wasm_classes()

        # Find intersections
        in_all_three = rust_types & python_classes & wasm_classes
        rust_and_python = (rust_types & python_classes) - wasm_classes
        rust_and_wasm = (rust_types & wasm_classes) - python_classes
        python_and_wasm = (python_classes & wasm_classes) - rust_types
        only_rust = rust_types - python_classes - wasm_classes
        only_python = python_classes - rust_types - wasm_classes
        only_wasm = wasm_classes - rust_types - python_classes

        return {
            "in_all_three": in_all_three,
            "rust_and_python": rust_and_python,
            "rust_and_wasm": rust_and_wasm,
            "python_and_wasm": python_and_wasm,
            "only_rust": only_rust,
            "only_python": only_python,
            "only_wasm": only_wasm,
            "rust_total": len(rust_types),
            "python_total": len(python_classes),
            "wasm_total": len(wasm_classes),
        }

    def compare_instruments(self) -> dict[str, Any]:
        """Compare instrument coverage.

        Instruments are exposed as individual Rust structs but Python/WASM
        bindings expose them via JSON `InstrumentSpec` passed to a single
        `price_instrument(spec)` entrypoint rather than per-class wrappers.
        Treat an instrument as "covered" in Python/WASM when `price_instrument`
        (or equivalent) is available; Rust is still the canonical per-class surface.
        """
        all_instruments = {
            # Fixed Income
            "Bond",
            "Deposit",
            "InterestRateSwap",
            "ForwardRateAgreement",
            "Swaption",
            "BasisSwap",
            "InterestRateOption",
            "InterestRateFuture",
            # FX
            "FxSpot",
            "FxOption",
            "FxSwap",
            "FxBarrierOption",
            # Credit (canonical Rust names use all-caps CDS acronym)
            "CreditDefaultSwap",
            "CDSIndex",
            "CDSTranche",
            "CDSOption",
            # Equity
            "Equity",
            "EquityOption",
            "EquityTotalReturnSwap",
            "FIIndexTotalReturnSwap",
            "VarianceSwap",
            # Inflation
            "InflationLinkedBond",
            "InflationSwap",
            # Structured
            "Basket",
            "StructuredCredit",
            "PrivateMarketsFund",
            "ConvertibleBond",
            "Repo",
            # Exotic Options
            "AsianOption",
            "BarrierOption",
            "LookbackOption",
            "CliquetOption",
            "QuantoOption",
            "Autocallable",
            "CmsOption",
            "RangeAccrual",
            # Private Credit
            "TermLoan",
            "RevolvingCredit",
        }

        rust_types = self._collect_rust_types()
        python_functions = self._collect_python_functions()
        wasm_functions = self._collect_wasm_functions()

        rust_instruments = all_instruments & rust_types

        # Python/WASM bindings route instruments through a JSON entrypoint.
        # If `price_instrument` exists, all Rust instruments are reachable.
        py_entrypoints = {"price_instrument", "price_instrument_with_metrics"}
        wasm_entrypoints = {
            "priceInstrument",
            "priceInstrumentWithMetrics",
            "price_instrument",
            "price_instrument_with_metrics",
        }
        python_covers_all = bool(py_entrypoints & python_functions)
        wasm_covers_all = bool(wasm_entrypoints & wasm_functions)

        python_covered = rust_instruments if python_covers_all else set()
        wasm_covered = rust_instruments if wasm_covers_all else set()
        in_all_three = rust_instruments & python_covered & wasm_covered

        return {
            "total_expected": len(all_instruments),
            "in_rust": len(rust_instruments),
            "in_python": len(python_covered),
            "in_wasm": len(wasm_covered),
            "in_all_three": len(in_all_three),
            "missing_in_rust": sorted(all_instruments - rust_instruments),
            "missing_in_python": sorted(all_instruments - python_covered),
            "missing_in_wasm": sorted(all_instruments - wasm_covered),
            "python_covers_all": python_covers_all,
            "wasm_covers_all": wasm_covers_all,
            "python_entrypoint_present": sorted(py_entrypoints & python_functions),
            "wasm_entrypoint_present": sorted(wasm_entrypoints & wasm_functions),
        }

    def compare_calibration(self) -> dict[str, Any]:
        """Compare calibration API coverage.

        Like instruments, calibration follows a JSON-envelope pattern:
        Rust exposes per-curve parameter structs (`DiscountCurveParams`,
        `HazardCurveParams`, etc.) via `CalibrationEnvelope`, and Python/WASM
        bindings expose a single `calibrate(envelope_json)` entrypoint.
        Coverage is measured by the presence of canonical Rust types and
        the JSON entrypoints.
        """
        # Canonical Rust calibration surface (from finstack/valuations/src/calibration).
        calibration_classes = {
            # Config / engine surface
            "CalibrationConfig",
            "CalibrationMethod",
            "SolverConfig",
            "ResidualWeightingScheme",
            # Report / result surface
            "CalibrationReport",
            "CalibrationDiagnostics",
            "CalibrationResult",
            "CalibrationResultEnvelope",
            "QuoteQuality",
            # JSON-plan surface
            "CalibrationEnvelope",
            "CalibrationPlan",
            "CalibrationStep",
            # Per-curve parameter specs (routed via CalibrationEnvelope)
            "DiscountCurveParams",
            "ForwardCurveParams",
            "HazardCurveParams",
            "InflationCurveParams",
            "VolSurfaceParams",
            "BaseCorrelationParams",
            # Solve configs
            "DiscountCurveSolveConfig",
            "HazardCurveSolveConfig",
            "InflationCurveSolveConfig",
            # Validation
            "ValidationConfig",
            "CurveValidator",
            "SurfaceValidator",
        }

        rust_types = self._collect_rust_types()
        python_functions = self._collect_python_functions()
        wasm_functions = self._collect_wasm_functions()

        rust_cal = calibration_classes & rust_types

        # Python/WASM route calibration through a JSON entrypoint.
        py_entrypoints = {"calibrate", "validate_calibration_json"}
        wasm_entrypoints = {"calibrate", "validateCalibrationJson"}
        python_covers_all = bool(py_entrypoints & python_functions)
        wasm_covers_all = bool(wasm_entrypoints & wasm_functions)

        python_covered = rust_cal if python_covers_all else set()
        wasm_covered = rust_cal if wasm_covers_all else set()
        in_all_three = rust_cal & python_covered & wasm_covered

        return {
            "total_expected": len(calibration_classes),
            "in_rust": len(rust_cal),
            "in_python": len(python_covered),
            "in_wasm": len(wasm_covered),
            "in_all_three": len(in_all_three),
            "missing_in_rust": sorted(calibration_classes - rust_cal),
            "missing_in_python": sorted(calibration_classes - python_covered),
            "missing_in_wasm": sorted(calibration_classes - wasm_covered),
            "python_covers_all": python_covers_all,
            "wasm_covers_all": wasm_covers_all,
            "python_entrypoint_present": sorted(py_entrypoints & python_functions),
            "wasm_entrypoint_present": sorted(wasm_entrypoints & wasm_functions),
        }

    def generate_report(self) -> str:
        """Generate comprehensive parity audit report in Markdown."""
        class_comparison = self.compare_classes()
        instruments = self.compare_instruments()
        calibration = self.compare_calibration()

        # Build the report
        lines = [
            "# Rust-Python-WASM Bindings Parity Audit",
            "",
            f"**Generated:** {Path(__file__).name}",
            "",
            "## Executive Summary",
            "",
            f"- **Total types in Rust:** {class_comparison['rust_total']}",
            f"- **Total classes in Python:** {class_comparison['python_total']}",
            f"- **Total classes in WASM:** {class_comparison['wasm_total']}",
            f"- **In all three:** {len(class_comparison['in_all_three'])}",
            f"- **Only in Rust:** {len(class_comparison['only_rust'])}",
            f"- **Only in Python:** {len(class_comparison['only_python'])}",
            f"- **Only in WASM:** {len(class_comparison['only_wasm'])}",
            "",
            "## Instrument Coverage",
            "",
            "Instruments are individual Rust types. Python and WASM bindings expose them",
            "through a JSON `InstrumentSpec` entrypoint rather than per-class wrappers;",
            "coverage here is measured by the presence of that entrypoint.",
            "",
            f"- **Expected instruments:** {instruments['total_expected']}",
            f"- **In Rust (per-class):** {instruments['in_rust']} "
            f"({instruments['in_rust'] * 100 // instruments['total_expected']}%)",
            f"- **In Python (via JSON entrypoint):** "
            f"{'yes — ' + ', '.join(instruments['python_entrypoint_present']) if instruments['python_covers_all'] else 'no entrypoint found'}",
            f"- **In WASM (via JSON entrypoint):** "
            f"{'yes — ' + ', '.join(instruments['wasm_entrypoint_present']) if instruments['wasm_covers_all'] else 'no entrypoint found'}",
            f"- **Reachable in all three:** {instruments['in_all_three']}",
            "",
        ]

        if instruments["missing_in_rust"]:
            lines.extend(["### Missing in Rust", "", "```"])
            lines.extend([f"- {instr}" for instr in instruments["missing_in_rust"]])
            lines.append("```")
            lines.append("")

        if instruments["missing_in_python"]:
            lines.extend(["### Missing in Python", "", "```"])
            lines.extend([f"- {instr}" for instr in instruments["missing_in_python"]])
            lines.append("```")
            lines.append("")

        if instruments["missing_in_wasm"]:
            lines.extend(["### Missing in WASM", "", "```"])
            lines.extend([f"- {instr}" for instr in instruments["missing_in_wasm"]])
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Calibration API Coverage",
            "",
            "Calibration follows the same JSON-envelope pattern as instruments:",
            "Rust exposes per-curve parameter structs via `CalibrationEnvelope`,",
            "and Python/WASM expose a single `calibrate(envelope_json)` entrypoint.",
            "",
            f"- **Expected calibration types:** {calibration['total_expected']}",
            f"- **In Rust (per-class):** {calibration['in_rust']} "
            f"({calibration['in_rust'] * 100 // calibration['total_expected']}%)",
            f"- **In Python (via JSON entrypoint):** "
            f"{'yes — ' + ', '.join(calibration['python_entrypoint_present']) if calibration['python_covers_all'] else 'no entrypoint found'}",
            f"- **In WASM (via JSON entrypoint):** "
            f"{'yes — ' + ', '.join(calibration['wasm_entrypoint_present']) if calibration['wasm_covers_all'] else 'no entrypoint found'}",
            f"- **Reachable in all three:** {calibration['in_all_three']}",
            "",
        ])

        if calibration["missing_in_rust"]:
            lines.extend(["### Missing in Rust", "", "```"])
            lines.extend([f"- {cal}" for cal in calibration["missing_in_rust"]])
            lines.append("```")
            lines.append("")

        if calibration["missing_in_python"]:
            lines.extend(["### Missing in Python", "", "```"])
            lines.extend([f"- {cal}" for cal in calibration["missing_in_python"]])
            lines.append("```")
            lines.append("")

        if calibration["missing_in_wasm"]:
            lines.extend(["### Missing in WASM", "", "```"])
            lines.extend([f"- {cal}" for cal in calibration["missing_in_wasm"]])
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Complete Type/Class Comparison",
            "",
            "### Types/Classes in All Three",
            "",
            f"**Count:** {len(class_comparison['in_all_three'])}",
            "",
            "```",
        ])
        lines.extend([f"✓ {cls}" for cls in sorted(class_comparison["in_all_three"])])
        lines.append("```")
        lines.append("")

        if class_comparison["rust_and_python"]:
            lines.extend([
                "### In Rust and Python (missing in WASM)",
                "",
                f"**Count:** {len(class_comparison['rust_and_python'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["rust_and_python"])[:20]])  # Limit to first 20
            if len(class_comparison["rust_and_python"]) > 20:
                lines.append(f"... and {len(class_comparison['rust_and_python']) - 20} more")
            lines.append("```")
            lines.append("")

        if class_comparison["rust_and_wasm"]:
            lines.extend([
                "### In Rust and WASM (missing in Python)",
                "",
                f"**Count:** {len(class_comparison['rust_and_wasm'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["rust_and_wasm"])[:20]])
            if len(class_comparison["rust_and_wasm"]) > 20:
                lines.append(f"... and {len(class_comparison['rust_and_wasm']) - 20} more")
            lines.append("```")
            lines.append("")

        if class_comparison["python_and_wasm"]:
            lines.extend([
                "### In Python and WASM (missing in Rust)",
                "",
                f"**Count:** {len(class_comparison['python_and_wasm'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["python_and_wasm"])[:20]])
            if len(class_comparison["python_and_wasm"]) > 20:
                lines.append(f"... and {len(class_comparison['python_and_wasm']) - 20} more")
            lines.append("```")
            lines.append("")

        if class_comparison["only_rust"]:
            lines.extend([
                "### Only in Rust",
                "",
                f"**Count:** {len(class_comparison['only_rust'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["only_rust"])[:20]])
            if len(class_comparison["only_rust"]) > 20:
                lines.append(f"... and {len(class_comparison['only_rust']) - 20} more")
            lines.append("```")
            lines.append("")

        if class_comparison["only_python"]:
            lines.extend([
                "### Only in Python",
                "",
                f"**Count:** {len(class_comparison['only_python'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["only_python"])[:20]])
            if len(class_comparison["only_python"]) > 20:
                lines.append(f"... and {len(class_comparison['only_python']) - 20} more")
            lines.append("```")
            lines.append("")

        if class_comparison["only_wasm"]:
            lines.extend([
                "### Only in WASM",
                "",
                f"**Count:** {len(class_comparison['only_wasm'])}",
                "",
                "```",
            ])
            lines.extend([f"- {cls}" for cls in sorted(class_comparison["only_wasm"])[:20]])
            if len(class_comparison["only_wasm"]) > 20:
                lines.append(f"... and {len(class_comparison['only_wasm']) - 20} more")
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Naming Convention Patterns",
            "",
            "### Identified Patterns",
            "",
            "| Rust | Python | WASM | Pattern |",
            "|------|--------|------|---------|",
        ])

        # Sample some naming patterns
        sample_patterns = [
            ("build_periods", "build_periods", "buildPeriods", "snake_case → snake_case → camelCase"),
            ("from_code", "from_code", "fromCode", "snake_case → snake_case → camelCase"),
            ("next_imm", "next_imm", "nextImm", "snake_case → snake_case → camelCase"),
            ("is_actual", "is_actual", "isActual", "snake_case → snake_case → camelCase"),
            ("Currency", "Currency", "Currency", "PascalCase → PascalCase → PascalCase"),
            ("Money", "Money", "Money", "PascalCase → PascalCase → PascalCase"),
        ]

        for rust_name, py_name, wasm_name, pattern in sample_patterns:
            lines.append(f"| `{rust_name}` | `{py_name}` | `{wasm_name}` | {pattern} |")

        lines.extend([
            "",
            "## Recommendations",
            "",
            "### High Priority",
            "",
        ])

        if instruments["missing_in_rust"]:
            lines.append(
                f"1. **Add {len(instruments['missing_in_rust'])} missing instruments to Rust:** "
                + ", ".join(instruments["missing_in_rust"][:3])
                + ("..." if len(instruments["missing_in_rust"]) > 3 else "")
            )

        if instruments["missing_in_wasm"]:
            lines.append(
                f"2. **Add {len(instruments['missing_in_wasm'])} missing instruments to WASM:** "
                + ", ".join(instruments["missing_in_wasm"][:3])
                + ("..." if len(instruments["missing_in_wasm"]) > 3 else "")
            )

        if instruments["missing_in_python"]:
            lines.append(
                f"3. **Add {len(instruments['missing_in_python'])} missing instruments to Python:** "
                + ", ".join(instruments["missing_in_python"][:3])
                + ("..." if len(instruments["missing_in_python"]) > 3 else "")
            )

        if calibration["missing_in_rust"]:
            lines.append(
                f"4. **Complete calibration API in Rust:** {len(calibration['missing_in_rust'])} types missing"
            )

        if calibration["missing_in_wasm"]:
            lines.append(
                f"5. **Complete calibration API in WASM:** {len(calibration['missing_in_wasm'])} types missing"
            )

        if calibration["missing_in_python"]:
            lines.append(
                f"6. **Complete calibration API in Python:** {len(calibration['missing_in_python'])} types missing"
            )

        lines.extend([
            "",
            "### Medium Priority",
            "",
            "1. **Create comprehensive method parity report** - Compare methods within each class",
            "2. **Document naming convention mapping** - Create NAMING_CONVENTIONS.md",
            "3. **Add TypeScript type definitions** - Generate .d.ts files with JSDoc",
            "4. **Create cross-language test suite** - Verify identical behavior",
            "",
            "### Low Priority",
            "",
            "1. **Create migration guide** - Help developers switch between languages",
            "2. **Add side-by-side examples** - Show equivalent code in both languages",
            "3. **Set up CI parity checks** - Prevent future regressions",
            "",
            "## Next Steps",
            "",
            "1. Run `scripts/compare_apis.py` to regenerate this report after changes",
            "2. Address high-priority gaps in both bindings",
            "3. Create detailed method-level comparison for shared classes",
            "4. Generate TypeScript definitions from wasm-bindgen",
            "5. Implement cross-language test suite with golden values",
            "",
            "---",
            "",
            "*This report was automatically generated. Do not edit manually.*",
        ])

        return "\n".join(lines)


def main() -> int:
    """Main entry point."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent

    # Load API data from .audit/ (gitignored build artifacts)
    audit_dir = project_root / ".audit"
    rust_api_file = audit_dir / "rust_api.json"
    python_api_file = audit_dir / "python_api.json"
    wasm_api_file = audit_dir / "wasm_api.json"

    if not rust_api_file.exists():
        print(f"Missing {rust_api_file} — run audit_rust_api.py first", file=sys.stderr)
        return 1

    if not python_api_file.exists():
        print(f"Missing {python_api_file} — run audit_python_api.py first", file=sys.stderr)
        return 1

    if not wasm_api_file.exists():
        print(f"Missing {wasm_api_file} — run audit_wasm_api.py first", file=sys.stderr)
        return 1

    with rust_api_file.open() as f:
        rust_api = json.load(f)

    with python_api_file.open() as f:
        python_api = json.load(f)

    with wasm_api_file.open() as f:
        wasm_api = json.load(f)

    # Compare APIs
    comparator = APIComparator(rust_api, python_api, wasm_api)
    report = comparator.generate_report()

    # Write report to .audit/ (gitignored) — never write to tracked repo files
    audit_dir.mkdir(exist_ok=True)
    output_file = audit_dir / "PARITY_AUDIT.md"
    output_file.write_text(report)
    print(f"Report written to {output_file}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
