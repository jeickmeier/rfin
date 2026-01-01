"""Tests for scenario DSL parser."""

from finstack.scenarios.dsl import DSLParseError, DSLParser, from_dsl
import pytest

from finstack.scenarios import (
    ScenarioSpec,
)


class TestDSLParser:
    """Test DSL parser."""

    def test_curve_shift_default(self) -> None:
        """Test default curve shift (discount curve)."""
        parser = DSLParser("shift USD.OIS +50bp")
        assert len(parser.operations) == 1
        # Note: We can't directly compare OperationSpec instances
        # Just verify it was parsed

    def test_curve_shift_explicit_kind(self) -> None:
        """Test curve shift with explicit kind."""
        parser = DSLParser("shift forward USD.SOFR -25bp")
        assert len(parser.operations) == 1

    def test_curve_shift_hazard(self) -> None:
        """Test hazard curve shift."""
        parser = DSLParser("shift hazard ACME.5Y +100bp")
        assert len(parser.operations) == 1

    def test_curve_shift_inflation(self) -> None:
        """Test inflation curve shift."""
        parser = DSLParser("shift inflation US.CPI +10bp")
        assert len(parser.operations) == 1

    def test_equity_shift_all(self) -> None:
        """Test all equities shift."""
        parser = DSLParser("shift equities -10%")
        assert len(parser.operations) == 1

    def test_equity_shift_single(self) -> None:
        """Test single equity shift."""
        parser = DSLParser("shift equity SPY +5%")
        assert len(parser.operations) == 1

    def test_fx_shift(self) -> None:
        """Test FX shift."""
        parser = DSLParser("shift fx USD/EUR +3%")
        assert len(parser.operations) == 1

    def test_vol_shift(self) -> None:
        """Test vol surface shift."""
        parser = DSLParser("shift vol SPX_VOL +15%")
        assert len(parser.operations) == 1

    def test_roll_forward_days(self) -> None:
        """Test roll forward in days."""
        parser = DSLParser("roll forward 1d")
        assert len(parser.operations) == 1

    def test_roll_forward_weeks(self) -> None:
        """Test roll forward in weeks."""
        parser = DSLParser("roll forward 2w")
        assert len(parser.operations) == 1

    def test_roll_forward_months(self) -> None:
        """Test roll forward in months."""
        parser = DSLParser("roll forward 3m")
        assert len(parser.operations) == 1

    def test_roll_forward_years(self) -> None:
        """Test roll forward in years."""
        parser = DSLParser("roll forward 1y")
        assert len(parser.operations) == 1

    def test_adjust_forecast(self) -> None:
        """Test statement forecast adjustment."""
        parser = DSLParser("adjust revenue +10%")
        assert len(parser.operations) == 1

    def test_set_forecast(self) -> None:
        """Test statement forecast assignment."""
        parser = DSLParser("set revenue 1000000")
        assert len(parser.operations) == 1

    def test_multiple_operations_newline(self) -> None:
        """Test multiple operations separated by newlines."""
        parser = DSLParser("""
            shift USD.OIS +50bp
            shift equities -10%
            roll forward 1m
        """)
        assert len(parser.operations) == 3

    def test_multiple_operations_semicolon(self) -> None:
        """Test multiple operations separated by semicolons."""
        parser = DSLParser("shift USD.OIS +50bp; shift equities -10%; roll forward 1m")
        assert len(parser.operations) == 3

    def test_comments(self) -> None:
        """Test comment handling."""
        parser = DSLParser("""
            # This is a comment
            shift USD.OIS +50bp  # Inline comment
            # Another comment
            shift equities -10%
        """)
        assert len(parser.operations) == 2

    def test_whitespace_tolerance(self) -> None:
        """Test tolerance to extra whitespace."""
        parser = DSLParser("   shift   USD.OIS   +50bp   ")
        assert len(parser.operations) == 1

    def test_case_insensitivity(self) -> None:
        """Test case insensitivity."""
        parser = DSLParser("SHIFT USD.OIS +50BP")
        assert len(parser.operations) == 1

    def test_negative_values(self) -> None:
        """Test negative values."""
        parser = DSLParser("shift USD.OIS -50bp")
        assert len(parser.operations) == 1

    def test_decimal_values(self) -> None:
        """Test decimal values."""
        parser = DSLParser("shift equities -12.5%")
        assert len(parser.operations) == 1

    def test_invalid_operation(self) -> None:
        """Test invalid operation raises error."""
        with pytest.raises(DSLParseError) as exc_info:
            DSLParser("invalid operation")
        assert "Unknown operation" in str(exc_info.value)

    def test_invalid_shift_syntax(self) -> None:
        """Test invalid shift syntax raises error."""
        with pytest.raises(DSLParseError) as exc_info:
            DSLParser("shift invalid syntax")
        assert "Invalid shift syntax" in str(exc_info.value)

    def test_invalid_roll_forward_syntax(self) -> None:
        """Test invalid roll forward syntax raises error."""
        with pytest.raises(DSLParseError) as exc_info:
            DSLParser("roll forward invalid")
        assert "Invalid roll forward syntax" in str(exc_info.value)

    def test_error_includes_line_number(self) -> None:
        """Test error includes line number."""
        with pytest.raises(DSLParseError) as exc_info:
            DSLParser("""
                shift USD.OIS +50bp
                invalid operation
                shift equities -10%
            """)
        assert "Line 2" in str(exc_info.value) or "Line 3" in str(exc_info.value)

    def test_empty_input(self) -> None:
        """Test empty input."""
        parser = DSLParser("")
        assert len(parser.operations) == 0

    def test_comments_only(self) -> None:
        """Test comments-only input."""
        parser = DSLParser("""
            # Just comments
            # Nothing else
        """)
        assert len(parser.operations) == 0


