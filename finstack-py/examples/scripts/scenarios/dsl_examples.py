"""Examples demonstrating scenario DSL usage.

This script shows how to use the DSL parser to create scenarios from text,
providing a more concise alternative to manually constructing OperationSpec objects.
"""

from finstack.scenarios import ScenarioEngine, ScenarioSpec


def example_basic_dsl():
    """Basic DSL parsing example."""
    print("\n=== Basic DSL Example ===")

    # Parse a simple scenario from text
    scenario = ScenarioSpec.from_dsl(
        """
        shift USD.OIS +50bp
        shift equities -10%
        roll forward 1m
        """,
        scenario_id="basic_stress",
        name="Basic Stress Test",
    )

    print(f"Scenario ID: {scenario.id()}")
    print(f"Scenario Name: {scenario.name()}")
    print(f"Operations: {len(scenario.operations())}")
    print("\nGenerated JSON:")
    print(scenario.to_json())


def example_comprehensive_dsl():
    """Comprehensive DSL example with all operation types."""
    print("\n=== Comprehensive DSL Example ===")

    scenario = ScenarioSpec.from_dsl(
        """
        # Market Data Shocks
        shift discount USD.OIS +50bp
        shift forward EUR.SOFR -25bp
        shift hazard ACME.5Y +100bp
        shift inflation US.CPI +10bp

        # Equity Shocks
        shift equities -15%
        shift equity SPY +5%

        # FX Shocks
        shift fx USD/EUR +3%
        shift fx GBP/USD -2%

        # Vol Shocks
        shift vol SPX_VOL +20%

        # Time Operations
        roll forward 3m

        # Statement Operations
        adjust revenue +10%
        set cogs 500000
        """,
        scenario_id="comprehensive_stress",
        name="Comprehensive Market Stress",
        description="Multi-factor stress test with rate, equity, FX, and vol shocks",
        priority=1,
    )

    print(f"Scenario ID: {scenario.id()}")
    print(f"Operations: {len(scenario.operations())}")
    print(f"Priority: {scenario.priority()}")
    print("\nOperations:")
    for i, op in enumerate(scenario.operations(), 1):
        print(f"  {i}. {op}")


def example_rate_shock_dsl():
    """Rate shock scenario using DSL."""
    print("\n=== Rate Shock DSL Example ===")

    scenario = ScenarioSpec.from_dsl(
        """
        # 50bp parallel shift across all major curves
        shift discount USD.OIS +50bp
        shift discount EUR.OIS +50bp
        shift discount GBP.OIS +50bp
        shift discount JPY.OIS +50bp

        # Forward curves
        shift forward USD.SOFR +50bp
        shift forward EUR.SOFR +50bp
        """,
        scenario_id="rate_shock_50bp",
        name="50bp Global Rate Shock",
        description="Parallel shift of 50bp across all major discount and forward curves",
    )

    print(f"Scenario: {scenario.name()}")
    print(f"Operations: {len(scenario.operations())}")


def example_equity_crash_dsl():
    """Equity crash scenario using DSL."""
    print("\n=== Equity Crash DSL Example ===")

    scenario = ScenarioSpec.from_dsl(
        """
        # Equity market crash with vol spike
        shift equities -20%

        # Vol surfaces spike
        shift vol SPX_VOL +50%
        shift vol VIX_VOL +100%

        # Flight to quality: rates down
        shift discount USD.OIS -25bp
        shift discount EUR.OIS -20bp
        """,
        scenario_id="equity_crash",
        name="Equity Market Crash",
        description="-20% equity shock with vol spike and flight to quality",
    )

    print(f"Scenario: {scenario.name()}")
    print(f"Operations: {len(scenario.operations())}")
    print("\nJSON representation:")
    print(scenario.to_json())


def example_horizon_dsl():
    """Horizon scenario using DSL."""
    print("\n=== Horizon Scenarios DSL Example ===")

    # 1-week horizon
    scenario_1w = ScenarioSpec.from_dsl(
        "roll forward 1w",
        scenario_id="horizon_1w",
        name="1-Week Horizon",
    )

    # 1-month horizon
    scenario_1m = ScenarioSpec.from_dsl(
        "roll forward 1m",
        scenario_id="horizon_1m",
        name="1-Month Horizon",
    )

    # 3-month horizon
    scenario_3m = ScenarioSpec.from_dsl(
        "roll forward 3m",
        scenario_id="horizon_3m",
        name="3-Month Horizon",
    )

    # 1-year horizon
    scenario_1y = ScenarioSpec.from_dsl(
        "roll forward 1y",
        scenario_id="horizon_1y",
        name="1-Year Horizon",
    )

    print("Created horizon scenarios:")
    for sc in [scenario_1w, scenario_1m, scenario_3m, scenario_1y]:
        print(f"  - {sc.name()}: {len(sc.operations())} operations")


