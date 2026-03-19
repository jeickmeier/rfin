"""Factor-model portfolio configuration, analysis, and what-if tools."""

from __future__ import annotations

from datetime import date
from typing import Any

from .portfolio import Portfolio
from .types import Position
from finstack.core.market_data.context import MarketContext

RiskMeasureLike = str | dict[str, dict[str, float]]

class MarketDependency:
    kind: str
    id: str | None
    dependency_type: str
    def to_json(self) -> str: ...

class BumpSizeConfig:
    rates_bp: float
    credit_bp: float
    equity_pct: float
    fx_pct: float
    vol_points: float
    overrides: list[tuple[str, float]]
    def __init__(
        self,
        rates_bp: float = 1.0,
        credit_bp: float = 1.0,
        equity_pct: float = 1.0,
        fx_pct: float = 1.0,
        vol_points: float = 1.0,
        overrides: list[tuple[str, float]] | None = None,
    ) -> None: ...

class MarketMapping:
    kind: str
    curve_ids: list[str]
    units: str | None
    tenor_weights: list[tuple[float, float]]

    @staticmethod
    def curve_parallel(curve_ids: list[str], units: str) -> MarketMapping: ...
    @staticmethod
    def curve_bucketed(curve_id: str, tenor_weights: list[tuple[float, float]]) -> MarketMapping: ...
    @staticmethod
    def equity_spot(tickers: list[str]) -> MarketMapping: ...
    @staticmethod
    def fx_rate(base: str, quote: str) -> MarketMapping: ...
    @staticmethod
    def vol_shift(surface_ids: list[str], units: str) -> MarketMapping: ...
    def to_json(self) -> str: ...

class FactorDefinition:
    id: str
    factor_type: str
    market_mapping: MarketMapping
    description: str | None
    def __init__(
        self,
        id: str,
        factor_type: str,
        market_mapping: MarketMapping,
        description: str | None = None,
    ) -> None: ...

class FactorCovarianceMatrix:
    factor_ids: list[str]
    def __init__(self, factor_ids: list[str], matrix: list[list[float]]) -> None: ...
    def matrix(self) -> list[list[float]]: ...
    def n_factors(self) -> int: ...
    def variance(self, factor_id: str) -> float: ...
    def covariance(self, lhs: str, rhs: str) -> float: ...
    def correlation(self, lhs: str, rhs: str) -> float: ...
    def to_json(self) -> str: ...

class AttributeFilter:
    tags: list[str]
    meta: list[tuple[str, str]]
    def __init__(
        self,
        tags: list[str] | None = None,
        meta: list[tuple[str, str]] | None = None,
    ) -> None: ...

class DependencyFilter:
    dependency_type: str | None
    curve_type: str | None
    id: str | None
    def __init__(
        self,
        dependency_type: str | None = None,
        curve_type: str | None = None,
        id: str | None = None,
    ) -> None: ...

class MappingRule:
    dependency_filter: DependencyFilter
    attribute_filter: AttributeFilter
    factor_id: str
    def __init__(
        self,
        dependency_filter: DependencyFilter,
        attribute_filter: AttributeFilter,
        factor_id: str,
    ) -> None: ...

class FactorNode:
    factor_id: str | None
    filter: AttributeFilter
    children: list[FactorNode]
    def __init__(
        self,
        factor_id: str | None = None,
        filter: AttributeFilter | None = None,
        children: list[FactorNode] | None = None,
    ) -> None: ...

class HierarchicalConfig:
    dependency_filter: DependencyFilter
    root: FactorNode
    def __init__(
        self,
        root: FactorNode,
        dependency_filter: DependencyFilter | None = None,
    ) -> None: ...

class MatchingConfig:
    kind: str
    @staticmethod
    def mapping_table(rules: list[MappingRule]) -> MatchingConfig: ...
    @staticmethod
    def cascade(configs: list[MatchingConfig]) -> MatchingConfig: ...
    @staticmethod
    def hierarchical(config: HierarchicalConfig) -> MatchingConfig: ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(json: str) -> MatchingConfig: ...

