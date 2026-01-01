"""Tests for scenario builder."""

import pytest

from finstack import Currency
from finstack.scenarios import CurveKind, ScenarioSpec, VolSurfaceKind
from finstack.scenarios.builder import ScenarioBuilder, scenario


class TestScenarioBuilder:
    """Test ScenarioBuilder."""

    def test_builder_basic(self):
        """Test basic builder construction."""
        builder = ScenarioBuilder("test_scenario")
        spec = builder.build()

        assert spec.id() == "test_scenario"
        assert len(spec.operations()) == 0

    def test_builder_with_name(self):
        """Test builder with name."""
        spec = ScenarioBuilder("test").name("Test Scenario").build()
        assert spec.name() == "Test Scenario"

    def test_builder_with_description(self):
        """Test builder with description."""
        spec = ScenarioBuilder("test").description("Test description").build()
        assert spec.description() == "Test description"

    def test_builder_with_priority(self):
        """Test builder with priority."""
        spec = ScenarioBuilder("test").priority(10).build()
        assert spec.priority() == 10

    def test_builder_chaining(self):
        """Test method chaining."""
        spec = (
            ScenarioBuilder("test")
            .name("Test")
            .description("Desc")
            .priority(5)
            .build()
        )
        assert spec.id() == "test"
        assert spec.name() == "Test"
        assert spec.description() == "Desc"
        assert spec.priority() == 5


class TestBuilderCurveOperations:
    """Test curve-related builder methods."""

    def test_shift_curve_default(self):
        """Test shift_curve with default curve kind."""
        spec = ScenarioBuilder("test").shift_curve("USD.OIS", 50).build()
        assert len(spec.operations()) == 1

    def test_shift_curve_explicit_kind(self):
        """Test shift_curve with explicit curve kind."""
        spec = (
            ScenarioBuilder("test")
            .shift_curve("USD.SOFR", -25, CurveKind.Forward)
            .build()
        )
        assert len(spec.operations()) == 1

    def test_shift_discount_curve(self):
        """Test shift_discount_curve."""
        spec = ScenarioBuilder("test").shift_discount_curve("USD.OIS", 50).build()
        assert len(spec.operations()) == 1

    def test_shift_forward_curve(self):
        """Test shift_forward_curve."""
        spec = ScenarioBuilder("test").shift_forward_curve("USD.SOFR", -25).build()
        assert len(spec.operations()) == 1

    def test_shift_hazard_curve(self):
        """Test shift_hazard_curve."""
        spec = ScenarioBuilder("test").shift_hazard_curve("ACME.5Y", 100).build()
        assert len(spec.operations()) == 1

    def test_shift_inflation_curve(self):
        """Test shift_inflation_curve."""
        spec = ScenarioBuilder("test").shift_inflation_curve("US.CPI", 10).build()
        assert len(spec.operations()) == 1

    def test_multiple_curve_shifts(self):
        """Test multiple curve shifts."""
        spec = (
            ScenarioBuilder("test")
            .shift_discount_curve("USD.OIS", 50)
            .shift_forward_curve("EUR.SOFR", -25)
            .shift_hazard_curve("ACME.5Y", 100)
            .build()
        )
        assert len(spec.operations()) == 3


class TestBuilderEquityOperations:
    """Test equity-related builder methods."""

    def test_shift_equities_all(self):
        """Test shift all equities."""
        spec = ScenarioBuilder("test").shift_equities(-10).build()
        assert len(spec.operations()) == 1

    def test_shift_equities_specific(self):
        """Test shift specific equities."""
        spec = ScenarioBuilder("test").shift_equities(5, ["SPY", "QQQ"]).build()
        assert len(spec.operations()) == 1

    def test_multiple_equity_shifts(self):
        """Test multiple equity shifts."""
        spec = (
            ScenarioBuilder("test")
            .shift_equities(-10)
            .shift_equities(5, ["SPY"])
            .build()
        )
        assert len(spec.operations()) == 2


