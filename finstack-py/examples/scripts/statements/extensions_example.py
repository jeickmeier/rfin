"""Statement Extensions Framework Example.
=======================================

This example demonstrates how to use the statement extensions framework
for balance sheet validation (Corkscrew) and credit rating assignment (Scorecard).

Extensions provide additional analysis and validation capabilities beyond
the core statement evaluation engine.
"""

import sys

from finstack.statements.extensions import (
    AccountType,
    CorkscrewAccount,
    CorkscrewConfig,
    CorkscrewExtension,
    CreditScorecardExtension,
    ExtensionRegistry,
    ScorecardConfig,
    ScorecardMetric,
)

from finstack.statements import (
    Evaluator,
    ForecastMethod,
    ForwardFill,
    ModelBuilder,
    NodeType,
    PeriodId,
)


def create_balance_sheet_model():
    """Create a simple balance sheet model for demonstration."""
    builder = ModelBuilder()

    # Define periods (3 quarters)
    builder.periods(
        start=PeriodId.from_str("2024Q1"),
        end=PeriodId.from_str("2024Q3"),
        frequency="quarterly",
    )

    # Assets
    builder.node("cash", NodeType.Value(), description="Cash and cash equivalents").value("2024Q1", 100_000.0)

    builder.node("cash_inflows", NodeType.Value(), description="Cash inflows from operations").value(
        "2024Q1", 50_000.0
    ).value("2024Q2", 55_000.0).value("2024Q3", 60_000.0)

    builder.node("cash_outflows", NodeType.Value(), description="Cash outflows for operations").value(
        "2024Q1", 40_000.0
    ).value("2024Q2", 45_000.0).value("2024Q3", 50_000.0)

    # Liabilities
    builder.node("debt", NodeType.Value(), description="Total debt outstanding").value("2024Q1", 200_000.0)

    builder.node("debt_issuance", NodeType.Value(), description="New debt issued").value("2024Q1", 50_000.0).value(
        "2024Q2", 30_000.0
    ).value("2024Q3", 0.0)

    builder.node("debt_repayment", NodeType.Value(), description="Debt principal repayments").value(
        "2024Q1", 10_000.0
    ).value("2024Q2", 15_000.0).value("2024Q3", 20_000.0)

    # Income statement metrics for credit scoring
    builder.node("ebitda", NodeType.Value(), description="EBITDA").value("2024Q1", 80_000.0).value(
        "2024Q2", 85_000.0
    ).value("2024Q3", 90_000.0)

    builder.node("interest_expense", NodeType.Value(), description="Interest expense").value("2024Q1", 5_000.0).value(
        "2024Q2", 5_500.0
    ).value("2024Q3", 6_000.0)

    # Total debt calculation for leverage ratio
    builder.node("total_debt", NodeType.Formula("debt"), description="Total debt (alias for credit metrics)")

    return builder.build()


def example_1_corkscrew_validation():
    """Example 1: Balance Sheet Roll-Forward Validation (Corkscrew).

    Validates that balance sheet accounts properly roll forward:
    Ending Balance = Beginning Balance + Additions - Reductions
    """
    print("\n" + "=" * 70)
    print("Example 1: Corkscrew Balance Sheet Validation")
    print("=" * 70)

    # Create model
    model = create_balance_sheet_model()

    # Evaluate model
    evaluator = Evaluator()
    results = evaluator.evaluate(model)

    print("\nModel evaluated successfully")
    print(f"Nodes: {results.meta().num_nodes}")
    print(f"Periods: {results.meta().num_periods}")

    # Configure corkscrew extension
    print("\nConfiguring corkscrew extension...")

    corkscrew_config = CorkscrewConfig(
        accounts=[
            CorkscrewAccount(node_id="cash", account_type=AccountType.ASSET, changes=["cash_inflows", "cash_outflows"]),
            CorkscrewAccount(
                node_id="debt", account_type=AccountType.LIABILITY, changes=["debt_issuance", "debt_repayment"]
            ),
        ],
        tolerance=0.01,
        fail_on_error=False,
    )

    print(f"Configuration: {corkscrew_config!r}")
    print(f"  - Accounts tracked: {len(corkscrew_config.accounts)}")
    print(f"  - Tolerance: {corkscrew_config.tolerance}")
    print(f"  - Fail on error: {corkscrew_config.fail_on_error}")

    # Create extension
    corkscrew = CorkscrewExtension.with_config(corkscrew_config)
    print(f"\nExtension created: {corkscrew!r}")

    # Create registry and execute extension
    print("\nExecuting corkscrew validation...")
    ExtensionRegistry.new()
    # Note: registry.register() is not yet exposed, so we demonstrate
    # the configuration API here. In production, you would register
    # the extension and call registry.execute_all()

    print("✓ Corkscrew extension configured successfully")
    print("\n" + "-" * 70)
    print("Manual validation (demonstration):")

    # Manually validate cash roll-forward
    q1_cash = results.get("cash", PeriodId.from_str("2024Q1"))
    q1_inflows = results.get("cash_inflows", PeriodId.from_str("2024Q1"))
    q1_outflows = results.get("cash_outflows", PeriodId.from_str("2024Q1"))
    q2_cash_expected = q1_cash + q1_inflows - q1_outflows

    print("\nCash roll-forward Q1 -> Q2:")
    print(f"  Beginning balance (Q1): ${q1_cash:,.2f}")
    print(f"  + Inflows (Q1):         ${q1_inflows:,.2f}")
    print(f"  - Outflows (Q1):        ${q1_outflows:,.2f}")
    print(f"  = Expected Q2 balance:  ${q2_cash_expected:,.2f}")
    print("\n✓ Balance sheet corkscrew validation complete")


