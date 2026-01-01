"""Examples demonstrating scenario builder usage.

This script shows how to use the ScenarioBuilder fluent API to create scenarios
with a chainable, method-based approach.
"""

from finstack.scenarios.builder import ScenarioBuilder, scenario

from finstack.scenarios import CurveKind, ScenarioEngine, VolSurfaceKind


def example_basic_builder():
    """Basic builder example."""
    print("\n=== Basic Builder Example ===")

    # Build a simple scenario using fluent API
    spec = (
        ScenarioBuilder("basic_stress")
        .name("Basic Stress Test")
        .description("Simple rate and equity shock")
        .shift_discount_curve("USD.OIS", 50)  # +50bp
        .shift_equities(-10)  # -10%
        .roll_forward("1m")
        .build()
    )

    print(f"Scenario ID: {spec.id()}")
    print(f"Scenario Name: {spec.name()}")
    print(f"Operations: {len(spec.operations())}")
    print("\nGenerated JSON:")
    print(spec.to_json())


def example_comprehensive_builder():
    """Comprehensive builder example with all operation types."""
    print("\n=== Comprehensive Builder Example ===")

    spec = (
        ScenarioBuilder("comprehensive_stress")
        .name("Comprehensive Market Stress")
        .description("Multi-factor stress test")
        .priority(1)
        # Curve shocks
        .shift_discount_curve("USD.OIS", 50)
        .shift_forward_curve("EUR.SOFR", -25)
        .shift_hazard_curve("ACME.5Y", 100)
        .shift_inflation_curve("US.CPI", 10)
        # Equity shocks
        .shift_equities(-15)
        .shift_equities(5, ["SPY"])
        # FX shocks
        .shift_fx("USD", "EUR", 3)
        .shift_fx("GBP", "USD", -2)
        # Vol shocks
        .shift_vol_surface("SPX_VOL", 20)
        # Time operations
        .roll_forward("3m")
        # Statement operations
        .adjust_forecast("revenue", 10)
        .set_forecast("cogs", 500000)
        .build()
    )

    print(f"Scenario ID: {spec.id()}")
    print(f"Operations: {len(spec.operations())}")
    print(f"Priority: {spec.priority()}")


def example_rate_shock_builder():
    """Rate shock scenario using builder."""
    print("\n=== Rate Shock Builder Example ===")

    spec = (
        ScenarioBuilder("rate_shock_50bp")
        .name("50bp Global Rate Shock")
        .description("Parallel shift across all major curves")
        # Major discount curves
        .shift_discount_curve("USD.OIS", 50)
        .shift_discount_curve("EUR.OIS", 50)
        .shift_discount_curve("GBP.OIS", 50)
        .shift_discount_curve("JPY.OIS", 50)
        # Forward curves
        .shift_forward_curve("USD.SOFR", 50)
        .shift_forward_curve("EUR.SOFR", 50)
        .build()
    )

    print(f"Scenario: {spec.name()}")
    print(f"Operations: {len(spec.operations())}")


def example_equity_crash_builder():
    """Equity crash scenario using builder."""
    print("\n=== Equity Crash Builder Example ===")

    spec = (
        ScenarioBuilder("equity_crash")
        .name("Equity Market Crash")
        .description("-20% equity shock with vol spike and flight to quality")
        # Equity crash
        .shift_equities(-20)
        # Vol spike
        .shift_vol_surface("SPX_VOL", 50)
        .shift_vol_surface("VIX_VOL", 100)
        # Flight to quality (rates down)
        .shift_discount_curve("USD.OIS", -25)
        .shift_discount_curve("EUR.OIS", -20)
        .build()
    )

    print(f"Scenario: {spec.name()}")
    print(f"Operations: {len(spec.operations())}")
    print("\nJSON representation:")
    print(spec.to_json())


def example_horizon_builder():
    """Horizon scenarios using builder."""
    print("\n=== Horizon Scenarios Builder Example ===")

    scenarios = [
        ScenarioBuilder("horizon_1w").name("1-Week Horizon").roll_forward("1w").build(),
        ScenarioBuilder("horizon_1m").name("1-Month Horizon").roll_forward("1m").build(),
        ScenarioBuilder("horizon_3m").name("3-Month Horizon").roll_forward("3m").build(),
        ScenarioBuilder("horizon_1y").name("1-Year Horizon").roll_forward("1y").build(),
    ]

    print("Created horizon scenarios:")
    for sc in scenarios:
        print(f"  - {sc.name()}: {len(sc.operations())} operations")


def example_statement_shocks_builder():
    """Statement shock scenarios using builder."""
    print("\n=== Statement Shocks Builder Example ===")

    spec = (
        ScenarioBuilder("revenue_growth")
        .name("Revenue Growth Scenario")
        .description("15% revenue growth with operating leverage")
        # Income statement
        .adjust_forecast("revenue", 15)
        .adjust_forecast("cogs", 10)
        .set_forecast("opex", 2000000)
        # Balance sheet
        .set_forecast("receivables", 1500000)
        .set_forecast("inventory", 800000)
        .set_forecast("payables", 1200000)
        .build()
    )

    print(f"Scenario: {spec.name()}")
    print(f"Operations: {len(spec.operations())}")


def example_convenience_function():
    """Demonstrate scenario() convenience function."""
    print("\n=== Convenience Function Example ===")

    # Using scenario() instead of ScenarioBuilder()
    spec = scenario("stress").name("Stress Test").shift_discount_curve("USD.OIS", 50).shift_equities(-10).build()

    print(f"Scenario ID: {spec.id()}")
    print(f"Scenario Name: {spec.name()}")
    print(f"Operations: {len(spec.operations())}")


