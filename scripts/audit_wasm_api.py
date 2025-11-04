#!/usr/bin/env python3
"""Extract all public APIs from finstack-wasm WASM bindings.

This script analyzes the WASM binding source files to extract:
- Exported classes (with #[wasm_bindgen])
- Exported methods
- Exported functions
- Module structure

Output: JSON file with complete API inventory.
"""

from dataclasses import asdict, dataclass
import json
from pathlib import Path
import re
import sys
from typing import Any


@dataclass
class MethodInfo:
    """Information about a class method."""
    name: str
    is_static: bool
    is_constructor: bool
    is_getter: bool
    js_name: str = ""  # JavaScript name if different


@dataclass
class ClassInfo:
    """Information about a WASM-bindgen class."""
    name: str
    js_name: str  # JavaScript-exported name
    methods: list[MethodInfo]
    properties: list[str]


@dataclass
class FunctionInfo:
    """Information about a standalone WASM function."""
    name: str
    js_name: str  # JavaScript-exported name


@dataclass
class ModuleInfo:
    """Information about a module."""
    name: str
    path: str
    classes: list[ClassInfo]
    functions: list[FunctionInfo]


class WasmAPIExtractor:
    """Extract API information from WASM binding source files."""

    def __init__(self, src_root: Path) -> None:
        """Initialize the extractor with the source root directory."""
        self.src_root = src_root

    def extract_from_rust_file(self, rust_file: Path) -> ModuleInfo:
        """Extract API info from a Rust file with wasm-bindgen bindings.

        Looks for:
        - #[wasm_bindgen] structs
        - #[wasm_bindgen] impl blocks
        - #[wasm_bindgen] functions
        """
        content = rust_file.read_text()
        module_name = rust_file.stem

        classes = []
        functions = []

        lines = content.split("\n")
        i = 0

        while i < len(lines):
            line = lines[i].strip()

            # Look for wasm_bindgen on struct
            if "#[wasm_bindgen" in line:
                # Check if next few lines have struct or fn
                for j in range(i + 1, min(i + 5, len(lines))):
                    next_line = lines[j].strip()

                    # Found a struct
                    if next_line.startswith("pub struct "):
                        class_info = self._extract_class(lines, i, j)
                        if class_info:
                            classes.append(class_info)
                        break

                    # Found a function
                    elif "pub fn " in next_line or "fn " in next_line:
                        func_info = self._extract_function(lines, i, j)
                        if func_info:
                            functions.append(func_info)
                        break

            i += 1

        return ModuleInfo(
            name=module_name,
            path=str(rust_file.relative_to(self.src_root)),
            classes=classes,
            functions=functions
        )

    def _extract_js_name(self, decorator_line: str) -> str:
        """Extract js_name from wasm_bindgen decorator."""
        match = re.search(r'js_name\s*=\s*"([^"]+)"', decorator_line)
        if match:
            return match.group(1)
        match = re.search(r"js_name\s*=\s*(\w+)", decorator_line)
        if match:
            return match.group(1)
        return ""

    def _extract_class(self, lines: list[str], decorator_idx: int, struct_idx: int) -> ClassInfo:
        """Extract class information from struct declaration."""
        # Get js_name from decorator
        decorator = lines[decorator_idx].strip()
        js_name = self._extract_js_name(decorator)

        # Get struct name
        struct_line = lines[struct_idx].strip()
        match = re.search(r"pub struct (\w+)", struct_line)
        if not match:
            return None

        struct_name = match.group(1)

        # Remove Js prefix if present
        rust_name = struct_name[2:] if struct_name.startswith("Js") else struct_name

        # Use js_name if specified, otherwise use rust name without Js prefix
        final_name = js_name if js_name else rust_name

        # Find associated impl block with methods
        methods = self._find_methods_for_struct(lines, struct_name)

        return ClassInfo(
            name=rust_name,
            js_name=final_name,
            methods=methods,
            properties=[m.name for m in methods if m.is_getter]
        )

    def _find_methods_for_struct(self, lines: list[str], struct_name: str) -> list[MethodInfo]:
        """Find all methods in wasm_bindgen impl blocks for a struct."""
        methods = []
        i = 0

        while i < len(lines):
            line = lines[i].strip()

            # Look for impl blocks for this struct
            if f"impl {struct_name}" in line or "impl<" in line:
                # Check if there's a #[wasm_bindgen] before it
                has_wasm_bindgen = False
                for j in range(max(0, i - 3), i):
                    if "#[wasm_bindgen]" in lines[j]:
                        has_wasm_bindgen = True
                        break

                if has_wasm_bindgen:
                    # Extract methods from this impl block
                    methods.extend(self._extract_methods_from_impl(lines, i))

            i += 1

        return methods

    def _extract_methods_from_impl(self, lines: list[str], impl_start: int) -> list[MethodInfo]:
        """Extract methods from a wasm_bindgen impl block."""
        methods = []
        i = impl_start
        brace_count = 0
        started = False

        while i < len(lines):
            line = lines[i].strip()

            # Track braces
            if "{" in line:
                brace_count += line.count("{")
                started = True
            if "}" in line:
                brace_count -= line.count("}")

            # End of impl block
            if started and brace_count == 0:
                break

            # Look for method markers
            is_constructor = "#[wasm_bindgen(constructor)]" in line
            is_getter = "#[wasm_bindgen(getter" in line
            is_static = "#[wasm_bindgen(js_name" in line or "static" in line

            # Extract js_name if present
            js_name = ""
            if "#[wasm_bindgen" in line:
                js_name = self._extract_js_name(line)

            # Look for function definition
            if "pub fn " in line or (i + 1 < len(lines) and "pub fn " in lines[i + 1]):
                func_line = line if "pub fn " in line else lines[i + 1]
                method_name = self._extract_method_name(func_line)

                if method_name and not method_name.startswith("_"):
                    methods.append(MethodInfo(
                        name=method_name,
                        is_static=is_static,
                        is_constructor=is_constructor,
                        is_getter=is_getter,
                        js_name=js_name if js_name else method_name
                    ))

            i += 1

        return methods

    def _extract_function(self, lines: list[str], decorator_idx: int, func_idx: int) -> FunctionInfo:
        """Extract standalone function information."""
        decorator = lines[decorator_idx].strip()
        js_name = self._extract_js_name(decorator)

        func_line = lines[func_idx].strip()
        func_name = self._extract_method_name(func_line)

        if not func_name:
            return None

        return FunctionInfo(
            name=func_name,
            js_name=js_name if js_name else func_name
        )

    def _extract_method_name(self, line: str) -> str:
        """Extract method/function name from definition."""
        match = re.search(r"pub fn (\w+)", line)
        if match:
            return match.group(1)
        match = re.search(r"fn (\w+)", line)
        if match:
            return match.group(1)
        return ""

    def extract_exports_from_lib(self, lib_file: Path) -> dict[str, list[str]]:
        """Extract pub use exports from lib.rs to get the public API surface."""
        content = lib_file.read_text()
        exports = {
            "types": [],
            "functions": []
        }

        for raw_line in content.split("\n"):
            line = raw_line.strip()

            # Look for pub use statements
            if line.startswith("pub use"):
                # Extract exported items
                # pub use module::{Type1, Type2, function1 as alias};
                match = re.search(r"pub use [^{]+\{([^}]+)\}", line)
                if match:
                    items = match.group(1).split(",")
                    for raw_item in items:
                        item = raw_item.strip()
                        # Handle "Type as Alias"
                        if " as " in item:
                            alias = item.split(" as ")[1].strip()
                            exports["types"].append(alias)
                        else:
                            exports["types"].append(item)

                # pub use module::Type;
                match = re.search(r"pub use [^:]+::(\w+)(?:\s+as\s+(\w+))?;", line)
                if match:
                    name = match.group(2) if match.group(2) else match.group(1)
                    exports["types"].append(name)

        return exports

    def scan_directory(self, directory: Path, module_prefix: str = "") -> dict[str, Any]:
        """Recursively scan directory for binding files."""
        api_tree = {
            "modules": {},
            "classes": [],
            "functions": []
        }

        for rust_file in directory.glob("*.rs"):
            if rust_file.name in ["mod.rs", "wrapper.rs"]:
                continue

            module_info = self.extract_from_rust_file(rust_file)
            module_name = f"{module_prefix}.{module_info.name}" if module_prefix else module_info.name

            api_tree["modules"][module_name] = {
                "path": module_info.path,
                "classes": [asdict(c) for c in module_info.classes],
                "functions": [asdict(f) for f in module_info.functions]
            }

            api_tree["classes"].extend([c.js_name for c in module_info.classes])
            api_tree["functions"].extend([f.js_name for f in module_info.functions])

        # Recursively process subdirectories
        for subdir in directory.iterdir():
            if subdir.is_dir() and not subdir.name.startswith("_") and subdir.name != "target":
                sub_prefix = f"{module_prefix}.{subdir.name}" if module_prefix else subdir.name
                sub_tree = self.scan_directory(subdir, sub_prefix)
                api_tree["modules"].update(sub_tree["modules"])
                api_tree["classes"].extend(sub_tree["classes"])
                api_tree["functions"].extend(sub_tree["functions"])

        return api_tree

    def extract_all(self) -> dict[str, Any]:
        """Extract all APIs from the WASM bindings."""
        result = {
            "binding": "wasm",
            "source_root": str(self.src_root),
            "api": {}
        }

        # First, extract exports from lib.rs
        lib_file = self.src_root / "lib.rs"
        if lib_file.exists():
            result["exports"] = self.extract_exports_from_lib(lib_file)

        # Scan major modules
        for module_dir in ["core", "valuations", "statements", "scenarios", "portfolio"]:
            module_path = self.src_root / module_dir
            if module_path.exists():
                result["api"][module_dir] = self.scan_directory(module_path, module_dir)

        return result


def main() -> int:
    """Main entry point."""
    # Find project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    wasm_src = project_root / "finstack-wasm" / "src"

    if not wasm_src.exists():
        return 1

    extractor = WasmAPIExtractor(wasm_src)
    api_data = extractor.extract_all()

    # Write to output file
    output_file = project_root / "scripts" / "wasm_api.json"
    with output_file.open("w") as f:
        json.dump(api_data, f, indent=2)

    # Print summary
    sum(len(mod.get("classes", [])) for mod in api_data["api"].values())
    sum(len(mod.get("functions", [])) for mod in api_data["api"].values())


    return 0


if __name__ == "__main__":
    sys.exit(main())