class TestFromDSL:
    """Test from_dsl function."""

    def test_from_dsl_basic(self) -> None:
        """Test from_dsl with basic input."""
        scenario = from_dsl("shift USD.OIS +50bp")
        assert scenario.id == "dsl_scenario"
        assert len(scenario.operations) == 1

    def test_from_dsl_custom_id(self) -> None:
        """Test from_dsl with custom ID."""
        scenario = from_dsl("shift USD.OIS +50bp", scenario_id="custom_id")
        assert scenario.id == "custom_id"

    def test_from_dsl_with_metadata(self) -> None:
        """Test from_dsl with name and description."""
        scenario = from_dsl(
            "shift USD.OIS +50bp",
            scenario_id="stress",
            name="Stress Test",
            description="Rate shock scenario",
        )
        assert scenario.id == "stress"
        assert scenario.name == "Stress Test"
        assert scenario.description == "Rate shock scenario"

    def test_from_dsl_with_priority(self) -> None:
        """Test from_dsl with priority."""
        scenario = from_dsl("shift USD.OIS +50bp", priority=10)
        assert scenario.priority == 10


class TestDSLIntegration:
    """Integration tests for DSL with scenario engine."""

    def test_dsl_scenario_serialization(self) -> None:
        """Test DSL-generated scenario can be serialized."""
        scenario = from_dsl("""
            shift USD.OIS +50bp
            shift equities -10%
        """)

        # Should be able to convert to JSON
        json_str = scenario.to_json()
        assert json_str is not None
        assert "USD.OIS" in json_str

        # Should be able to round-trip
        scenario2 = ScenarioSpec.from_json(json_str)
        assert scenario2.id == scenario.id
        assert len(scenario2.operations) == len(scenario.operations)

    def test_dsl_scenario_to_dict(self) -> None:
        """Test DSL-generated scenario can be converted to dict."""
        scenario = from_dsl("shift USD.OIS +50bp")
        data = scenario.to_dict()
        assert data is not None
        assert data["id"] == "dsl_scenario"
        assert len(data["operations"]) == 1


class TestDSLDocumentation:
    """Test DSL syntax documented in module docstring."""

    def test_example_in_module_docstring(self) -> None:
        """Test example from module docstring works."""
        # This is the example from the dsl.py module docstring
        scenario = from_dsl("""
            shift USD.OIS +50bp
            shift equities -10%
            roll forward 1m
        """)
        assert len(scenario.operations) == 3

    def test_all_documented_syntax_works(self) -> None:
        """Test all syntax examples from docstring."""
        examples = [
            "shift USD.OIS +50bp",
            "shift discount USD.OIS +50bp",
            "shift forward USD.SOFR -25bp",
            "shift hazard ACME.5Y +100bp",
            "shift inflation US.CPI +10bp",
            "shift equities -10%",
            "shift equity SPY +5%",
            "shift fx USD/EUR +3%",
            "shift vol SPX_VOL +15%",
            "roll forward 1d",
            "roll forward 1w",
            "roll forward 1m",
            "roll forward 3m",
            "roll forward 1y",
            "adjust revenue +10%",
            "set revenue 1000000",
        ]

        for example in examples:
            # Each should parse without error
            parser = DSLParser(example)
            assert len(parser.operations) == 1
