#!/usr/bin/env python3
"""Extract public API surface from finstack Rust library.

This script analyzes the Rust library to extract only the public API surface:
- Public modules declared in lib.rs (pub mod)
- Public re-exports from lib.rs (pub use)
- Public items exported from public modules

This focuses on the actual API surface that users can access, not internal
implementation details marked as pub.
"""

from dataclasses import dataclass
import json
from pathlib import Path
import re
import sys
from typing import Any


@dataclass
class TypeInfo:
    """Information about a public type."""

    name: str
    kind: str  # "struct", "enum", "trait", "type", "const"
    path: str = ""  # Module path where it's exported


@dataclass
class FunctionInfo:
    """Information about a public function."""

    name: str
    path: str = ""  # Module path where it's exported


@dataclass
class ModuleExports:
    """Public exports from a module."""

    modules: list[str]  # Public submodules
    types: list[str]  # Re-exported types
    functions: list[str]  # Re-exported functions
    re_exports: dict[str, list[str]]  # pub use statements by module path


class RustAPIExtractor:
    """Extract public API surface from Rust library."""

    def __init__(self, src_root: Path) -> None:
        """Initialize the extractor with the source root directory."""
        self.src_root = src_root
        self.base_path = src_root

    def extract_pub_use_items(self, line: str) -> list[str]:
        """Extract items from pub use statements."""
        items = []

        # Handle: pub use module::{Item1, Item2};
        brace_match = re.search(r"pub\s+use\s+[^{]+\{([^}]+)\}", line)
        if brace_match:
            items_str = brace_match.group(1)
            for raw_item in items_str.split(","):
                item_clean = raw_item.strip()
                if not item_clean:
                    continue
                # Handle "Item as Alias"
                if " as " in item_clean:
                    alias = item_clean.split(" as ")[1].strip()
                    items.append(alias)
                else:
                    items.append(item_clean)
            return items

        # Handle: pub use module::Item;
        # Handle: pub use module::Item as Alias;
        single_match = re.search(r"pub\s+use\s+[^:]+::(\w+)(?:\s+as\s+(\w+))?;", line)
        if single_match:
            name = single_match.group(2) if single_match.group(2) else single_match.group(1)
            items.append(name)

        return items

    def extract_exports_from_lib(self, lib_file: Path) -> ModuleExports:
        """Extract public API surface from a lib.rs file."""
        content = lib_file.read_text()
        exports = ModuleExports(modules=[], types=[], functions=[], re_exports={})

        lines = content.split("\n")
        i = 0
        while i < len(lines):
            line = lines[i].strip()

            # Coalesce multi-line `pub use foo::{ ... }` blocks onto one logical line
            if line.startswith("pub use ") and "{" in line and "}" not in line:
                combined = [line]
                i += 1
                while i < len(lines):
                    next_line = lines[i].strip()
                    combined.append(next_line)
                    if "}" in next_line:
                        break
                    i += 1
                line = " ".join(combined)

            # Look for pub mod (public modules)
            if line.startswith("pub mod ") and not line.startswith("pub(crate) mod "):
                mod_match = re.search(r"pub\s+mod\s+(\w+)", line)
                if mod_match:
                    exports.modules.append(mod_match.group(1))

            # Look for pub use (re-exports)
            if line.startswith("pub use "):
                items = self.extract_pub_use_items(line)
                # Determine if these are types or functions (heuristic)
                for item in items:
                    # Functions typically start with lowercase, but this is imperfect
                    # We'll categorize based on context
                    if item[0].isupper():
                        exports.types.append(item)
                    else:
                        exports.functions.append(item)
                # Store the full re-export path
                path_match = re.search(r"pub\s+use\s+([^:;]+)", line)
                if path_match:
                    path = path_match.group(1).strip()
                    exports.re_exports[path] = items

            # Extract pub struct/enum/trait/type/const declarations (public API items)
            elif re.match(r"^pub\s+struct\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+struct\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+enum\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+enum\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+trait\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+trait\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+type\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+type\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+const\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+const\s+(\w+)", line)
                if match:
                    exports.types.append(match.group(1))

            # Extract pub fn declarations (public API functions)
            elif (
                re.match(r"^pub\s+fn\s+\w+", line) or re.match(r"^pub\s+async\s+fn\s+\w+", line)
            ) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+(?:async\s+)?fn\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.functions.append(match.group(1))

            i += 1

        return exports

    def extract_module_exports(self, mod_file: Path) -> ModuleExports:
        """Extract public exports from a module file (mod.rs or other .rs files)."""
        content = mod_file.read_text()
        exports = ModuleExports(modules=[], types=[], functions=[], re_exports={})

        lines = content.split("\n")
        i = 0
        while i < len(lines):
            line = lines[i].strip()

            # Coalesce multi-line `pub use foo::{ ... }` blocks onto one logical line
            if line.startswith("pub use ") and "{" in line and "}" not in line:
                combined = [line]
                i += 1
                while i < len(lines):
                    next_line = lines[i].strip()
                    combined.append(next_line)
                    if "}" in next_line:
                        break
                    i += 1
                line = " ".join(combined)

            # Look for pub mod (public submodules)
            if line.startswith("pub mod ") and not line.startswith("pub(crate) mod "):
                mod_match = re.search(r"pub\s+mod\s+(\w+)", line)
                if mod_match:
                    exports.modules.append(mod_match.group(1))

            # Look for pub use (re-exports)
            if line.startswith("pub use "):
                items = self.extract_pub_use_items(line)
                for item in items:
                    if item[0].isupper():
                        exports.types.append(item)
                    else:
                        exports.functions.append(item)
                # Store the full re-export path
                path_match = re.search(r"pub\s+use\s+([^:;]+)", line)
                if path_match:
                    path = path_match.group(1).strip()
                    exports.re_exports[path] = items

            # Extract pub struct/enum/trait/type/const declarations (public API items)
            elif re.match(r"^pub\s+struct\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+struct\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+enum\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+enum\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+trait\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+trait\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+type\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+type\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.types.append(match.group(1))

            elif re.match(r"^pub\s+const\s+\w+", line) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+const\s+(\w+)", line)
                if match:
                    exports.types.append(match.group(1))

            # Extract pub fn declarations (public API functions)
            elif (
                re.match(r"^pub\s+fn\s+\w+", line) or re.match(r"^pub\s+async\s+fn\s+\w+", line)
            ) and not line.startswith("pub(crate)"):
                match = re.search(r"pub\s+(?:async\s+)?fn\s+(\w+)", line)
                if match and not match.group(1).startswith("_"):
                    exports.functions.append(match.group(1))

            i += 1

        return exports

    def collect_public_api_from_module(self, module_path: Path, module_name: str, visited: set[str]) -> dict[str, Any]:
        """Recursively collect public API from a module and its submodules."""
        if module_name in visited:
            return {}
        visited.add(module_name)

        result = {
            "path": str(module_path.relative_to(self.base_path)),
            "types": [],
            "functions": [],
            "modules": {},
        }

        # Check for mod.rs first
        mod_rs = module_path / "mod.rs"
        if mod_rs.exists():
            exports = self.extract_module_exports(mod_rs)
            result["types"].extend(exports.types)
            result["functions"].extend(exports.functions)
            result["re_exports"] = exports.re_exports

            # Process public submodules
            for submod_name in exports.modules:
                submod_path = module_path / submod_name
                if submod_path.is_dir():
                    submod_full_name = f"{module_name}::{submod_name}"
                    submod_result = self.collect_public_api_from_module(submod_path, submod_full_name, visited)
                    if submod_result:
                        result["modules"][submod_name] = submod_result
                elif (module_path / f"{submod_name}.rs").exists():
                    # Inline module - extract its public API
                    submod_file = module_path / f"{submod_name}.rs"
                    submod_exports = self.extract_module_exports(submod_file)
                    result["modules"][submod_name] = {
                        "path": str(submod_file.relative_to(self.base_path)),
                        "types": submod_exports.types,
                        "functions": submod_exports.functions,
                        "re_exports": submod_exports.re_exports,
                        "modules": {},
                    }
        # Also check for standalone .rs files in the directory (public API items)
        elif module_path.is_dir():
            for rs_file in module_path.glob("*.rs"):
                if rs_file.name == "mod.rs":
                    continue
                # Extract public items from this file
                file_exports = self.extract_module_exports(rs_file)
                if file_exports.types or file_exports.functions:
                    file_name = rs_file.stem
                    result["modules"][file_name] = {
                        "path": str(rs_file.relative_to(self.base_path)),
                        "types": file_exports.types,
                        "functions": file_exports.functions,
                        "re_exports": file_exports.re_exports,
                        "modules": {},
                    }
        else:
            # Check for lib.rs (for crate root)
            lib_rs = module_path / "lib.rs"
            if lib_rs.exists():
                exports = self.extract_exports_from_lib(lib_rs)
                result["types"].extend(exports.types)
                result["functions"].extend(exports.functions)
                result["re_exports"] = exports.re_exports

                # Process public submodules
                for submod_name in exports.modules:
                    submod_path = module_path / submod_name
                    if submod_path.is_dir():
                        submod_full_name = f"{module_name}::{submod_name}"
                        submod_result = self.collect_public_api_from_module(submod_path, submod_full_name, visited)
                        if submod_result:
                            result["modules"][submod_name] = submod_result
                    elif (module_path / f"{submod_name}.rs").exists():
                        # Inline module - extract its public API
                        submod_file = module_path / f"{submod_name}.rs"
                        submod_exports = self.extract_module_exports(submod_file)
                        result["modules"][submod_name] = {
                            "path": str(submod_file.relative_to(self.base_path)),
                            "types": submod_exports.types,
                            "functions": submod_exports.functions,
                            "re_exports": submod_exports.re_exports,
                            "modules": {},
                        }

        return result

    def extract_all(self) -> dict[str, Any]:
        """Extract public API surface from all Rust crates."""
        result = {"binding": "rust", "source_root": str(self.src_root), "api": {}}

        # Extract from main lib.rs if it exists
        main_lib = self.src_root / "src" / "lib.rs"
        if main_lib.exists():
            main_exports = self.extract_exports_from_lib(main_lib)
            result["exports"] = {
                "modules": main_exports.modules,
                "types": main_exports.types,
                "functions": main_exports.functions,
            }

        # Process major crates
        crate_dirs = ["core", "valuations", "statements", "scenarios", "portfolio", "io"]
        for crate_dir in crate_dirs:
            crate_path = self.src_root / crate_dir / "src"
            if not crate_path.exists():
                continue

            crate_lib = crate_path / "lib.rs"
            if not crate_lib.exists():
                continue

            # Extract exports from crate's lib.rs
            crate_exports = self.extract_exports_from_lib(crate_lib)

            # Collect public API from public modules
            crate_api = {
                "exports": {
                    "modules": crate_exports.modules,
                    "types": crate_exports.types,
                    "functions": crate_exports.functions,
                    "re_exports": crate_exports.re_exports,
                },
                "modules": {},
            }

            visited = set()
            for mod_name in crate_exports.modules:
                mod_path = crate_path / mod_name
                if mod_path.is_dir():
                    mod_result = self.collect_public_api_from_module(mod_path, mod_name, visited)
                    if mod_result:
                        crate_api["modules"][mod_name] = mod_result
                elif (crate_path / f"{mod_name}.rs").exists():
                    # Inline module
                    mod_file = crate_path / f"{mod_name}.rs"
                    mod_exports = self.extract_module_exports(mod_file)
                    crate_api["modules"][mod_name] = {
                        "path": str(mod_file.relative_to(self.base_path)),
                        "types": mod_exports.types,
                        "functions": mod_exports.functions,
                        "re_exports": mod_exports.re_exports,
                        "modules": {},
                    }

            result["api"][crate_dir] = crate_api

        return result

    def _flatten_types_and_functions(self, api_data: dict[str, Any]) -> tuple[list[str], list[str]]:
        """Flatten all types and functions from the API tree."""
        types = []
        functions = []

        def collect_from_dict(d: dict[str, Any]) -> None:
            types.extend(d.get("types", []))
            functions.extend(d.get("functions", []))
            for mod_data in d.get("modules", {}).values():
                collect_from_dict(mod_data)

        for crate_data in api_data.get("api", {}).values():
            # Collect from exports
            exports = crate_data.get("exports", {})
            types.extend(exports.get("types", []))
            functions.extend(exports.get("functions", []))
            # Collect from modules
            collect_from_dict(crate_data)

        return types, functions


def main() -> int:
    """Main entry point."""
    # Find project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    rust_src = project_root / "finstack"

    if not rust_src.exists():
        return 1

    extractor = RustAPIExtractor(rust_src)
    api_data = extractor.extract_all()

    # Write to output file
    output_file = script_dir / "rust_api.json"
    with output_file.open("w") as f:
        json.dump(api_data, f, indent=2)

    # Print summary
    _types, _functions = extractor._flatten_types_and_functions(api_data)

    return 0


if __name__ == "__main__":
    sys.exit(main())