def example_scenario_composition():
    """Demonstrate scenario composition with builder."""
    print("\n=== Scenario Composition Example ===")

    # Create base scenario
    base = scenario("base").name("Base Scenario").priority(0).shift_discount_curve("USD.OIS", 25).build()

    # Create overlay scenario
    overlay = (
        scenario("overlay")
        .name("Overlay Scenario")
        .priority(1)
        .shift_equities(-10)
        .shift_vol_surface("SPX_VOL", 20)
        .build()
    )

    # Compose scenarios
    engine = ScenarioEngine()
    composed = engine.compose([base, overlay])

    print(f"Base operations: {len(base.operations())}")
    print(f"Overlay operations: {len(overlay.operations())}")
    print(f"Composed operations: {len(composed.operations())}")
    print(f"\nComposed scenario ID: {composed.id()}")


def example_builder_patterns():
    """Demonstrate common builder patterns."""
    print("\n=== Common Builder Patterns ===")

    # Pattern 1: Multi-curve shock
    print("\n1. Multi-curve shock pattern:")
    multi_curve = (
        scenario("multi_curve")
        .shift_discount_curve("USD.OIS", 50)
        .shift_discount_curve("EUR.OIS", 50)
        .shift_discount_curve("GBP.OIS", 50)
        .build()
    )
    print(f"   Operations: {len(multi_curve.operations())}")

    # Pattern 2: Rate + Equity shock
    print("\n2. Rate + Equity shock pattern:")
    rate_equity = scenario("rate_equity").shift_discount_curve("USD.OIS", 50).shift_equities(-10).build()
    print(f"   Operations: {len(rate_equity.operations())}")

    # Pattern 3: Horizon + market shock
    print("\n3. Horizon + market shock pattern:")
    horizon_shock = (
        scenario("horizon_shock").roll_forward("1m").shift_discount_curve("USD.OIS", 25).shift_equities(-5).build()
    )
    print(f"   Operations: {len(horizon_shock.operations())}")

    # Pattern 4: Credit shock
    print("\n4. Credit shock pattern:")
    credit = (
        scenario("credit_shock")
        .shift_hazard_curve("CORP.IG", 50)
        .shift_hazard_curve("CORP.HY", 200)
        .shift_equities(-15)
        .build()
    )
    print(f"   Operations: {len(credit.operations())}")


def example_builder_validation():
    """Demonstrate builder with metadata and validation."""
    print("\n=== Builder with Metadata ===")

    spec = (
        ScenarioBuilder("validated_scenario")
        .name("Q1 2024 Regulatory Stress Test")
        .description(
            """
            Regulatory stress test scenario for Q1 2024:
            - 50bp rate shock across major curves
            - -10% equity market shock
            - FX stress (USD strengthens 5% vs EUR)
            - Vol surface spike (+20%)
            - 3-month horizon with carry
            """
        )
        .priority(0)
        .shift_discount_curve("USD.OIS", 50)
        .shift_discount_curve("EUR.OIS", 50)
        .shift_equities(-10)
        .shift_fx("USD", "EUR", 5)
        .shift_vol_surface("SPX_VOL", 20)
        .roll_forward("3m")
        .build()
    )

    print(f"Scenario: {spec.name()}")
    print(f"Description: {spec.description()[:100]}...")
    print(f"Priority: {spec.priority()}")
    print(f"Operations: {len(spec.operations())}")

    # Serialize to verify
    json_str = spec.to_json()
    print(f"\nJSON length: {len(json_str)} characters")


def example_builder_vs_manual():
    """Compare builder vs manual construction."""
    print("\n=== Builder vs Manual Construction ===")

    # Builder approach
    print("\n1. Builder Approach (fluent, chainable):")
    builder_code = """
scenario("stress")
    .shift_discount_curve("USD.OIS", 50)
    .shift_equities(-10)
    .roll_forward("1m")
    .build()
    """
    print(builder_code)

    scenario_builder = (
        scenario("stress").shift_discount_curve("USD.OIS", 50).shift_equities(-10).roll_forward("1m").build()
    )

    # Manual approach
    print("\n2. Manual Approach (explicit):")
    manual_code = """
from finstack.scenarios import ScenarioSpec, OperationSpec, CurveKind

ScenarioSpec(
    "stress",
    [
        OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD.OIS", 50.0),
        OperationSpec.equity_price_pct([], -10.0),
        OperationSpec.time_roll_forward("1m"),
    ],
)
    """
    print(manual_code)

    from finstack.scenarios import OperationSpec, ScenarioSpec

    scenario_manual = ScenarioSpec(
        "stress",
        [
            OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD.OIS", 50.0),
            OperationSpec.equity_price_pct([], -10.0),
            OperationSpec.time_roll_forward("1m"),
        ],
    )

    print("\nBoth produce equivalent scenarios:")
    print(f"  Builder operations: {len(scenario_builder.operations())}")
    print(f"  Manual operations: {len(scenario_manual.operations())}")


def main():
    """Run all builder examples."""
    print("=" * 80)
    print("Scenario Builder Examples")
    print("=" * 80)

    example_basic_builder()
    example_comprehensive_builder()
    example_rate_shock_builder()
    example_equity_crash_builder()
    example_horizon_builder()
    example_statement_shocks_builder()
    example_convenience_function()
    example_scenario_composition()
    example_builder_patterns()
    example_builder_validation()
    example_builder_vs_manual()

    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80)


if __name__ == "__main__":
    main()
