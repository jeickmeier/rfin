"""
Title: Statements DSL Helpers
Persona: Financial Modeler
Complexity: Beginner
Runtime: ~1 second

Description:
Parse and compile statement DSL formulas without building a full model.

Key Concepts:
- DSL parsing to AST (StmtExpr)
- Compilation to core Expr
- Basic expression evaluation
"""

from __future__ import annotations


def main() -> None:
    from finstack.core.expr import CompiledExpr
    from finstack.statements import compile_formula, parse_and_compile, parse_formula

    # Parse-only (inspect AST)
    ast = parse_formula("(revenue - cogs) / revenue")
    print("AST:", ast)

    # Compile the AST to a core expression
    expr = compile_formula(ast)
    print("Expr:", expr)

    # Parse + compile in one step
    expr2 = parse_and_compile("revenue * 1.05")
    print("Expr2:", expr2)

    # Evaluate (toy data)
    compiled = CompiledExpr(expr2)
    result = compiled.eval(columns=["revenue"], data=[[100.0, 110.0, 120.0]])
    print("values:", result.values)


if __name__ == "__main__":
    main()
