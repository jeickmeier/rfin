"""Statements DSL helpers type stubs."""

from __future__ import annotations

from finstack.core.expr import Expr

class StmtExpr:
    """Parsed statements DSL AST."""

    def __repr__(self) -> str: ...

def parse_formula(formula: str) -> StmtExpr:
    """Parse a DSL formula into an AST."""
    ...

def compile_formula(ast: StmtExpr) -> Expr:
    """Compile a parsed AST into a core expression."""
    ...

def parse_and_compile(formula: str) -> Expr:
    """Parse and compile a DSL formula in one step."""
    ...

__all__ = [
    "StmtExpr",
    "parse_formula",
    "compile_formula",
    "parse_and_compile",
]
