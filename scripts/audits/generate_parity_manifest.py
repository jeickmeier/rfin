#!/usr/bin/env python3
"""Generate a parity manifest from Python .pyi stub files.

This script extracts the public API surface from Python type stubs (.pyi files)
to create a machine-readable manifest that serves as the parity contract for
WASM bindings.

The manifest includes:
- Module paths (e.g., finstack.core.market_data.bumps)
- Symbol names (classes, functions, constants)
- Method signatures with key markers (from_json/to_json, static methods, etc.)
- Property definitions

Output: JSON manifest file (parity_manifest.json)
"""

import ast
from dataclasses import asdict, dataclass, field
import json
from pathlib import Path
import sys
from typing import Any


@dataclass
class PropertyInfo:
    """Information about a class property."""

    name: str
    type_hint: str = ""
    is_readonly: bool = True


@dataclass
class MethodInfo:
    """Information about a class method."""

    name: str
    signature: str = ""
    is_static: bool = False
    is_classmethod: bool = False
    is_property: bool = False
    return_type: str = ""
    parameters: list[str] = field(default_factory=list)


@dataclass
class ClassInfo:
    """Information about a class."""

    name: str
    module: str
    methods: list[MethodInfo] = field(default_factory=list)
    properties: list[PropertyInfo] = field(default_factory=list)
    bases: list[str] = field(default_factory=list)
    has_from_json: bool = False
    has_to_json: bool = False
    has_from_state: bool = False
    has_to_state: bool = False
    is_enum: bool = False


@dataclass
class FunctionInfo:
    """Information about a standalone function."""

    name: str
    module: str
    signature: str = ""
    return_type: str = ""
    parameters: list[str] = field(default_factory=list)


@dataclass
class ConstantInfo:
    """Information about a module-level constant."""

    name: str
    module: str
    type_hint: str = ""


@dataclass
class ModuleInfo:
    """Information about a module."""

    name: str
    path: str
    classes: list[ClassInfo] = field(default_factory=list)
    functions: list[FunctionInfo] = field(default_factory=list)
    constants: list[ConstantInfo] = field(default_factory=list)
    submodules: list[str] = field(default_factory=list)
    all_exports: list[str] = field(default_factory=list)


