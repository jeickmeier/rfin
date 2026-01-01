"""Scenario analysis helpers.

This package provides Python-side utilities for scenario construction and parsing:

- `dsl`: Text-based DSL parser for scenarios
- `builder`: Fluent API builder for scenarios

These complement the Rust-based scenario execution engine.
"""

from finstack.scenarios.builder import ScenarioBuilder, scenario
from finstack.scenarios.dsl import DSLParseError, DSLParser, from_dsl

__all__ = [
    "DSLParseError",
    "DSLParser",
    "ScenarioBuilder",
    "from_dsl",
    "scenario",
]