def example_2_credit_scorecard():
    """Example 2: Credit Rating Assignment (Scorecard).

    Assigns credit ratings based on financial metrics and thresholds.
    """
    print("\n" + "=" * 70)
    print("Example 2: Credit Scorecard Rating Assignment")
    print("=" * 70)

    # Create model
    model = create_balance_sheet_model()

    # Evaluate model
    evaluator = Evaluator()
    results = evaluator.evaluate(model)

    print("\nModel evaluated successfully")

    # Configure scorecard extension
    print("\nConfiguring credit scorecard extension...")

    scorecard_config = ScorecardConfig(
        rating_scale="S&P",
        metrics=[
            ScorecardMetric(
                name="debt_to_ebitda",
                formula="total_debt / ttm(ebitda)",
                weight=0.4,
                thresholds={
                    "AAA": (0.0, 1.0),
                    "AA": (1.0, 2.0),
                    "A": (2.0, 3.0),
                    "BBB": (3.0, 4.0),
                    "BB": (4.0, 5.0),
                    "B": (5.0, 10.0),
                    "CCC": (10.0, 999.0),
                },
                description="Leverage ratio",
            ),
            ScorecardMetric(
                name="interest_coverage",
                formula="ebitda / interest_expense",
                weight=0.3,
                thresholds={
                    "AAA": (10.0, 999.0),
                    "AA": (8.0, 10.0),
                    "A": (6.0, 8.0),
                    "BBB": (4.0, 6.0),
                    "BB": (3.0, 4.0),
                    "B": (2.0, 3.0),
                    "CCC": (0.0, 2.0),
                },
                description="Debt service coverage",
            ),
        ],
        min_rating="BB",
    )

    print(f"Configuration: {scorecard_config!r}")
    print(f"  - Rating scale: {scorecard_config.rating_scale}")
    print(f"  - Metrics: {len(scorecard_config.metrics)}")
    print(f"  - Minimum rating: {scorecard_config.min_rating}")

    print("\nMetrics:")
    for metric in scorecard_config.metrics:
        print(f"  - {metric.name} (weight: {metric.weight})")
        print(f"    Formula: {metric.formula}")

    # Create extension
    scorecard = CreditScorecardExtension.with_config(scorecard_config)
    print(f"\nExtension created: {scorecard!r}")

    # Manual scorecard calculation (demonstration)
    print("\n" + "-" * 70)
    print("Manual scorecard calculation (demonstration):")

    q3_debt = results.get("total_debt", PeriodId.from_str("2024Q3"))
    q3_ebitda = results.get("ebitda", PeriodId.from_str("2024Q3"))
    q3_interest = results.get("interest_expense", PeriodId.from_str("2024Q3"))

    debt_to_ebitda = q3_debt / q3_ebitda
    interest_coverage = q3_ebitda / q3_interest

    print("\nQ3 Metrics:")
    print(f"  Debt/EBITDA:        {debt_to_ebitda:.2f}x")
    print(f"  Interest Coverage:  {interest_coverage:.2f}x")

    # Simple rating logic (would be done by extension in production)
    if debt_to_ebitda < 2.0 and interest_coverage > 8.0:
        rating = "A"
    elif debt_to_ebitda < 3.0 and interest_coverage > 6.0:
        rating = "BBB"
    elif debt_to_ebitda < 4.0 and interest_coverage > 4.0:
        rating = "BB"
    else:
        rating = "B"

    print(f"\nImplied Rating: {rating}")
    print("✓ Credit scorecard rating complete")