class TestBuilderFXOperations:
    """Test FX-related builder methods."""

    def test_shift_fx(self):
        """Test FX shift."""
        spec = ScenarioBuilder("test").shift_fx("USD", "EUR", 5).build()
        assert len(spec.operations()) == 1

    def test_multiple_fx_shifts(self):
        """Test multiple FX shifts."""
        spec = (
            ScenarioBuilder("test")
            .shift_fx("USD", "EUR", 5)
            .shift_fx("GBP", "USD", -3)
            .build()
        )
        assert len(spec.operations()) == 2


class TestBuilderVolOperations:
    """Test volatility-related builder methods."""

    def test_shift_vol_surface_default(self):
        """Test vol surface shift with default kind."""
        spec = ScenarioBuilder("test").shift_vol_surface("SPX_VOL", 10).build()
        assert len(spec.operations()) == 1

    def test_shift_vol_surface_explicit_kind(self):
        """Test vol surface shift with explicit kind."""
        spec = (
            ScenarioBuilder("test")
            .shift_vol_surface("SPX_VOL", 10, VolSurfaceKind.Equity)
            .build()
        )
        assert len(spec.operations()) == 1

    def test_multiple_vol_shifts(self):
        """Test multiple vol shifts."""
        spec = (
            ScenarioBuilder("test")
            .shift_vol_surface("SPX_VOL", 10)
            .shift_vol_surface("VIX_VOL", -5)
            .build()
        )
        assert len(spec.operations()) == 2


class TestBuilderTimeOperations:
    """Test time-related builder methods."""

    def test_roll_forward_days(self):
        """Test roll forward in days."""
        spec = ScenarioBuilder("test").roll_forward("1d").build()
        assert len(spec.operations()) == 1

    def test_roll_forward_weeks(self):
        """Test roll forward in weeks."""
        spec = ScenarioBuilder("test").roll_forward("2w").build()
        assert len(spec.operations()) == 1

    def test_roll_forward_months(self):
        """Test roll forward in months."""
        spec = ScenarioBuilder("test").roll_forward("3m").build()
        assert len(spec.operations()) == 1

    def test_roll_forward_years(self):
        """Test roll forward in years."""
        spec = ScenarioBuilder("test").roll_forward("1y").build()
        assert len(spec.operations()) == 1


class TestBuilderStatementOperations:
    """Test statement-related builder methods."""

    def test_adjust_forecast(self):
        """Test forecast adjustment."""
        spec = ScenarioBuilder("test").adjust_forecast("revenue", 10).build()
        assert len(spec.operations()) == 1

    def test_adjust_forecast_with_period(self):
        """Test forecast adjustment for specific period."""
        spec = ScenarioBuilder("test").adjust_forecast("revenue", 10, "2024Q1").build()
        assert len(spec.operations()) == 1

    def test_set_forecast(self):
        """Test forecast assignment."""
        spec = ScenarioBuilder("test").set_forecast("revenue", 1000000).build()
        assert len(spec.operations()) == 1

    def test_set_forecast_with_period(self):
        """Test forecast assignment for specific period."""
        spec = (
            ScenarioBuilder("test")
            .set_forecast("revenue", 1000000, "2024Q1")
            .build()
        )
        assert len(spec.operations()) == 1

    def test_multiple_statement_operations(self):
        """Test multiple statement operations."""
        spec = (
            ScenarioBuilder("test")
            .adjust_forecast("revenue", 10)
            .set_forecast("cogs", 500000)
            .build()
        )
        assert len(spec.operations()) == 2


