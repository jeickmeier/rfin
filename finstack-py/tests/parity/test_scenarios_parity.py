"""Comprehensive parity tests for scenarios module.

Tests scenario spec, engine, DSL, and operation execution.
"""

from datetime import date

from finstack.core.currency import Currency
from finstack.core.market_data import DiscountCurve, MarketContext
import pytest

from finstack.scenarios import (
    CurveKind,
    ExecutionContext,
    OperationSpec,
    ScenarioEngine,
    ScenarioSpec,
    TenorMatchMode,
)


class TestScenarioSpecParity:
    """Test scenario spec construction matches Rust."""

    def test_scenario_spec_construction(self) -> None:
        """Test basic scenario spec construction."""
        spec = ScenarioSpec(
            "stress_test",
            [],
            name="Q1 Stress Test",
            description="Rate shock scenario",
        )

        assert spec.id == "stress_test"
        assert spec.name == "Q1 Stress Test"
        assert len(spec.operations) == 0

    def test_scenario_spec_with_operations(self) -> None:
        """Test scenario spec with operations."""
        ops = [
            OperationSpec.curve_parallel_bp(
                CurveKind.Discount,
                "USD-OIS",
                50.0,
            )
        ]

        spec = ScenarioSpec(
            "rate_up",
            ops,
            name="Rates Up 50bp",
        )

        assert len(spec.operations) == 1

    def test_scenario_spec_priority(self) -> None:
        """Test scenario spec with priority."""
        spec = ScenarioSpec(
            "high_priority",
            [],
            name="High Priority Scenario",
            priority=1,
        )

        assert spec.priority == 1


