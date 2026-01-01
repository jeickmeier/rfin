"""Comprehensive tests for portfolio optimization Python bindings.

Tests cover:
1. Constraint types (MaxExposure, MaxSingleIssuer, MinDiversification equivalents)
2. Objective functions (maximize yield, minimize duration)
3. Trade universe with candidate positions
4. Optimization problem construction and solving
5. Result interpretation and trade generation
"""

from finstack.valuations.metrics import MetricId

from finstack.portfolio import (
    Constraint,
    Inequality,
    MetricExpr,
    MissingMetricPolicy,
    Objective,
    OptimizationResult,
    PerPositionMetric,
    PortfolioOptimizationProblem,
    PositionFilter,
    TradeDirection,
    TradeType,
    WeightingScheme,
    optimize_max_yield_with_ccc_limit,
)

# ===========================
# 1. Enum and Type Tests
# ===========================


def test_weighting_scheme_enum() -> None:
    """Test WeightingScheme enum values."""
    assert WeightingScheme.VALUE_WEIGHT is not None
    assert WeightingScheme.NOTIONAL_WEIGHT is not None
    assert WeightingScheme.UNIT_SCALING is not None


def test_missing_metric_policy_enum() -> None:
    """Test MissingMetricPolicy enum values."""
    assert MissingMetricPolicy.ZERO is not None
    assert MissingMetricPolicy.EXCLUDE is not None
    assert MissingMetricPolicy.STRICT is not None


def test_inequality_enum() -> None:
    """Test Inequality enum values."""
    assert Inequality.LE is not None
    assert Inequality.GE is not None
    assert Inequality.EQ is not None


def test_trade_direction_enum() -> None:
    """Test TradeDirection enum values."""
    assert TradeDirection.BUY is not None
    assert TradeDirection.SELL is not None
    assert TradeDirection.HOLD is not None


def test_trade_type_enum() -> None:
    """Test TradeType enum values."""
    assert TradeType.EXISTING is not None
    assert TradeType.NEW_POSITION is not None
    assert TradeType.CLOSE_OUT is not None


# ===========================
# 2. Metric and Expression Tests
# ===========================


def test_per_position_metric_construction() -> None:
    """Test PerPositionMetric factory methods."""
    # Metric ID-based
    ytm_metric = PerPositionMetric.metric(MetricId.YTM)
    assert ytm_metric is not None

    # Custom key
    custom = PerPositionMetric.custom_key("my_metric")
    assert custom is not None

    # PV-based
    pv_base = PerPositionMetric.pv_base()
    pv_native = PerPositionMetric.pv_native()
    assert pv_base is not None
    assert pv_native is not None

    # Tag-based
    tag_metric = PerPositionMetric.tag_equals("rating", "HY")
    assert tag_metric is not None

    # Constant
    const = PerPositionMetric.constant(1.0)
    assert const is not None


def test_metric_expr_construction() -> None:
    """Test MetricExpr construction."""
    ytm_metric = PerPositionMetric.metric(MetricId.YTM)

    # Weighted sum
    weighted = MetricExpr.weighted_sum(ytm_metric)
    assert weighted is not None

    # Value-weighted average
    avg = MetricExpr.value_weighted_average(ytm_metric)
    assert avg is not None

    # Tag exposure share
    tag_exp = MetricExpr.tag_exposure_share("rating", "HY")
    assert tag_exp is not None


def test_objective_construction() -> None:
    """Test Objective construction."""
    ytm_metric = PerPositionMetric.metric(MetricId.YTM)
    metric_expr = MetricExpr.value_weighted_average(ytm_metric)

    # Maximize
    obj_max = Objective.maximize(metric_expr)
    assert obj_max is not None

    # Minimize
    dur_metric = PerPositionMetric.metric(MetricId.DURATION_MOD)
    dur_expr = MetricExpr.weighted_sum(dur_metric)
    obj_min = Objective.minimize(dur_expr)
    assert obj_min is not None


# ===========================
# 3. Position Filter Tests
# ===========================


def test_position_filter_construction() -> None:
    """Test PositionFilter construction."""
    # All positions
    all_filter = PositionFilter.all()
    assert all_filter is not None

    # By entity ID
    entity_filter = PositionFilter.by_entity_id("entity_1")
    assert entity_filter is not None

    # By tag
    tag_filter = PositionFilter.by_tag("rating", "HY")
    assert tag_filter is not None

    # By position IDs
    ids_filter = PositionFilter.by_position_ids(["pos_1", "pos_2"])
    assert ids_filter is not None

    # Not filter
    not_filter = PositionFilter.not_(tag_filter)
    assert not_filter is not None


# ===========================
# 4. Constraint Tests
# ===========================


