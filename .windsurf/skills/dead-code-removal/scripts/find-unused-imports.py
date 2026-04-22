#!/usr/bin/env python3
"""Find unused imports in Python files.

Uses AST parsing to accurately detect unused imports.
"""

import ast
import json
from pathlib import Path
import sys


class ImportVisitor(ast.NodeVisitor):
    """AST visitor to collect imports and their usage."""

    def __init__(self) -> None:
        """Initialise empty import / usage tracking state."""
        self.imports: dict[str, int] = {}  # name -> line_number
        self.used_names: set[str] = set()
        self.import_star = False

    def visit_Import(self, node: ast.Import) -> None:
        """Record each top-level `import` alias."""
        for alias in node.names:
            name = alias.asname if alias.asname else alias.name.split(".")[0]
            self.imports[name] = node.lineno
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        """Record each `from X import Y` alias; track `import *` separately."""
        if node.module:
            if node.names[0].name == "*":
                self.import_star = True
            else:
                for alias in node.names:
                    name = alias.asname if alias.asname else alias.name
                    self.imports[name] = node.lineno
        self.generic_visit(node)

    def visit_Name(self, node: ast.Name) -> None:
        """Record a load-context name as used."""
        if isinstance(node.ctx, ast.Load):
            self.used_names.add(node.id)
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        """Record the root of attribute chains (e.g. `os` in `os.path`)."""
        if isinstance(node.value, ast.Name):
            self.used_names.add(node.value.id)
        self.generic_visit(node)


def find_unused_imports(file_path: Path) -> dict:
    """Find unused imports in a Python file."""
    try:
        content = file_path.read_text()
        tree = ast.parse(content, filename=str(file_path))

        visitor = ImportVisitor()
        visitor.visit(tree)

        unused = []
        for name, line_num in visitor.imports.items():
            if name not in visitor.used_names and not visitor.import_star:
                unused.append({
                    "name": name,
                    "line": line_num,
                })

        return {
            "file": str(file_path),
            "unused_imports": unused,
            "total_imports": len(visitor.imports),
            "has_import_star": visitor.import_star,
        }
    except SyntaxError as e:
        return {
            "file": str(file_path),
            "error": f"Syntax error: {e}",
            "unused_imports": [],
        }
    except (OSError, ValueError) as e:
        return {
            "file": str(file_path),
            "error": f"Error parsing file: {e}",
            "unused_imports": [],
        }


def main() -> None:
    """Entry point: scan each file in argv and print JSON results."""
    if len(sys.argv) < 2:
        print("Usage: find_unused_imports.py <python_file> [<python_file>...]")
        sys.exit(1)

    results = []
    for file_path_str in sys.argv[1:]:
        file_path = Path(file_path_str)
        if not file_path.exists():
            print(f"Warning: File not found: {file_path}", file=sys.stderr)
            continue

        result = find_unused_imports(file_path)
        results.append(result)

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
