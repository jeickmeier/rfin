"""Comprehensive parity tests for scenarios module.

Tests scenario spec, engine, DSL, and operation execution.
"""

from datetime import date

from finstack.core.market_data import DiscountCurve, MarketContext
import pytest

from finstack.scenarios import (
    CurveKind,
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
            scenario_id="stress_test",
            name="Q1 Stress Test",
            description="Rate shock scenario",
            operations=[],
        )

        assert spec.scenario_id == "stress_test"
        assert spec.name == "Q1 Stress Test"
        assert len(spec.operations) == 0

    def test_scenario_spec_with_operations(self) -> None:
        """Test scenario spec with operations."""
        ops = [
            OperationSpec.curve_parallel_bp(
                CurveKind.DISCOUNT,
                "USD-OIS",
                50.0,
            )
        ]

        spec = ScenarioSpec(
            scenario_id="rate_up",
            name="Rates Up 50bp",
            operations=ops,
        )

        assert len(spec.operations) == 1

    def test_scenario_spec_priority(self) -> None:
        """Test scenario spec with priority."""
        spec = ScenarioSpec(
            scenario_id="high_priority",
            name="High Priority Scenario",
            operations=[],
            priority=1,
        )

        assert spec.priority == 1


class TestOperationSpecParity:
    """Test operation spec construction matches Rust."""

    def test_curve_parallel_bp_operation(self) -> None:
        """Test parallel basis point curve shift."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.DISCOUNT,
            "USD-OIS",
            50.0,
        )

        assert op is not None

    def test_curve_node_bp_operation(self) -> None:
        """Test node-specific basis point curve shift."""
        op = OperationSpec.curve_node_bp(
            CurveKind.DISCOUNT,
            "USD-OIS",
            "5Y",
            25.0,
            TenorMatchMode.EXACT,
        )

        assert op is not None

    def test_equity_price_pct_operation(self) -> None:
        """Test equity price percentage shift."""
        op = OperationSpec.equity_price_pct("SPY", -10.0)

        assert op is not None

    def test_fx_pct_operation(self) -> None:
        """Test FX rate percentage shift."""
        op = OperationSpec.market_fx_pct("EUR", "USD", 5.0)

        assert op is not None

    def test_vol_surface_parallel_pct_operation(self) -> None:
        """Test parallel volatility surface shift."""
        from finstack.scenarios import VolSurfaceKind

        op = OperationSpec.vol_surface_parallel_pct(
            VolSurfaceKind.EQUITY,
            "SPX_VOL",
            10.0,
        )

        assert op is not None

    def test_time_roll_forward_operation(self) -> None:
        """Test time roll forward operation."""
        op = OperationSpec.time_roll_forward("1m", apply_shocks=True)

        assert op is not None


class TestScenarioEngineParity:
    """Test scenario engine execution matches Rust."""

    def test_engine_creation(self) -> None:
        """Test scenario engine creation."""
        engine = ScenarioEngine()
        assert engine is not None

    def test_apply_curve_parallel_shock(self) -> None:
        """Test applying parallel curve shock."""
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
            scenario_id="rate_up",
            name="Rates Up 50bp",
            operations=[
                OperationSpec.curve_parallel_bp(
                    CurveKind.DISCOUNT,
                    "USD-OIS",
                    50.0,  # +50bp
                )
            ],
        )

        # Apply scenario
        engine = ScenarioEngine()
        result = engine.apply(spec, market)

        # Should have report
        assert result is not None

    def test_apply_multiple_operations(self) -> None:
        """Test applying multiple operations."""
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
            scenario_id="multi_shock",
            name="Multiple Shocks",
            operations=[
                OperationSpec.curve_parallel_bp(
                    CurveKind.DISCOUNT,
                    "USD-OIS",
                    50.0,
                ),
                OperationSpec.equity_price_pct("SPY", -10.0),
            ],
        )

        engine = ScenarioEngine()
        result = engine.apply(spec, market)

        assert result is not None

    def test_scenario_composition(self) -> None:
        """Test composing multiple scenarios."""
        # Create base scenario
        base = ScenarioSpec(
            scenario_id="base",
            name="Base Scenario",
            operations=[
                OperationSpec.curve_parallel_bp(
                    CurveKind.DISCOUNT,
                    "USD-OIS",
                    25.0,
                )
            ],
            priority=10,
        )

        # Create overlay scenario
        overlay = ScenarioSpec(
            scenario_id="overlay",
            name="Overlay Scenario",
            operations=[
                OperationSpec.equity_price_pct("SPY", -5.0),
            ],
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
        assert CurveKind.DISCOUNT is not None
        assert CurveKind.FORWARD is not None
        assert CurveKind.HAZARD is not None
        assert CurveKind.INFLATION is not None


class TestTenorMatchModeParity:
    """Test tenor match mode enum matches Rust."""

    def test_tenor_match_mode_values(self) -> None:
        """Test tenor match mode enum values."""
        assert TenorMatchMode.EXACT is not None
        assert TenorMatchMode.INTERPOLATE is not None


class TestDSLParity:
    """Test DSL parser matches expected behavior."""

    def test_dsl_parse_curve_shift(self) -> None:
        """Test DSL parsing of curve shift."""
        dsl_text = """
        shift USD.OIS +50bp
        """

        spec = ScenarioSpec.from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 1

    def test_dsl_parse_multiple_operations(self) -> None:
        """Test DSL parsing multiple operations."""
        dsl_text = """
        shift USD.OIS +50bp
        shift equities -10%
        shift fx USD/EUR +3%
        """

        spec = ScenarioSpec.from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 3

    def test_dsl_parse_with_comments(self) -> None:
        """Test DSL parsing with comments."""
        dsl_text = """
        # Rate shock
        shift USD.OIS +50bp

        # Equity crash
        shift equities -10%
        """

        spec = ScenarioSpec.from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 2

    def test_dsl_parse_roll_forward(self) -> None:
        """Test DSL parsing time roll forward."""
        dsl_text = """
        roll forward 1m
        """

        spec = ScenarioSpec.from_dsl(dsl_text)

        assert spec is not None
        assert len(spec.operations) == 1


class TestScenarioBuilderParity:
    """Test scenario builder API matches expected behavior."""

    def test_builder_basic(self) -> None:
        """Test basic scenario builder."""
        from finstack.scenarios.builder import scenario

        spec = scenario("test").name("Test Scenario").shift_discount_curve("USD-OIS", 50).build()

        assert spec.scenario_id == "test"
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
        spec = ScenarioSpec(
            scenario_id="empty",
            name="Empty Scenario",
            operations=[],
        )

        market = MarketContext()
        engine = ScenarioEngine()
        result = engine.apply(spec, market)

        # Should succeed with no changes
        assert result is not None

    def test_zero_basis_point_shift(self) -> None:
        """Test zero basis point shift."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.DISCOUNT,
            "USD-OIS",
            0.0,  # Zero shift
        )

        spec = ScenarioSpec(
            scenario_id="zero_shift",
            name="Zero Shift",
            operations=[op],
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
        result = engine.apply(spec, market)

        # Should succeed, curve unchanged
        assert result is not None

    def test_large_shock_magnitude(self) -> None:
        """Test very large shock magnitude."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.DISCOUNT,
            "USD-OIS",
            1000.0,  # +1000bp (10%)
        )

        assert op is not None

    def test_negative_shock(self) -> None:
        """Test negative shock."""
        op = OperationSpec.curve_parallel_bp(
            CurveKind.DISCOUNT,
            "USD-OIS",
            -50.0,  # -50bp
        )

        assert op is not None

    def test_scenario_composition_same_priority(self) -> None:
        """Test composing scenarios with same priority."""
        s1 = ScenarioSpec(
            scenario_id="s1",
            operations=[OperationSpec.curve_parallel_bp(CurveKind.DISCOUNT, "USD-OIS", 25.0)],
            priority=5,
        )

        s2 = ScenarioSpec(
            scenario_id="s2",
            operations=[OperationSpec.equity_price_pct("SPY", -10.0)],
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
            scenario_id="test",
            name="Test Scenario",
            operations=[
                OperationSpec.curve_parallel_bp(
                    CurveKind.DISCOUNT,
                    "USD-OIS",
                    50.0,
                )
            ],
        )

        json_str = spec.to_json()
        assert json_str is not None
        assert "test" in json_str

    def test_scenario_spec_from_json(self) -> None:
        """Test scenario spec JSON deserialization."""
        json_str = '{"scenario_id":"test","name":"Test","operations":[]}'

        spec = ScenarioSpec.from_json(json_str)
        assert spec.scenario_id == "test"
        assert spec.name == "Test"

    def test_scenario_spec_roundtrip(self) -> None:
        """Test scenario spec JSON roundtrip."""
        original = ScenarioSpec(
            scenario_id="roundtrip",
            name="Roundtrip Test",
            operations=[
                OperationSpec.curve_parallel_bp(
                    CurveKind.DISCOUNT,
                    "USD-OIS",
                    50.0,
                )
            ],
            priority=5,
        )

        json_str = original.to_json()
        restored = ScenarioSpec.from_json(json_str)

        assert restored.scenario_id == original.scenario_id
        assert restored.name == original.name
        assert restored.priority == original.priority
        assert len(restored.operations) == len(original.operations)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