def example_statement_shocks_dsl():
    """Statement shock scenarios using DSL."""
    print("\n=== Statement Shocks DSL Example ===")

    scenario = ScenarioSpec.from_dsl(
        """
        # Revenue growth scenario
        adjust revenue +15%
        adjust cogs +10%
        set opex 2000000

        # Working capital assumptions
        set receivables 1500000
        set inventory 800000
        set payables 1200000
        """,
        scenario_id="revenue_growth",
        name="Revenue Growth Scenario",
        description="15% revenue growth with operating leverage",
    )

    print(f"Scenario: {scenario.name()}")
    print(f"Operations: {len(scenario.operations())}")


def example_scenario_composition():
    """Demonstrate scenario composition."""
    print("\n=== Scenario Composition Example ===")

    # Create base scenarios using DSL
    base_scenario = ScenarioSpec.from_dsl(
        "shift discount USD.OIS +25bp",
        scenario_id="base",
        priority=0,
    )

    overlay_scenario = ScenarioSpec.from_dsl(
        """
        shift equities -10%
        shift vol SPX_VOL +20%
        """,
        scenario_id="overlay",
        priority=1,
    )

    # Compose scenarios
    engine = ScenarioEngine()
    composed = engine.compose([base_scenario, overlay_scenario])

    print(f"Base operations: {len(base_scenario.operations())}")
    print(f"Overlay operations: {len(overlay_scenario.operations())}")
    print(f"Composed operations: {len(composed.operations())}")
    print(f"\nComposed scenario ID: {composed.id()}")


def example_dsl_error_handling():
    """Demonstrate DSL error handling."""
    print("\n=== DSL Error Handling Example ===")

    from finstack.scenarios.dsl import DSLParseError

    # Valid scenario
    try:
        ScenarioSpec.from_dsl("shift USD.OIS +50bp")
        print("✓ Valid scenario parsed successfully")
    except DSLParseError as e:
        print(f"✗ Unexpected error: {e}")

    # Invalid syntax
    try:
        ScenarioSpec.from_dsl("invalid syntax here")
        print("✗ Should have raised error")
    except DSLParseError as e:
        print(f"✓ Caught expected error: {e}")

    # Multiple operations with one invalid
    try:
        ScenarioSpec.from_dsl(
            """
            shift USD.OIS +50bp
            this is invalid
            shift equities -10%
            """
        )
        print("✗ Should have raised error")
    except DSLParseError as e:
        print(f"✓ Caught error with line info: {e}")


def example_dsl_vs_manual():
    """Compare DSL vs manual construction."""
    print("\n=== DSL vs Manual Construction ===")

    # DSL approach
    print("\n1. DSL Approach (concise):")
    dsl_code = '''
ScenarioSpec.from_dsl("""
    shift USD.OIS +50bp
    shift equities -10%
    roll forward 1m
""", scenario_id="stress")
    '''
    print(dsl_code)

    scenario_dsl = ScenarioSpec.from_dsl(
        """
        shift USD.OIS +50bp
        shift equities -10%
        roll forward 1m
        """,
        scenario_id="stress",
    )

    # Manual approach
    print("\n2. Manual Approach (verbose):")
    manual_code = """
from finstack.scenarios import ScenarioSpec, OperationSpec, CurveKind

scenario = ScenarioSpec(
    "stress",
    [
        OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD.OIS", 50.0),
        OperationSpec.equity_price_pct([], -10.0),
        OperationSpec.time_roll_forward("1m"),
    ],
)
    """
    print(manual_code)

    from finstack.scenarios import CurveKind, OperationSpec

    scenario_manual = ScenarioSpec(
        "stress",
        [
            OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD.OIS", 50.0),
            OperationSpec.equity_price_pct([], -10.0),
            OperationSpec.time_roll_forward("1m"),
        ],
    )

    print("\nBoth produce equivalent scenarios:")
    print(f"  DSL operations: {len(scenario_dsl.operations())}")
    print(f"  Manual operations: {len(scenario_manual.operations())}")


def main():
    """Run all DSL examples."""
    print("=" * 80)
    print("Scenario DSL Examples")
    print("=" * 80)

    example_basic_dsl()
    example_comprehensive_dsl()
    example_rate_shock_dsl()
    example_equity_crash_dsl()
    example_horizon_dsl()
    example_statement_shocks_dsl()
    example_scenario_composition()
    example_dsl_error_handling()
    example_dsl_vs_manual()

    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80)


if __name__ == "__main__":
    main()
