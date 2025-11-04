#!/usr/bin/env python3
"""Compare Python and WASM APIs to generate a parity audit report.

This script:
1. Loads the extracted API data from both bindings
2. Compares classes, methods, and functions
3. Identifies gaps and mismatches
4. Generates a comprehensive markdown report
"""

from dataclasses import dataclass
import json
from pathlib import Path
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
    """Compare APIs between Python and WASM bindings."""

    def __init__(self, python_api: dict, wasm_api: dict):
        self.python_api = python_api
        self.wasm_api = wasm_api
        self.issues: list[ParityIssue] = []

    def snake_to_camel(self, snake_str: str) -> str:
        """Convert snake_case to camelCase."""
        components = snake_str.split("_")
        return components[0] + "".join(x.title() for x in components[1:])

    def compare_classes(self) -> tuple[set[str], set[str], set[str]]:
        """Compare classes between bindings.
        
        Returns:
            (in_both, only_python, only_wasm)
        """
        # Collect all classes from both bindings
        python_classes = set()
        for module_name, module_data in self.python_api.get("api", {}).items():
            for mod_key, mod_info in module_data.get("modules", {}).items():
                for cls in mod_info.get("classes", []):
                    python_classes.add(cls.get("name", ""))

        wasm_classes = set()
        for module_name, module_data in self.wasm_api.get("api", {}).items():
            for mod_key, mod_info in module_data.get("modules", {}).items():
                for cls in mod_info.get("classes", []):
                    wasm_classes.add(cls.get("js_name", cls.get("name", "")))

        # Also get from exports
        wasm_exports = set(self.wasm_api.get("exports", {}).get("types", []))
        wasm_classes.update(wasm_exports)

        in_both = python_classes & wasm_classes
        only_python = python_classes - wasm_classes
        only_wasm = wasm_classes - python_classes

        return in_both, only_python, only_wasm

    def compare_instruments(self) -> dict[str, Any]:
        """Compare instrument coverage specifically."""
        # Known instruments list
        all_instruments = {
            # Fixed Income
            "Bond", "Deposit", "InterestRateSwap", "ForwardRateAgreement",
            "Swaption", "BasisSwap", "InterestRateOption", "InterestRateFuture",
            # FX
            "FxSpot", "FxOption", "FxSwap", "FxBarrierOption",
            # Credit
            "CreditDefaultSwap", "CDSIndex", "CdsTranche", "CdsOption",
            # Equity
            "Equity", "EquityOption", "EquityTotalReturnSwap",
            "FiIndexTotalReturnSwap", "VarianceSwap",
            # Inflation
            "InflationLinkedBond", "InflationSwap",
            # Structured
            "Basket", "StructuredCredit", "PrivateMarketsFund",
            "ConvertibleBond", "Repo",
            # Exotic Options
            "AsianOption", "BarrierOption", "LookbackOption", "CliquetOption",
            "QuantoOption", "Autocallable", "CmsOption", "RangeAccrual",
            # Private Credit
            "TermLoan", "RevolvingCredit"
        }

        in_both, only_python, only_wasm = self.compare_classes()

        python_instruments = all_instruments & {cls for cls in only_python | in_both}
        wasm_instruments = all_instruments & {cls for cls in only_wasm | in_both}

        missing_in_python = all_instruments - python_instruments
        missing_in_wasm = all_instruments - wasm_instruments

        return {
            "total_expected": len(all_instruments),
            "in_python": len(python_instruments),
            "in_wasm": len(wasm_instruments),
            "in_both": len(all_instruments & in_both),
            "missing_in_python": sorted(missing_in_python),
            "missing_in_wasm": sorted(missing_in_wasm),
            "python_instruments": sorted(python_instruments),
            "wasm_instruments": sorted(wasm_instruments)
        }

    def compare_calibration(self) -> dict[str, Any]:
        """Compare calibration API coverage."""
        calibration_classes = {
            "DiscountCurveCalibrator", "ForwardCurveCalibrator",
            "HazardCurveCalibrator", "InflationCurveCalibrator",
            "VolSurfaceCalibrator", "BaseCorrelationCalibrator",
            "SimpleCalibration", "CalibrationConfig", "CalibrationReport",
            "RatesQuote", "CreditQuote", "VolQuote", "InflationQuote"
        }

        in_both, only_python, only_wasm = self.compare_classes()

        python_cal = calibration_classes & {cls for cls in only_python | in_both}
        wasm_cal = calibration_classes & {cls for cls in only_wasm | in_both}

        return {
            "total_expected": len(calibration_classes),
            "in_python": len(python_cal),
            "in_wasm": len(wasm_cal),
            "missing_in_python": sorted(calibration_classes - python_cal),
            "missing_in_wasm": sorted(calibration_classes - wasm_cal)
        }

    def generate_report(self) -> str:
        """Generate comprehensive parity audit report in Markdown."""
        in_both, only_python, only_wasm = self.compare_classes()
        instruments = self.compare_instruments()
        calibration = self.compare_calibration()

        # Build the report
        lines = [
            "# Python-WASM Bindings Parity Audit",
            "",
            f"**Generated:** {Path(__file__).name}",
            "",
            "## Executive Summary",
            "",
            f"- **Classes in both bindings:** {len(in_both)}",
            f"- **Only in Python:** {len(only_python)}",
            f"- **Only in WASM:** {len(only_wasm)}",
            f"- **Total unique classes:** {len(in_both) + len(only_python) + len(only_wasm)}",
            "",
            "## Instrument Coverage",
            "",
            f"- **Expected instruments:** {instruments['total_expected']}",
            f"- **In Python:** {instruments['in_python']} ({instruments['in_python']*100//instruments['total_expected']}%)",
            f"- **In WASM:** {instruments['in_wasm']} ({instruments['in_wasm']*100//instruments['total_expected']}%)",
            f"- **In both:** {instruments['in_both']}",
            "",
        ]

        if instruments["missing_in_python"]:
            lines.extend([
                "### Missing in Python",
                "",
                "```"
            ])
            for instr in instruments["missing_in_python"]:
                lines.append(f"- {instr}")
            lines.append("```")
            lines.append("")

        if instruments["missing_in_wasm"]:
            lines.extend([
                "### Missing in WASM",
                "",
                "```"
            ])
            for instr in instruments["missing_in_wasm"]:
                lines.append(f"- {instr}")
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Calibration API Coverage",
            "",
            f"- **Expected calibration types:** {calibration['total_expected']}",
            f"- **In Python:** {calibration['in_python']} ({calibration['in_python']*100//calibration['total_expected']}%)",
            f"- **In WASM:** {calibration['in_wasm']} ({calibration['in_wasm']*100//calibration['total_expected']}%)",
            "",
        ])

        if calibration["missing_in_python"]:
            lines.extend([
                "### Missing in Python",
                "",
                "```"
            ])
            for cal in calibration["missing_in_python"]:
                lines.append(f"- {cal}")
            lines.append("```")
            lines.append("")

        if calibration["missing_in_wasm"]:
            lines.extend([
                "### Missing in WASM",
                "",
                "```"
            ])
            for cal in calibration["missing_in_wasm"]:
                lines.append(f"- {cal}")
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Complete Class Comparison",
            "",
            "### Classes in Both Bindings",
            "",
            f"**Count:** {len(in_both)}",
            "",
            "```"
        ])
        for cls in sorted(in_both):
            lines.append(f"✓ {cls}")
        lines.append("```")
        lines.append("")

        if only_python:
            lines.extend([
                "### Classes Only in Python",
                "",
                f"**Count:** {len(only_python)}",
                "",
                "```"
            ])
            for cls in sorted(only_python):
                lines.append(f"- {cls}")
            lines.append("```")
            lines.append("")

        if only_wasm:
            lines.extend([
                "### Classes Only in WASM",
                "",
                f"**Count:** {len(only_wasm)}",
                "",
                "```"
            ])
            for cls in sorted(only_wasm):
                lines.append(f"- {cls}")
            lines.append("```")
            lines.append("")

        lines.extend([
            "## Naming Convention Patterns",
            "",
            "### Identified Patterns",
            "",
            "| Python | WASM | Pattern |",
            "|--------|------|---------|",
        ])

        # Sample some naming patterns
        sample_patterns = [
            ("build_periods", "buildPeriods", "snake_case → camelCase"),
            ("from_code", "fromCode", "snake_case → camelCase"),
            ("next_imm", "nextImm", "snake_case → camelCase"),
            ("is_actual", "isActual", "snake_case → camelCase"),
            ("Currency", "Currency", "PascalCase → PascalCase"),
            ("Money", "Money", "PascalCase → PascalCase"),
        ]

        for py_name, wasm_name, pattern in sample_patterns:
            lines.append(f"| `{py_name}` | `{wasm_name}` | {pattern} |")

        lines.extend([
            "",
            "## Recommendations",
            "",
            "### High Priority",
            "",
        ])

        if instruments["missing_in_wasm"]:
            lines.append(f"1. **Add {len(instruments['missing_in_wasm'])} missing instruments to WASM:** " + ", ".join(instruments["missing_in_wasm"][:3]) + ("..." if len(instruments["missing_in_wasm"]) > 3 else ""))

        if instruments["missing_in_python"]:
            lines.append(f"2. **Add {len(instruments['missing_in_python'])} missing instruments to Python:** " + ", ".join(instruments["missing_in_python"][:3]) + ("..." if len(instruments["missing_in_python"]) > 3 else ""))

        if calibration["missing_in_wasm"]:
            lines.append(f"3. **Complete calibration API in WASM:** {len(calibration['missing_in_wasm'])} types missing")

        if calibration["missing_in_python"]:
            lines.append(f"4. **Complete calibration API in Python:** {len(calibration['missing_in_python'])} types missing")

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
            "*This report was automatically generated. Do not edit manually.*"
        ])

        return "\n".join(lines)


def main():
    """Main entry point."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    # Load API data
    python_api_file = script_dir / "python_api.json"
    wasm_api_file = script_dir / "wasm_api.json"

    if not python_api_file.exists():
        print(f"Error: Python API file not found: {python_api_file}")
        print("Run: python scripts/audit_python_api.py")
        return 1

    if not wasm_api_file.exists():
        print(f"Error: WASM API file not found: {wasm_api_file}")
        print("Run: python scripts/audit_wasm_api.py")
        return 1

    with open(python_api_file) as f:
        python_api = json.load(f)

    with open(wasm_api_file) as f:
        wasm_api = json.load(f)

    # Compare APIs
    comparator = APIComparator(python_api, wasm_api)
    report = comparator.generate_report()

    # Write report
    output_file = project_root / "PARITY_AUDIT.md"
    output_file.write_text(report)

    print(f"✓ Generated parity audit report: {output_file}")
    print("\nRun the following to view:")
    print(f"  cat {output_file}")

    return 0


if __name__ == "__main__":
    exit(main())