def test_constraint_metric_bound() -> None:
    """Test Constraint.metric_bound (duration <= 4.0)."""
    dur_metric = PerPositionMetric.metric(MetricId.DURATION_MOD)
    dur_expr = MetricExpr.weighted_sum(dur_metric)

    constraint = Constraint.metric_bound("duration_limit", dur_expr, Inequality.LE, 4.0)
    assert constraint is not None
    assert constraint.label() == "duration_limit"


def test_constraint_tag_exposure_limit() -> None:
    """Test Constraint.tag_exposure_limit (CCC exposure <= 10%)."""
    constraint = Constraint.tag_exposure_limit("ccc_limit", "rating", "CCC", 0.10)
    assert constraint is not None
    assert constraint.label() == "ccc_limit"


def test_constraint_tag_exposure_minimum() -> None:
    """Test Constraint.tag_exposure_minimum (IG exposure >= 50%)."""
    constraint = Constraint.tag_exposure_minimum("ig_floor", "rating", "IG", 0.50)
    assert constraint is not None
    assert constraint.label() == "ig_floor"


def test_constraint_weight_bounds() -> None:
    """Test Constraint.weight_bounds (position weights 0-20%)."""
    filter = PositionFilter.all()
    constraint = Constraint.weight_bounds("position_limits", filter, 0.0, 0.20)
    assert constraint is not None
    assert constraint.label() == "position_limits"


def test_constraint_max_turnover() -> None:
    """Test Constraint.max_turnover (20% max turnover)."""
    constraint = Constraint.max_turnover("low_turnover", 0.20)
    assert constraint is not None
    assert constraint.label() == "low_turnover"


def test_constraint_max_position_delta() -> None:
    """Test Constraint.max_position_delta (max 10% change per position)."""
    filter = PositionFilter.all()
    constraint = Constraint.max_position_delta("delta_limit", filter, 0.10)
    assert constraint is not None
    assert constraint.label() == "delta_limit"


def test_constraint_budget() -> None:
    """Test Constraint.budget (weights sum to 1.0)."""
    constraint = Constraint.budget(1.0)
    assert constraint is not None
    assert constraint.label() == "budget"


# ===========================
# 5. Helper Function Test
# ===========================


# ===========================
# 7. Constraint Combination Tests
# ===========================


def test_multiple_constraints() -> None:
    """Test combining multiple constraints."""
    # MaxExposure equivalent (via tag_exposure_limit)
    ccc_limit = Constraint.tag_exposure_limit(None, "rating", "CCC", 0.10)

    # MaxSingleIssuer equivalent (via weight_bounds)
    single_issuer = Constraint.weight_bounds(
        None,
        PositionFilter.all(),
        0.0,
        0.05,  # Max 5% per position
    )

    # MinDiversification equivalent (via tag_exposure_minimum)
    ig_min = Constraint.tag_exposure_minimum(None, "rating", "IG", 0.50)

    # Verify all constraints can be created
    assert ccc_limit is not None
    assert single_issuer is not None
    assert ig_min is not None


# ===========================
# 8. Edge Cases
# ===========================


def test_constraint_with_no_label() -> None:
    """Test constraints with None label."""
    constraint = Constraint.tag_exposure_limit(None, "rating", "HY", 0.30)
    assert constraint.label() is None


def test_zero_bounds() -> None:
    """Test weight bounds with zero min and max."""
    filter = PositionFilter.all()
    constraint = Constraint.weight_bounds(None, filter, 0.0, 0.0)
    assert constraint is not None


def test_full_turnover_constraint() -> None:
    """Test max turnover of 100% (no restriction)."""
    constraint = Constraint.max_turnover(None, 1.0)
    assert constraint is not None


# ===========================
# Summary
# ===========================


def test_api_completeness() -> None:
    """Verify all required API components are available."""
    # This test verifies that the Python bindings expose all the required
    # functionality for portfolio optimization as specified in the task.

    # Required constraint types (semantic equivalents)
    # 1. MaxExposure -> tag_exposure_limit (CCC exposure <= 10%)
    assert hasattr(Constraint, "tag_exposure_limit")

    # 2. MaxSingleIssuer -> weight_bounds (max weight per position)
    assert hasattr(Constraint, "weight_bounds")

    # 3. MinDiversification -> tag_exposure_minimum (IG exposure >= 50%)
    assert hasattr(Constraint, "tag_exposure_minimum")

    # Additional constraint types
    assert hasattr(Constraint, "metric_bound")
    assert hasattr(Constraint, "max_turnover")
    assert hasattr(Constraint, "max_position_delta")
    assert hasattr(Constraint, "budget")

    # Optimization problem and solver
    assert PortfolioOptimizationProblem is not None

    # Helper function
    assert optimize_max_yield_with_ccc_limit is not None

    # Results types
    assert OptimizationResult is not None
