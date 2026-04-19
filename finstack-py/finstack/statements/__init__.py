"""Financial statement modeling: builders, evaluators, forecasts, DSL, adjustments.

Bindings for the ``finstack-statements`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import statements as _statements

ForecastMethod = _statements.ForecastMethod
NodeType = _statements.NodeType
NodeId = _statements.NodeId
NumericMode = _statements.NumericMode
FinancialModelSpec = _statements.FinancialModelSpec
ModelBuilder = _statements.ModelBuilder
StatementResult = _statements.StatementResult
Evaluator = _statements.Evaluator
parse_formula = _statements.parse_formula
validate_formula = _statements.validate_formula
NormalizationConfig = _statements.NormalizationConfig
normalize = _statements.normalize
CheckSuiteSpec = _statements.CheckSuiteSpec
CheckReport = _statements.CheckReport

__all__: list[str] = [
    "CheckReport",
    "CheckSuiteSpec",
    "Evaluator",
    "FinancialModelSpec",
    "ForecastMethod",
    "ModelBuilder",
    "NodeId",
    "NodeType",
    "NormalizationConfig",
    "NumericMode",
    "StatementResult",
    "normalize",
    "parse_formula",
    "validate_formula",
]