class PyiExtractor(ast.NodeVisitor):
    """Extract API information from Python .pyi stub files."""

    def __init__(self, module_name: str):
        """Initialize the extractor for a given module."""
        self.module_name = module_name
        self.classes: list[ClassInfo] = []
        self.functions: list[FunctionInfo] = []
        self.constants: list[ConstantInfo] = []
        self.all_exports: list[str] = []
        self._current_class: ClassInfo | None = None

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        """Visit a class definition."""
        # Check if it's an enum-like class
        bases = [self._get_name(b) for b in node.bases]
        is_enum = any("Enum" in b for b in bases)

        class_info = ClassInfo(
            name=node.name,
            module=self.module_name,
            bases=bases,
            is_enum=is_enum,
        )
        self._current_class = class_info

        # Visit class body
        for item in node.body:
            if isinstance(item, ast.FunctionDef | ast.AsyncFunctionDef):
                method_info = self._extract_method(item)
                if method_info:
                    class_info.methods.append(method_info)
                    # Check for serialization methods
                    if method_info.name == "from_json":
                        class_info.has_from_json = True
                    elif method_info.name == "to_json":
                        class_info.has_to_json = True
                    elif method_info.name == "from_state":
                        class_info.has_from_state = True
                    elif method_info.name == "to_state":
                        class_info.has_to_state = True
            elif isinstance(item, ast.AnnAssign):
                # Class-level annotated assignments (properties)
                if isinstance(item.target, ast.Name):
                    prop_info = PropertyInfo(
                        name=item.target.id,
                        type_hint=self._get_annotation(item.annotation),
                    )
                    class_info.properties.append(prop_info)

        self.classes.append(class_info)
        self._current_class = None
        self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        """Visit a standalone function definition."""
        if self._current_class is None and not node.name.startswith("_"):
            func_info = self._extract_function(node)
            if func_info:
                self.functions.append(func_info)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        """Visit an async function definition."""
        if self._current_class is None and not node.name.startswith("_"):
            func_info = self._extract_function(node)
            if func_info:
                self.functions.append(func_info)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        """Visit an annotated assignment (potential constant)."""
        if (
            self._current_class is None
            and isinstance(node.target, ast.Name)
            and (node.target.id.isupper() or not node.target.id.startswith("_"))
        ):
            # Module-level constant
            const_info = ConstantInfo(
                name=node.target.id,
                module=self.module_name,
                type_hint=self._get_annotation(node.annotation),
            )
            self.constants.append(const_info)

    def visit_Assign(self, node: ast.Assign) -> None:
        """Visit an assignment to capture __all__."""
        for target in node.targets:
            if isinstance(target, ast.Name) and target.id == "__all__" and isinstance(node.value, ast.List):
                for elt in node.value.elts:
                    if isinstance(elt, ast.Constant) and isinstance(elt.value, str):
                        self.all_exports.append(elt.value)

    def _extract_method(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> MethodInfo | None:
        """Extract method information from a function definition."""
        if node.name.startswith("_") and node.name != "__init__":
            return None

        # Check for decorators
        is_static = False
        is_classmethod = False
        is_property = False

        for decorator in node.decorator_list:
            dec_name = self._get_name(decorator)
            if dec_name == "staticmethod":
                is_static = True
            elif dec_name == "classmethod":
                is_classmethod = True
            elif dec_name == "property":
                is_property = True

        # Extract parameters
        params = []
        for arg in node.args.args:
            if arg.arg not in ("self", "cls"):
                param_type = self._get_annotation(arg.annotation) if arg.annotation else ""
                params.append(f"{arg.arg}: {param_type}" if param_type else arg.arg)

        # Extract return type
        return_type = self._get_annotation(node.returns) if node.returns else ""

        return MethodInfo(
            name=node.name,
            is_static=is_static,
            is_classmethod=is_classmethod,
            is_property=is_property,
            return_type=return_type,
            parameters=params,
        )

    def _extract_function(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> FunctionInfo | None:
        """Extract function information."""
        if node.name.startswith("_"):
            return None

        # Extract parameters
        params = []
        for arg in node.args.args:
            param_type = self._get_annotation(arg.annotation) if arg.annotation else ""
            params.append(f"{arg.arg}: {param_type}" if param_type else arg.arg)

        # Extract return type
        return_type = self._get_annotation(node.returns) if node.returns else ""

        return FunctionInfo(
            name=node.name,
            module=self.module_name,
            return_type=return_type,
            parameters=params,
        )

    def _get_name(self, node: ast.expr | None) -> str:
        """Get name from a node."""
        if node is None:
            return ""
        if isinstance(node, ast.Name):
            return node.id
        if isinstance(node, ast.Attribute):
            return f"{self._get_name(node.value)}.{node.attr}"
        if isinstance(node, ast.Call):
            return self._get_name(node.func)
        return ""

    def _get_annotation(self, node: ast.expr | None) -> str:
        """Get type annotation as string."""
        if node is None:
            return ""

        result = ""
        if isinstance(node, ast.Name):
            result = node.id
        elif isinstance(node, ast.Constant):
            result = str(node.value)
        elif isinstance(node, ast.Attribute):
            result = f"{self._get_name(node.value)}.{node.attr}"
        elif isinstance(node, ast.Subscript):
            value = self._get_annotation(node.value)
            slice_val = self._get_annotation(node.slice)
            result = f"{value}[{slice_val}]"
        elif isinstance(node, ast.Tuple):
            result = ", ".join(self._get_annotation(e) for e in node.elts)
        elif isinstance(node, ast.BinOp) and isinstance(node.op, ast.BitOr):
            # Union type: X | Y
            result = f"{self._get_annotation(node.left)} | {self._get_annotation(node.right)}"
        else:
            result = ast.unparse(node) if hasattr(ast, "unparse") else ""

        return result


class ParityManifestGenerator:
    """Generate parity manifest from Python .pyi files."""

    def __init__(self, pyi_root: Path):
        """Initialize the generator with a root directory for .pyi files."""
        self.pyi_root = pyi_root
        self.modules: dict[str, ModuleInfo] = {}

    def extract_from_pyi_file(self, pyi_file: Path) -> ModuleInfo | None:
        """Extract API information from a .pyi file."""
        try:
            content = pyi_file.read_text(encoding="utf-8")
            tree = ast.parse(content)
        except (SyntaxError, UnicodeDecodeError) as e:
            print(f"Warning: Could not parse {pyi_file}: {e}", file=sys.stderr)
            return None

        # Calculate module name
        rel_path = pyi_file.relative_to(self.pyi_root)
        parts = list(rel_path.parts)
        if parts[-1] == "__init__.pyi":
            parts = parts[:-1]
        else:
            parts[-1] = parts[-1].replace(".pyi", "")

        module_name = "finstack." + ".".join(parts) if parts else "finstack"

        extractor = PyiExtractor(module_name)
        extractor.visit(tree)

        return ModuleInfo(
            name=module_name,
            path=str(rel_path),
            classes=extractor.classes,
            functions=extractor.functions,
            constants=extractor.constants,
            all_exports=extractor.all_exports,
        )

    def scan_directory(self) -> dict[str, Any]:
        """Scan the pyi directory and extract all modules."""
        all_classes: dict[str, ClassInfo] = {}
        all_functions: dict[str, FunctionInfo] = {}
        all_modules: dict[str, ModuleInfo] = {}

        for pyi_file in sorted(self.pyi_root.rglob("*.pyi")):
            # Skip test files and private modules
            if "test" in str(pyi_file).lower() or "/.pytest" in str(pyi_file):
                continue

            module_info = self.extract_from_pyi_file(pyi_file)
            if module_info:
                all_modules[module_info.name] = module_info

                # Track classes and functions globally
                for cls in module_info.classes:
                    all_classes[f"{module_info.name}.{cls.name}"] = cls
                for func in module_info.functions:
                    all_functions[f"{module_info.name}.{func.name}"] = func

        return {
            "modules": all_modules,
            "classes": all_classes,
            "functions": all_functions,
        }

    def generate_manifest(self) -> dict[str, Any]:
        """Generate the complete parity manifest."""
        data = self.scan_directory()

        # Build summary statistics
        total_classes = len(data["classes"])
        total_functions = len(data["functions"])
        total_modules = len(data["modules"])

        # Identify serialization patterns
        classes_with_from_json = []
        classes_with_to_json = []
        classes_with_from_state = []
        classes_with_to_state = []

        for fqn, cls in data["classes"].items():
            if cls.has_from_json:
                classes_with_from_json.append(fqn)
            if cls.has_to_json:
                classes_with_to_json.append(fqn)
            if cls.has_from_state:
                classes_with_from_state.append(fqn)
            if cls.has_to_state:
                classes_with_to_state.append(fqn)

        # Group by domain
        domains = {
            "core": [],
            "valuations": [],
            "statements": [],
            "scenarios": [],
            "portfolio": [],
        }

        for mod_name in data["modules"]:
            for domain, modules in domains.items():
                if f"finstack.{domain}" in mod_name:
                    modules.append(mod_name)
                    break

        manifest = {
            "version": "1.0.0",
            "source": "python-pyi",
            "root": str(self.pyi_root),
            "summary": {
                "total_modules": total_modules,
                "total_classes": total_classes,
                "total_functions": total_functions,
                "classes_with_from_json": len(classes_with_from_json),
                "classes_with_to_json": len(classes_with_to_json),
                "classes_with_from_state": len(classes_with_from_state),
                "classes_with_to_state": len(classes_with_to_state),
            },
            "domains": {domain: sorted(mods) for domain, mods in domains.items()},
            "serialization": {
                "from_json": sorted(classes_with_from_json),
                "to_json": sorted(classes_with_to_json),
                "from_state": sorted(classes_with_from_state),
                "to_state": sorted(classes_with_to_state),
            },
            "modules": {
                name: {
                    "path": mod.path,
                    "classes": [asdict(c) for c in mod.classes],
                    "functions": [asdict(f) for f in mod.functions],
                    "constants": [asdict(c) for c in mod.constants],
                    "all_exports": mod.all_exports,
                }
                for name, mod in sorted(data["modules"].items())
            },
        }

        return manifest


def generate_wasm_checklist(manifest: dict[str, Any]) -> dict[str, Any]:
    """Generate a WASM parity checklist from the manifest."""
    checklist = {
        "required_modules": [],
        "required_classes": [],
        "required_functions": [],
        "serialization_requirements": {
            "must_have_fromJson": [],
            "must_have_toJson": [],
        },
    }

    # All modules should be present in WASM
    for mod_name in manifest["modules"]:
        checklist["required_modules"].append(mod_name)

    # All classes should be present
    for mod_name, mod_data in manifest["modules"].items():
        for cls in mod_data["classes"]:
            checklist["required_classes"].append({
                "module": mod_name,
                "name": cls["name"],
                "js_name": snake_to_camel(cls["name"]),
                "has_from_json": cls["has_from_json"],
                "has_to_json": cls["has_to_json"],
            })

    # All standalone functions should be present
    for mod_name, mod_data in manifest["modules"].items():
        for func in mod_data["functions"]:
            checklist["required_functions"].append({
                "module": mod_name,
                "name": func["name"],
                "js_name": snake_to_camel(func["name"]),
            })

    # Serialization requirements
    checklist["serialization_requirements"]["must_have_fromJson"] = manifest["serialization"]["from_json"]
    checklist["serialization_requirements"]["must_have_toJson"] = manifest["serialization"]["to_json"]

    return checklist


def snake_to_camel(name: str) -> str:
    """Convert snake_case to camelCase."""
    if not name or "_" not in name:
        return name
    components = name.split("_")
    return components[0] + "".join(x.title() for x in components[1:])


def main() -> int:
    """Main entry point."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    pyi_root = project_root / "finstack-py" / "finstack"

    if not pyi_root.exists():
        print(f"Error: Python stub directory not found: {pyi_root}", file=sys.stderr)
        return 1

    print(f"Scanning Python stubs in: {pyi_root}")

    generator = ParityManifestGenerator(pyi_root)
    manifest = generator.generate_manifest()

    # Write to .audit/ (gitignored) — never write to tracked repo files
    audit_dir = project_root / ".audit"
    audit_dir.mkdir(exist_ok=True)

    manifest_file = audit_dir / "parity_manifest.json"
    with manifest_file.open("w") as f:
        json.dump(manifest, f, indent=2)
    print(f"Generated manifest: {manifest_file}")

    # Generate WASM checklist
    checklist = generate_wasm_checklist(manifest)
    checklist_file = audit_dir / "wasm_parity_checklist.json"
    with checklist_file.open("w") as f:
        json.dump(checklist, f, indent=2)
    print(f"Generated WASM checklist: {checklist_file}")

    # Print summary
    print("\n=== Parity Manifest Summary ===")
    print(f"Total modules: {manifest['summary']['total_modules']}")
    print(f"Total classes: {manifest['summary']['total_classes']}")
    print(f"Total functions: {manifest['summary']['total_functions']}")
    print(f"Classes with from_json: {manifest['summary']['classes_with_from_json']}")
    print(f"Classes with to_json: {manifest['summary']['classes_with_to_json']}")

    print("\nDomain breakdown:")
    for domain, mods in manifest["domains"].items():
        print(f"  {domain}: {len(mods)} modules")

    return 0


if __name__ == "__main__":
    sys.exit(main())
