#!/usr/bin/env python3
"""Extract all public APIs from finstack-py Python bindings.

This script analyzes the Python binding source files to extract:
- Classes and their methods
- Functions
- Module structure
- Properties and getters

Output: JSON file with complete API inventory.
"""

from dataclasses import asdict, dataclass
import json
import re
from pathlib import Path
import sys
from typing import Any


@dataclass
class MethodInfo:
    """Information about a class method."""

    name: str
    is_static: bool
    is_property: bool
    is_classmethod: bool
    signature: str = ""


@dataclass
class ClassInfo:
    """Information about a class."""

    name: str
    methods: list[MethodInfo]
    properties: list[str]
    is_exported: bool = True


@dataclass
class FunctionInfo:
    """Information about a standalone function."""

    name: str
    signature: str = ""


@dataclass
class ModuleInfo:
    """Information about a module."""

    name: str
    path: str
    classes: list[ClassInfo]
    functions: list[FunctionInfo]
    submodules: list[str]


class PythonAPIExtractor:
    """Extract API information from Python binding source files."""

    def __init__(self, src_root: Path) -> None:
        """Initialize the extractor with the source root directory."""
        self.src_root = src_root
        self.modules: dict[str, ModuleInfo] = {}

    def extract_from_rust_file(self, rust_file: Path) -> ModuleInfo:
        """Extract API info from a Rust file with PyO3 bindings.

        This parses Rust source looking for:
        - #[pyclass] declarations
        - #[pymethods] blocks
        - #[pyfunction] declarations
        - #[pymodule] declarations
        """
        content = rust_file.read_text()
        module_name = rust_file.stem

        classes = []
        functions = []

        # Find all #[pyclass] declarations
        lines = content.split("\n")
        i = 0
        while i < len(lines):
            line = lines[i].strip()

            # Look for pyclass
            if "#[pyclass" in line or line.startswith("pub struct Py"):
                # Extract class name
                class_name = self._extract_class_name(lines, i)
                if class_name:
                    methods = self._extract_methods(lines, i, class_name)
                    classes.append(
                        ClassInfo(
                            name=class_name, methods=methods, properties=[m.name for m in methods if m.is_property]
                        )
                    )

            # Look for pyfunction (may be `#[pyfunction]` or `#[pyfunction(...)]`)
            elif line.startswith("#[pyfunction"):
                # Scan forward past additional attributes (e.g. `#[pyo3(signature=...)]`)
                # to the first `pub fn` or `fn` line.
                func_name = ""
                for j in range(i + 1, min(i + 10, len(lines))):
                    candidate = lines[j].strip()
                    if candidate.startswith("#[") or candidate.startswith("//") or not candidate:
                        continue
                    func_name = self._extract_function_name(candidate)
                    break
                if func_name:
                    functions.append(FunctionInfo(name=func_name))

            i += 1

        return ModuleInfo(
            name=module_name,
            path=str(rust_file.relative_to(self.src_root)),
            classes=classes,
            functions=functions,
            submodules=[],
        )

    def _extract_class_name(self, lines: list[str], start_idx: int) -> str:
        """Extract class name from pyclass declaration.

        Prefers the explicit `name = "..."` argument on `#[pyclass(...)]` if
        present; otherwise falls back to the Rust struct name (stripping any
        leading `Py` prefix convention).
        """
        # First try to find `name = "Foo"` in the decorator itself.
        decorator_blob = " ".join(lines[start_idx : min(start_idx + 3, len(lines))])
        name_match = re.search(r'name\s*=\s*"([^"]+)"', decorator_blob)
        if name_match:
            return name_match.group(1)

        # Otherwise, look ahead a few lines for the struct definition.
        for i in range(start_idx, min(start_idx + 8, len(lines))):
            line = lines[i].strip()
            if line.startswith("pub struct ") or line.startswith("struct "):
                parts = line.replace("pub ", "", 1).split()
                if len(parts) >= 2:
                    name = parts[1].rstrip("{").split("<", 1)[0].split("(", 1)[0]
                    if name.startswith("Py") and len(name) > 2 and name[2].isupper():
                        return name[2:]
                    return name
        return ""

    def _extract_methods(self, lines: list[str], class_start: int, _class_name: str) -> list[MethodInfo]:
        """Extract methods from a pymethods block."""
        methods = []
        in_pymethods = False
        i = class_start

        while i < len(lines):
            line = lines[i].strip()

            # Start of pymethods block
            if "#[pymethods]" in line:
                in_pymethods = True
                i += 1
                continue

            # End of impl block
            if in_pymethods and line == "}":
                break

            # Inside pymethods block
            if in_pymethods:
                is_property = "#[getter]" in line or "#[pyo3(get)]" in line
                is_static = "#[staticmethod]" in line
                is_classmethod = "#[classmethod]" in line
                is_new = "#[new]" in line

                # Look for function definition
                if "pub fn " in line or "fn " in lines[min(i + 1, len(lines) - 1)]:
                    # Get next line if decorator is on separate line
                    func_line = line if "fn " in line else lines[min(i + 1, len(lines) - 1)]
                    method_name = self._extract_method_name(func_line)

                    if method_name and not method_name.startswith("_"):
                        methods.append(
                            MethodInfo(
                                name=method_name,
                                is_static=is_static or is_new,
                                is_property=is_property,
                                is_classmethod=is_classmethod,
                            )
                        )

            i += 1

        return methods

    def _extract_function_name(self, line: str) -> str:
        """Extract function name from function definition (pub or private)."""
        # PyO3 bindings use `fn` or `pub fn`; `#[pyfunction]` fns are often private.
        marker = "pub fn " if "pub fn " in line else ("fn " if line.startswith(("fn ", "async fn ", "pub async fn ")) or " fn " in line else "")
        if not marker:
            return ""
        after = line.split(marker, 1)[1]
        return after.split("(", 1)[0].split("<", 1)[0].strip()

    def _extract_method_name(self, line: str) -> str:
        """Extract method name from method definition."""
        if "fn " in line:
            # Extract: pub fn method_name( -> method_name
            parts = line.split("fn ", maxsplit=1)[1].split("(", maxsplit=1)[0].strip()
            return parts
        return ""

    def scan_directory(self, directory: Path, module_prefix: str = "") -> dict[str, Any]:
        """Recursively scan directory for binding files."""
        api_tree = {"modules": {}, "classes": [], "functions": []}

        for rust_file in directory.glob("*.rs"):
            # lib.rs is only a registration shim; mod.rs frequently hosts real
            # `#[pyfunction]` definitions (e.g. scenarios/mod.rs) and must be scanned.
            if rust_file.name == "lib.rs":
                continue

            module_info = self.extract_from_rust_file(rust_file)
            module_name = f"{module_prefix}.{module_info.name}" if module_prefix else module_info.name

            api_tree["modules"][module_name] = {
                "path": module_info.path,
                "classes": [asdict(c) for c in module_info.classes],
                "functions": [asdict(f) for f in module_info.functions],
            }

            # Collect all classes and functions at top level too
            api_tree["classes"].extend([c.name for c in module_info.classes])
            api_tree["functions"].extend([f.name for f in module_info.functions])

        # Recursively process subdirectories
        for subdir in directory.iterdir():
            if subdir.is_dir() and not subdir.name.startswith("_"):
                sub_prefix = f"{module_prefix}.{subdir.name}" if module_prefix else subdir.name
                sub_tree = self.scan_directory(subdir, sub_prefix)
                api_tree["modules"].update(sub_tree["modules"])
                api_tree["classes"].extend(sub_tree["classes"])
                api_tree["functions"].extend(sub_tree["functions"])

        return api_tree

    def extract_all(self) -> dict[str, Any]:
        """Extract all APIs from the Python bindings."""
        result = {"binding": "python", "source_root": str(self.src_root), "api": {}}

        # Scan all crate-domain subdirectories under bindings/
        for module_path in sorted(self.src_root.iterdir()):
            if not module_path.is_dir() or module_path.name.startswith("_"):
                continue
            result["api"][module_path.name] = self.scan_directory(module_path, module_path.name)

        return result


def main() -> int:
    """Main entry point."""
    # Find project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    py_src = project_root / "finstack-py" / "src" / "bindings"

    if not py_src.exists():
        return 1

    extractor = PythonAPIExtractor(py_src)
    api_data = extractor.extract_all()

    # Write to .audit/ (gitignored) — never write to tracked repo files
    audit_dir = project_root / ".audit"
    audit_dir.mkdir(exist_ok=True)
    output_file = audit_dir / "python_api.json"
    with output_file.open("w") as f:
        json.dump(api_data, f, indent=2)

    # Print summary
    sum(len(mod.get("classes", [])) for mod in api_data["api"].values())
    sum(len(mod.get("functions", [])) for mod in api_data["api"].values())

    return 0


if __name__ == "__main__":
    sys.exit(main())