class TestBuilderComplexScenarios:
    """Test complex multi-operation scenarios."""

    def test_comprehensive_scenario(self):
        """Test comprehensive scenario with all operation types."""
        spec = (
            ScenarioBuilder("comprehensive")
            .name("Comprehensive Stress Test")
            .description("Tests all operation types")
            .priority(1)
            .shift_discount_curve("USD.OIS", 50)
            .shift_forward_curve("EUR.SOFR", -25)
            .shift_hazard_curve("ACME.5Y", 100)
            .shift_equities(-10)
            .shift_fx("USD", "EUR", 5)
            .shift_vol_surface("SPX_VOL", 10)
            .roll_forward("1m")
            .adjust_forecast("revenue", 10)
            .set_forecast("cogs", 500000)
            .build()
        )

        assert spec.id() == "comprehensive"
        assert spec.name() == "Comprehensive Stress Test"
        assert spec.priority() == 1
        assert len(spec.operations()) == 9

    def test_rate_shock_scenario(self):
        """Test rate shock scenario."""
        spec = (
            ScenarioBuilder("rate_shock")
            .name("50bp Rate Shock")
            .description("Parallel shift across major curves")
            .shift_discount_curve("USD.OIS", 50)
            .shift_discount_curve("EUR.OIS", 50)
            .shift_discount_curve("GBP.OIS", 50)
            .shift_forward_curve("USD.SOFR", 50)
            .shift_forward_curve("EUR.SOFR", 50)
            .build()
        )

        assert len(spec.operations()) == 5

    def test_equity_drawdown_scenario(self):
        """Test equity drawdown scenario."""
        spec = (
            ScenarioBuilder("equity_crash")
            .name("Equity Market Crash")
            .description("-20% equity shock with vol spike")
            .shift_equities(-20)
            .shift_vol_surface("SPX_VOL", 50)
            .shift_vol_surface("VIX_VOL", 100)
            .build()
        )

        assert len(spec.operations()) == 3

    def test_horizon_scenario(self):
        """Test horizon scenario with time decay."""
        spec = (
            ScenarioBuilder("horizon_1m")
            .name("1-Month Horizon")
            .description("Roll forward 1 month with carry")
            .roll_forward("1m")
            .build()
        )

        assert len(spec.operations()) == 1


class TestScenarioConvenienceFunction:
    """Test scenario() convenience function."""

    def test_scenario_function(self):
        """Test scenario() creates builder."""
        spec = scenario("test").name("Test").build()
        assert spec.id() == "test"
        assert spec.name() == "Test"

    def test_scenario_function_chaining(self):
        """Test scenario() with full chain."""
        spec = (
            scenario("stress")
            .shift_discount_curve("USD.OIS", 50)
            .shift_equities(-10)
            .build()
        )
        assert spec.id() == "stress"
        assert len(spec.operations()) == 2


class TestBuilderIntegration:
    """Integration tests for builder with scenario engine."""

    def test_builder_scenario_serialization(self):
        """Test builder-generated scenario can be serialized."""
        spec = (
            ScenarioBuilder("test")
            .shift_discount_curve("USD.OIS", 50)
            .shift_equities(-10)
            .build()
        )

        # Should be able to convert to JSON
        json_str = spec.to_json()
        assert json_str is not None
        assert "USD.OIS" in json_str

        # Should be able to round-trip
        spec2 = ScenarioSpec.from_json(json_str)
        assert spec2.id() == spec.id()
        assert len(spec2.operations()) == len(spec.operations())

    def test_builder_scenario_to_dict(self):
        """Test builder-generated scenario can be converted to dict."""
        spec = ScenarioBuilder("test").shift_discount_curve("USD.OIS", 50).build()
        data = spec.to_dict()
        assert data is not None
        assert data["id"] == "test"
        assert len(data["operations"]) == 1


class TestBuilderDocumentation:
    """Test builder examples from docstrings."""

    def test_example_in_module_docstring(self):
        """Test example from module docstring works."""
        # This is the example from builder.py module docstring
        scenario_spec = (
            ScenarioBuilder("stress_test")
            .name("Q1 2024 Stress Test")
            .description("Rate shock + equity drawdown")
            .shift_curve("USD.OIS", 50)
            .shift_equities(-10)
            .roll_forward("1m")
            .build()
        )
        assert scenario_spec.id() == "stress_test"
        assert len(scenario_spec.operations()) == 3

    def test_example_in_class_docstring(self):
        """Test example from class docstring works."""
        # This is the example from ScenarioBuilder class docstring
        builder = ScenarioBuilder("stress_test")
        scenario_spec = (
            builder
            .name("Q1 Stress")
            .shift_curve("USD.OIS", 50)
            .shift_equities(-10)
            .build()
        )
        assert scenario_spec.id() == "stress_test"
        assert len(scenario_spec.operations()) == 2