def example_3_configuration_serialization():
    """Example 3: Configuration Serialization.

    Demonstrates JSON serialization/deserialization of extension configs.
    """
    print("\n" + "=" * 70)
    print("Example 3: Configuration Serialization")
    print("=" * 70)

    # Create corkscrew config
    print("\nCreating corkscrew configuration...")
    corkscrew_config = CorkscrewConfig(
        accounts=[
            CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"]),
            CorkscrewAccount("debt", AccountType.LIABILITY, changes=["debt_issuance"]),
        ],
        tolerance=0.01,
    )

    # Serialize to JSON
    json_str = corkscrew_config.to_json()
    print("\nSerialized to JSON:")
    print(json_str[:200] + "..." if len(json_str) > 200 else json_str)

    # Deserialize from JSON
    config_restored = CorkscrewConfig.from_json(json_str)
    print(f"\nDeserialized config: {config_restored!r}")
    print(f"  Accounts: {len(config_restored.accounts)}")
    print(f"  Tolerance: {config_restored.tolerance}")

    # Create scorecard config
    print("\nCreating scorecard configuration...")
    scorecard_config = ScorecardConfig(
        rating_scale="Moody's",
        metrics=[ScorecardMetric("debt_to_ebitda", "debt / ebitda", weight=0.4)],
    )

    # Serialize to JSON
    json_str = scorecard_config.to_json()
    print("\nSerialized to JSON:")
    print(json_str[:200] + "..." if len(json_str) > 200 else json_str)

    # Deserialize from JSON
    config_restored = ScorecardConfig.from_json(json_str)
    print(f"\nDeserialized config: {config_restored!r}")
    print(f"  Rating scale: {config_restored.rating_scale}")
    print(f"  Metrics: {len(config_restored.metrics)}")

    print("\n✓ Configuration serialization complete")


def example_4_runtime_configuration():
    """Example 4: Runtime Configuration Updates.

    Demonstrates modifying extension configuration at runtime.
    """
    print("\n" + "=" * 70)
    print("Example 4: Runtime Configuration Updates")
    print("=" * 70)

    # Create extension with default config
    print("\nCreating corkscrew extension with default config...")
    extension = CorkscrewExtension.new()
    print(f"Initial config: {extension.config()}")

    # Set configuration
    print("\nSetting new configuration...")
    config = CorkscrewConfig(
        accounts=[CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"])],
        tolerance=0.001,
    )
    extension.set_config(config)

    # Verify configuration
    current_config = extension.config()
    print(f"Current config: {current_config!r}")
    print(f"  Tolerance: {current_config.tolerance}")

    # Update configuration
    print("\nUpdating configuration...")
    new_config = CorkscrewConfig(
        accounts=[
            CorkscrewAccount("cash", AccountType.ASSET, changes=["cash_inflows"]),
            CorkscrewAccount("debt", AccountType.LIABILITY, changes=["debt_issuance"]),
        ],
        tolerance=0.01,
    )
    extension.set_config(new_config)

    # Verify update
    updated_config = extension.config()
    print(f"Updated config: {updated_config!r}")
    print(f"  Accounts: {len(updated_config.accounts)}")
    print(f"  Tolerance: {updated_config.tolerance}")

    print("\n✓ Runtime configuration update complete")


def main():
    """Run all examples."""
    print("\n" + "=" * 70)
    print("FINSTACK STATEMENT EXTENSIONS FRAMEWORK EXAMPLES")
    print("=" * 70)

    try:
        example_1_corkscrew_validation()
        example_2_credit_scorecard()
        example_3_configuration_serialization()
        example_4_runtime_configuration()

        print("\n" + "=" * 70)
        print("ALL EXAMPLES COMPLETED SUCCESSFULLY")
        print("=" * 70)
        print("\nKey takeaways:")
        print("  1. Corkscrew extension validates balance sheet roll-forwards")
        print("  2. Scorecard extension assigns credit ratings based on metrics")
        print("  3. Configurations can be serialized to/from JSON")
        print("  4. Extensions support runtime configuration updates")
        print("\nNext steps:")
        print("  - See tests/test_extensions.py for comprehensive unit tests")
        print("  - See finstack/statements/src/extensions/ for Rust source")
        print("  - See API docs for full extension framework documentation")

    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback

        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