class FactorModelConfig:
    factors: list[FactorDefinition]
    covariance: FactorCovarianceMatrix
    matching: MatchingConfig
    pricing_mode: str
    risk_measure: Any
    bump_size: BumpSizeConfig | None
    unmatched_policy: str | None

    def __init__(
        self,
        factors: list[FactorDefinition],
        covariance: FactorCovarianceMatrix,
        matching: MatchingConfig,
        pricing_mode: str,
        risk_measure: RiskMeasureLike | None = None,
        bump_size: BumpSizeConfig | None = None,
        unmatched_policy: str | None = None,
    ) -> None: ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(json: str) -> FactorModelConfig: ...

class PositionAssignment:
    position_id: str
    mappings: list[tuple[MarketDependency, str]]

class UnmatchedEntry:
    position_id: str
    dependency: MarketDependency

class FactorAssignmentReport:
    assignments: list[PositionAssignment]
    unmatched: list[UnmatchedEntry]

class SensitivityMatrix:
    def n_positions(self) -> int: ...
    def n_factors(self) -> int: ...
    def position_ids(self) -> list[str]: ...
    def factor_ids(self) -> list[str]: ...
    def delta(self, position_idx: int, factor_idx: int) -> float: ...
    def position_deltas(self, position_idx: int) -> list[float]: ...
    def factor_deltas(self, factor_idx: int) -> list[float]: ...

class FactorContribution:
    factor_id: str
    absolute_risk: float
    relative_risk: float
    marginal_risk: float

class PositionFactorContribution:
    position_id: str
    factor_id: str
    risk_contribution: float

class RiskDecomposition:
    total_risk: float
    measure: str
    factor_contributions: list[FactorContribution]
    residual_risk: float
    position_factor_contributions: list[PositionFactorContribution]

class FactorContributionDelta:
    factor_id: str
    absolute_change: float
    relative_change: float

class WhatIfResult:
    before: RiskDecomposition
    after: RiskDecomposition
    delta: list[FactorContributionDelta]

class StressResult:
    total_pnl: float
    position_pnl: list[tuple[str, float]]
    stressed_decomposition: RiskDecomposition

class PositionChange:
    @staticmethod
    def add(position: Position) -> PositionChange: ...
    @staticmethod
    def remove(position_id: str) -> PositionChange: ...
    @staticmethod
    def resize(position_id: str, new_quantity: float) -> PositionChange: ...

class FactorConstraint:
    @staticmethod
    def max_factor_risk(factor_id: str, max_risk: float) -> FactorConstraint: ...
    @staticmethod
    def max_factor_concentration(factor_id: str, max_fraction: float) -> FactorConstraint: ...
    @staticmethod
    def factor_neutral(factor_id: str) -> FactorConstraint: ...

class FactorOptimizationResult:
    optimized_quantities: list[tuple[str, float]]

class FactorModelBuilder:
    def __init__(self) -> None: ...
    def config(self, config: FactorModelConfig) -> FactorModelBuilder: ...
    def build(self) -> FactorModel: ...

class FactorModel:
    def factors(self) -> list[FactorDefinition]: ...
    def assign_factors(self, portfolio: Portfolio) -> FactorAssignmentReport: ...
    def compute_sensitivities(
        self,
        portfolio: Portfolio,
        market: MarketContext,
        as_of: date,
    ) -> SensitivityMatrix: ...
    def analyze(
        self,
        portfolio: Portfolio,
        market: MarketContext,
        as_of: date,
    ) -> RiskDecomposition: ...
    def what_if(
        self,
        base: RiskDecomposition,
        sensitivities: SensitivityMatrix,
        portfolio: Portfolio,
        market: MarketContext,
        as_of: date,
    ) -> WhatIfEngine: ...

class WhatIfEngine:
    def position_what_if(self, changes: list[PositionChange]) -> WhatIfResult: ...
    def factor_stress(self, stresses: list[tuple[str, float]]) -> StressResult: ...
    def optimize(self, constraints: list[FactorConstraint]) -> FactorOptimizationResult: ...