class TestOperationSpecParity:
    """Test operation spec construction matches Rust."""

    def test_curve_parallel_bp_operation(self) -> None:
        """Test parallel basis point curve shift."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.Discount,
            "USD-OIS",
            50.0,
        )

        assert op is not None

    def test_curve_node_bp_operation(self) -> None:
        """Test node-specific basis point curve shift."""
        op = OperationSpec.curve_node_bp(
            CurveKind.Discount,
            "USD-OIS",
            [("5Y", 25.0)],
            TenorMatchMode.Exact,
        )

        assert op is not None

    def test_equity_price_pct_operation(self) -> None:
        """Test equity price percentage shift."""
        op = OperationSpec.equity_price_pct(["SPY"], -10.0)

        assert op is not None

    def test_fx_pct_operation(self) -> None:
        """Test FX rate percentage shift."""
        op = OperationSpec.market_fx_pct(Currency("EUR"), Currency("USD"), 5.0)

        assert op is not None

    def test_vol_surface_parallel_pct_operation(self) -> None:
        """Test parallel volatility surface shift."""
        from finstack.scenarios import VolSurfaceKind

        op = OperationSpec.vol_surface_parallel_pct(
            VolSurfaceKind.Equity,
            "SPX_VOL",
            10.0,
        )

        assert op is not None

    def test_time_roll_forward_operation(self) -> None:
        """Test time roll forward operation."""
        op = OperationSpec.time_roll_forward("1m", True, None)

        assert op is not None


class TestScenarioEngineParity:
    """Test scenario engine execution matches Rust."""

    def test_engine_creation(self) -> None:
        """Test scenario engine creation."""
        engine = ScenarioEngine()
        assert engine is not None

    def test_apply_curve_parallel_shock(self) -> None:
        """Test applying parallel curve shock."""
        from finstack.statements.builder import ModelBuilder

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Create scenario
        spec = ScenarioSpec(
            "rate_up",
            [
                OperationSpec.curve_parallel_bp(
                    CurveKind.Discount,
                    "USD-OIS",
                    50.0,  # +50bp
                )
            ],
            name="Rates Up 50bp",
        )

        # Apply scenario
        engine = ScenarioEngine()
        builder = ModelBuilder.new("m")
        builder.periods("2024Q1..Q1", None)
        model = builder.build()
        ctx = ExecutionContext(market, model, date(2024, 1, 1))
        result = engine.apply(spec, ctx)

        # Should have report
        assert result is not None

    def test_apply_multiple_operations(self) -> None:
        """Test applying multiple operations."""
        from finstack.statements.builder import ModelBuilder

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Create scenario with multiple operations
        spec = ScenarioSpec(
            "multi_shock",
            [
                OperationSpec.curve_parallel_bp(
                    CurveKind.Discount,
                    "USD-OIS",
                    50.0,
                ),
                OperationSpec.equity_price_pct(["SPY"], -10.0),
            ],
            name="Multiple Shocks",
        )

        engine = ScenarioEngine()
        builder = ModelBuilder.new("m")
        builder.periods("2024Q1..Q1", None)
        model = builder.build()
        ctx = ExecutionContext(market, model, date(2024, 1, 1))
        result = engine.apply(spec, ctx)

        assert result is not None

    def test_scenario_composition(self) -> None:
        """Test composing multiple scenarios."""
        # Create base scenario
        base = ScenarioSpec(
            "base",
            [
                OperationSpec.curve_parallel_bp(
                    CurveKind.Discount,
                    "USD-OIS",
                    25.0,
                )
            ],
            name="Base Scenario",
            priority=10,
        )

        # Create overlay scenario
        overlay = ScenarioSpec(
            "overlay",
            [
                OperationSpec.equity_price_pct(["SPY"], -5.0),
            ],
            name="Overlay Scenario",
            priority=5,  # Higher priority (lower value)
        )

        # Compose scenarios
        engine = ScenarioEngine()
        composed = engine.compose([base, overlay])

        # Composed scenario should have operations from both
        assert len(composed.operations) == 2


class TestCurveKindParity:
    """Test curve kind enum matches Rust."""

    def test_curve_kind_values(self) -> None:
        """Test curve kind enum values."""
        assert CurveKind.Discount is not None
        assert CurveKind.Forecast is not None
        assert CurveKind.ParCDS is not None
        assert CurveKind.Inflation is not None

    def test_curve_kind_commodity_and_volindex(self) -> None:
        """Test CurveKind.Commodity and CurveKind.VolIndex are accessible."""
        assert CurveKind.Commodity is not None
        assert CurveKind.VolIndex is not None
        assert str(CurveKind.Commodity) == "Commodity"
        assert str(CurveKind.VolIndex) == "VolIndex"
        # Verify they are distinct from other variants
        assert CurveKind.Commodity != CurveKind.Discount
        assert CurveKind.VolIndex != CurveKind.Discount


class TestTenorMatchModeParity:
    """Test tenor match mode enum matches Rust."""

    def test_tenor_match_mode_values(self) -> None:
        """Test tenor match mode enum values."""
        assert TenorMatchMode.Exact is not None
        assert TenorMatchMode.Interpolate is not None


class TestDSLParity:
    """Test DSL parser matches expected behavior."""

    def test_dsl_parse_curve_shift(self) -> None:
        """Test DSL parsing of curve shift."""
        from finstack.scenarios.dsl import from_dsl

        dsl_text = """
        shift USD.OIS +50bp
        """

        spec = from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 1

    def test_dsl_parse_multiple_operations(self) -> None:
        """Test DSL parsing multiple operations."""
        from finstack.scenarios.dsl import from_dsl

        dsl_text = """
        shift USD.OIS +50bp
        shift equities -10%
        shift fx USD/EUR +3%
        """

        spec = from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 3

    def test_dsl_parse_with_comments(self) -> None:
        """Test DSL parsing with comments."""
        from finstack.scenarios.dsl import from_dsl

        dsl_text = """
        # Rate shock
        shift USD.OIS +50bp

        # Equity crash
        shift equities -10%
        """

        spec = from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 2

    def test_dsl_parse_roll_forward(self) -> None:
        """Test DSL parsing time roll forward."""
        from finstack.scenarios.dsl import from_dsl

        dsl_text = """
        roll forward 1m
        """

        spec = from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 1


class TestScenarioBuilderParity:
    """Test scenario builder API matches expected behavior."""

    def test_builder_basic(self) -> None:
        """Test basic scenario builder."""
        from finstack.scenarios.builder import scenario

        spec = scenario("test").name("Test Scenario").shift_discount_curve("USD-OIS", 50).build()

        assert spec.id == "test"
        assert spec.name == "Test Scenario"
        assert len(spec.operations) == 1

    def test_builder_multiple_operations(self) -> None:
        """Test builder with multiple operations."""
        from finstack.scenarios.builder import scenario

        spec = (
            scenario("multi").shift_discount_curve("USD-OIS", 50).shift_equities(-10).shift_fx("USD", "EUR", 3).build()
        )

        assert len(spec.operations) == 3

    def test_builder_with_priority(self) -> None:
        """Test builder with priority."""
        from finstack.scenarios.builder import scenario

        spec = scenario("priority_test").priority(5).shift_discount_curve("USD-OIS", 25).build()

        assert spec.priority == 5


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_empty_scenario(self) -> None:
        """Test scenario with no operations."""
        from finstack.statements.builder import ModelBuilder

        spec = ScenarioSpec(
            "empty",
            [],
            name="Empty Scenario",
        )

        market = MarketContext()
        engine = ScenarioEngine()
        builder = ModelBuilder.new("m")
        builder.periods("2024Q1..Q1", None)
        model = builder.build()
        ctx = ExecutionContext(market, model, date(2024, 1, 1))
        result = engine.apply(spec, ctx)

        # Should succeed with no changes
        assert result is not None

    def test_zero_basis_point_shift(self) -> None:
        """Test zero basis point shift."""
        from finstack.statements.builder import ModelBuilder

        op = OperationSpec.curve_parallel_bp(
            CurveKind.Discount,
            "USD-OIS",
            0.0,  # Zero shift
        )

        spec = ScenarioSpec(
            "zero_shift",
            [op],
            name="Zero Shift",
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        engine = ScenarioEngine()
        builder = ModelBuilder.new("m")
        builder.periods("2024Q1..Q1", None)
        model = builder.build()
        ctx = ExecutionContext(market, model, date(2024, 1, 1))
        result = engine.apply(spec, ctx)

        # Should succeed, curve unchanged
        assert result is not None

    def test_large_shock_magnitude(self) -> None:
        """Test very large shock magnitude."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.Discount,
            "USD-OIS",
            1000.0,  # +1000bp (10%)
        )

        assert op is not None

    def test_negative_shock(self) -> None:
        """Test negative shock."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.Discount,
            "USD-OIS",
            -50.0,  # -50bp
        )

        assert op is not None

    def test_scenario_composition_same_priority(self) -> None:
        """Test composing scenarios with same priority."""
        s1 = ScenarioSpec(
            "s1",
            [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 25.0)],
            priority=5,
        )

        s2 = ScenarioSpec(
            "s2",
            [OperationSpec.equity_price_pct(["SPY"], -10.0)],
            priority=5,
        )

        engine = ScenarioEngine()
        composed = engine.compose([s1, s2])

        # Should compose successfully
        assert len(composed.operations) == 2


class TestSerializationParity:
    """Test scenario serialization matches Rust."""

    def test_scenario_spec_to_json(self) -> None:
        """Test scenario spec JSON serialization."""
        spec = ScenarioSpec(
            "test",
            [
                OperationSpec.curve_parallel_bp(
                    CurveKind.Discount,
                    "USD-OIS",
                    50.0,
                )
            ],
            name="Test Scenario",
        )

        json_str = spec.to_json()
        assert json_str is not None
        assert "test" in json_str

    def test_scenario_spec_from_json(self) -> None:
        """Test scenario spec JSON deserialization."""
        json_str = '{"id":"test","name":"Test","operations":[]}'

        spec = ScenarioSpec.from_json(json_str)
        assert spec.id == "test"
        assert spec.name == "Test"

    def test_scenario_spec_roundtrip(self) -> None:
        """Test scenario spec JSON roundtrip."""
        original = ScenarioSpec(
            "roundtrip",
            [
                OperationSpec.curve_parallel_bp(
                    CurveKind.Discount,
                    "USD-OIS",
                    50.0,
                )
            ],
            name="Roundtrip Test",
            priority=5,
        )

        json_str = original.to_json()
        restored = ScenarioSpec.from_json(json_str)

        assert restored.id == original.id
        assert restored.name == original.name
        assert restored.priority == original.priority
        assert len(restored.operations) == len(original.operations)


class TestTimeRollModeParity:
    """Test TimeRollMode enum is exported and matches Rust."""

    def test_time_roll_mode_importable(self) -> None:
        """Test TimeRollMode is importable from finstack.scenarios."""
        from finstack.scenarios import TimeRollMode

        assert TimeRollMode.BusinessDays is not None
        assert TimeRollMode.CalendarDays is not None
        assert TimeRollMode.Approximate is not None

    def test_time_roll_mode_equality(self) -> None:
        """Test TimeRollMode equality and hashing."""
        from finstack.scenarios import TimeRollMode

        assert TimeRollMode.BusinessDays == TimeRollMode.BusinessDays
        assert TimeRollMode.BusinessDays != TimeRollMode.CalendarDays
        assert hash(TimeRollMode.BusinessDays) == hash(TimeRollMode.BusinessDays)

    def test_time_roll_mode_str(self) -> None:
        """Test TimeRollMode string representations."""
        from finstack.scenarios import TimeRollMode

        assert str(TimeRollMode.BusinessDays) == "BusinessDays"
        assert str(TimeRollMode.CalendarDays) == "CalendarDays"
        assert str(TimeRollMode.Approximate) == "Approximate"


class TestValidateParity:
    """Test validate() methods match Rust validation behavior."""

    def test_scenario_spec_validate_valid(self) -> None:
        """Test validate() passes for valid scenario."""
        spec = ScenarioSpec(
            "valid_test",
            [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0)],
        )
        # Should not raise
        spec.validate()

    def test_scenario_spec_validate_empty_id(self) -> None:
        """Test validate() raises on empty scenario ID."""
        spec = ScenarioSpec("", [])
        with pytest.raises(ValueError, match=r"[Ee]mpty"):
            spec.validate()

    def test_scenario_spec_validate_whitespace_id(self) -> None:
        """Test validate() raises on whitespace-only scenario ID."""
        spec = ScenarioSpec("   ", [])
        with pytest.raises(ValueError, match=r"[Ee]mpty"):
            spec.validate()

    def test_operation_spec_validate_valid(self) -> None:
        """Test validate() passes for valid operation."""
        op = OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0)
        # Should not raise
        op.validate()

    def test_operation_spec_validate_nan(self) -> None:
        """Test validate() raises on NaN value."""
        op = OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", float("nan"))
        with pytest.raises(ValueError, match="finite"):
            op.validate()

    def test_operation_spec_validate_empty_curve_id(self) -> None:
        """Test validate() raises on empty curve ID."""
        op = OperationSpec.curve_parallel_bp(CurveKind.Discount, "", 50.0)
        with pytest.raises(ValueError, match=r"[Ee]mpty"):
            op.validate()

    def test_operation_spec_validate_pct_floor(self) -> None:
        """Test validate() raises on percentage <= -100%."""
        op = OperationSpec.equity_price_pct(["SPY"], -100.0)
        with pytest.raises(ValueError, match="-100"):
            op.validate()

    def test_scenario_spec_validate_duplicate_time_roll(self) -> None:
        """Test validate() raises on multiple TimeRollForward operations."""
        spec = ScenarioSpec(
            "double_roll",
            [
                OperationSpec.time_roll_forward("1M", True, None),
                OperationSpec.time_roll_forward("3M", True, None),
            ],
        )
        with pytest.raises(ValueError, match="TimeRollForward"):
            spec.validate()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
