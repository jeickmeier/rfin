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
        """Collect all public types from Rust API."""
        rust_types = set()

        def collect_from_module(module_data: dict[str, Any]) -> None:
            """Recursively collect types from module tree."""
            # Collect types from this module
            rust_types.update(module_data.get("types", []))
            rust_types.update(module_data.get("functions", []))  # Functions are also part of API
            # Recursively collect from submodules
            for submod_data in module_data.get("modules", {}).values():
                collect_from_module(submod_data)

        for crate_data in self.rust_api.get("api", {}).values():
            # Get types from exports
            exports = crate_data.get("exports", {})
            rust_types.update(exports.get("types", []))
            rust_types.update(exports.get("functions", []))
            # Collect from modules recursively
            for mod_info in crate_data.get("modules", {}).values():
                collect_from_module(mod_info)

        return rust_types

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
        """Compare instrument coverage specifically."""
        # Known instruments list
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
            # Credit
            "CreditDefaultSwap",
            "CDSIndex",
            "CdsTranche",
            "CdsOption",
            # Equity
            "Equity",
            "EquityOption",
            "EquityTotalReturnSwap",
            "FiIndexTotalReturnSwap",
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
        python_classes = self._collect_python_classes()
        wasm_classes = self._collect_wasm_classes()

        rust_instruments = all_instruments & rust_types
        python_instruments = all_instruments & python_classes
        wasm_instruments = all_instruments & wasm_classes

        in_all_three = all_instruments & rust_types & python_classes & wasm_classes

        return {
            "total_expected": len(all_instruments),
            "in_rust": len(rust_instruments),
            "in_python": len(python_instruments),
            "in_wasm": len(wasm_instruments),
            "in_all_three": len(in_all_three),
            "missing_in_rust": sorted(all_instruments - rust_instruments),
            "missing_in_python": sorted(all_instruments - python_instruments),
            "missing_in_wasm": sorted(all_instruments - wasm_instruments),
            "rust_instruments": sorted(rust_instruments),
            "python_instruments": sorted(python_instruments),
            "wasm_instruments": sorted(wasm_instruments),
        }

    def compare_calibration(self) -> dict[str, Any]:
        """Compare calibration API coverage."""
        calibration_classes = {
            "DiscountCurveCalibrator",
            "ForwardCurveCalibrator",
            "HazardCurveCalibrator",
            "InflationCurveCalibrator",
            "VolSurfaceCalibrator",
            "BaseCorrelationCalibrator",
            "SimpleCalibration",
            "CalibrationConfig",
            "CalibrationReport",
            "RatesQuote",
            "CreditQuote",
            "VolQuote",
            "InflationQuote",
        }

        rust_types = self._collect_rust_types()
        python_classes = self._collect_python_classes()
        wasm_classes = self._collect_wasm_classes()

        rust_cal = calibration_classes & rust_types
        python_cal = calibration_classes & python_classes
        wasm_cal = calibration_classes & wasm_classes
        in_all_three = calibration_classes & rust_types & python_classes & wasm_classes

        return {
            "total_expected": len(calibration_classes),
            "in_rust": len(rust_cal),
            "in_python": len(python_cal),
            "in_wasm": len(wasm_cal),
            "in_all_three": len(in_all_three),
            "missing_in_rust": sorted(calibration_classes - rust_cal),
            "missing_in_python": sorted(calibration_classes - python_cal),
            "missing_in_wasm": sorted(calibration_classes - wasm_cal),
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
            f"- **Expected instruments:** {instruments['total_expected']}",
            f"- **In Rust:** {instruments['in_rust']} ({instruments['in_rust'] * 100 // instruments['total_expected']}%)",
            f"- **In Python:** {instruments['in_python']} ({instruments['in_python'] * 100 // instruments['total_expected']}%)",
            f"- **In WASM:** {instruments['in_wasm']} ({instruments['in_wasm'] * 100 // instruments['total_expected']}%)",
            f"- **In all three:** {instruments['in_all_three']}",
            "",
        ]

        if instruments["missing_in_rust"]:
            lines.extend(["### Missing in Rust", "", "```"])
            for instr in instruments["missing_in_rust"]:
                lines.append(f"- {instr}")
            lines.append("```")
            lines.append("")

        if instruments["missing_in_python"]:
            lines.extend(["### Missing in Python", "", "```"])
            for instr in instruments["missing_in_python"]:
                lines.append(f"- {instr}")
            lines.append("```")
            lines.append("")

        if instruments["missing_in_wasm"]:
            lines.extend(["### Missing in WASM", "", "```"])
            for instr in instruments["missing_in_wasm"]:
                lines.append(f"- {instr}")
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Calibration API Coverage",
            "",
            f"- **Expected calibration types:** {calibration['total_expected']}",
            f"- **In Rust:** {calibration['in_rust']} ({calibration['in_rust'] * 100 // calibration['total_expected']}%)",
            f"- **In Python:** {calibration['in_python']} ({calibration['in_python'] * 100 // calibration['total_expected']}%)",
            f"- **In WASM:** {calibration['in_wasm']} ({calibration['in_wasm'] * 100 // calibration['total_expected']}%)",
            f"- **In all three:** {calibration['in_all_three']}",
            "",
        ])

        if calibration["missing_in_rust"]:
            lines.extend(["### Missing in Rust", "", "```"])
            for cal in calibration["missing_in_rust"]:
                lines.append(f"- {cal}")
            lines.append("```")
            lines.append("")

        if calibration["missing_in_python"]:
            lines.extend(["### Missing in Python", "", "```"])
            for cal in calibration["missing_in_python"]:
                lines.append(f"- {cal}")
            lines.append("```")
            lines.append("")

        if calibration["missing_in_wasm"]:
            lines.extend(["### Missing in WASM", "", "```"])
            for cal in calibration["missing_in_wasm"]:
                lines.append(f"- {cal}")
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
        for cls in sorted(class_comparison["in_all_three"]):
            lines.append(f"✓ {cls}")
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
            for cls in sorted(class_comparison["rust_and_python"])[:20]:  # Limit to first 20
                lines.append(f"- {cls}")
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
            for cls in sorted(class_comparison["rust_and_wasm"])[:20]:
                lines.append(f"- {cls}")
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
            for cls in sorted(class_comparison["python_and_wasm"])[:20]:
                lines.append(f"- {cls}")
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
            for cls in sorted(class_comparison["only_rust"])[:20]:
                lines.append(f"- {cls}")
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
            for cls in sorted(class_comparison["only_python"])[:20]:
                lines.append(f"- {cls}")
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
            for cls in sorted(class_comparison["only_wasm"])[:20]:
                lines.append(f"- {cls}")
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

    # Load API data
    rust_api_file = script_dir / "rust_api.json"
    python_api_file = script_dir / "python_api.json"
    wasm_api_file = script_dir / "wasm_api.json"

    if not rust_api_file.exists():
        print(f"Error: {rust_api_file} not found. Run scripts/audit_rust_api.py first.")
        return 1

    if not python_api_file.exists():
        print(f"Error: {python_api_file} not found. Run scripts/audit_python_api.py first.")
        return 1

    if not wasm_api_file.exists():
        print(f"Error: {wasm_api_file} not found. Run scripts/audit_wasm_api.py first.")
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

    # Write report
    output_file = project_root / "PARITY_AUDIT.md"
    output_file.write_text(report)

    print(f"Parity audit report generated: {output_file}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
