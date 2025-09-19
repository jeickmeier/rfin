#!/usr/bin/env python3
"""
Private Markets Fund Waterfall Engine Example

Demonstrates the equity waterfall functionality for private markets fund analysis.
Shows how to:
1. Create a waterfall specification with standard private markets fund terms
2. Model fund events (contributions and distributions)
3. Run waterfall allocation calculations
4. Compute standard private markets fund performance metrics (IRR, MOIC, DPI, TVPI)
5. Export results for analysis

This example models a typical private markets fund with:
- Return of capital first
- 8% preferred return to LPs
- 100% catch-up to GP
- 80/20 promote split thereafter
"""

import sys
from pathlib import Path

# Add the finstack Python package to the path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "finstack-py"))

try:
    import finstack
    from datetime import date
    import json
    import pandas as pd
except ImportError as e:
    print(f"Missing required package: {e}")
    print("Please ensure finstack Python bindings are built and pandas is installed")
    sys.exit(1)


def main():
    print("Private Markets Fund Waterfall Engine Example")
    print("=" * 50)

    # Example 1: Simple 2x return scenario
    print("\n1. Simple 2x Return Scenario")
    print("-" * 30)

    try:
        # Create waterfall specification
        spec = (
            finstack.WaterfallSpec.builder()
            .style("European")
            .return_of_capital()
            .preferred_irr(0.08)
            .catchup(1.0)
            .promote_tier(0.0, 0.8, 0.2)
            .build()
        )

        # Create fund events
        events = [
            finstack.FundEvent.contribution(date(2020, 1, 1), 1_000_000.0, "USD"),
            finstack.FundEvent.distribution(date(2025, 1, 1), 2_000_000.0, "USD"),
        ]

        # Create private markets fund investment
        pe_investment = finstack.PrivateMarketsFund("FUND_A", "USD", spec, events)

        # Run waterfall allocation
        ledger = pe_investment.run_waterfall()

        # Print allocation summary
        print(f"Total allocation rows: {len(ledger.rows)}")
        for row in ledger.rows:
            print(f"  {row.date}: {row.tranche}")
            print(f"    LP: ${row.to_lp:,.2f}, GP: ${row.to_gp:,.2f}")
            print(f"    LP Unreturned: ${row.lp_unreturned:,.2f}")
            if row.lp_irr_to_date:
                print(f"    LP IRR: {row.lp_irr_to_date:.2%}")

        # Compute metrics
        metrics = pe_investment.compute_metrics(
            ["lp_irr", "moic_lp", "dpi_lp", "tvpi_lp"]
        )
        print(f"\nPerformance Metrics:")
        for metric_id, value in metrics.items():
            if metric_id == "lp_irr":
                print(f"  LP IRR: {value:.2%}")
            elif metric_id == "moic_lp":
                print(f"  LP MOIC: {value:.2f}x")
            elif metric_id == "dpi_lp":
                print(f"  LP DPI: {value:.2f}")
            elif metric_id == "tvpi_lp":
                print(f"  LP TVPI: {value:.2f}")

    except Exception as e:
        print(f"Note: Python bindings not yet implemented - {e}")
        print("This example shows the intended API design")

    # Example 2: Multi-tier waterfall with American style
    print("\n2. Multi-Tier American Style Waterfall")
    print("-" * 40)

    print("Waterfall Structure:")
    print("  1. Return of Capital")
    print("  2. 8% Preferred Return")
    print("  3. 100% Catch-up to GP")
    print("  4. 80/20 split up to 12% IRR")
    print("  5. 70/30 split above 15% IRR")

    # This would be the API once Python bindings are implemented
    example_spec = {
        "style": "american",
        "tranches": [
            {"type": "return_of_capital"},
            {"type": "preferred_irr", "irr": 0.08},
            {"type": "catchup", "gp_share": 1.0},
            {
                "type": "promote_tier",
                "hurdle": {"irr": {"rate": 0.12}},
                "lp_share": 0.8,
                "gp_share": 0.2,
            },
            {
                "type": "promote_tier",
                "hurdle": {"irr": {"rate": 0.15}},
                "lp_share": 0.7,
                "gp_share": 0.3,
            },
        ],
        "irr_basis": "act_365f",
        "catchup_mode": "full",
    }

    example_events = [
        {"date": "2020-01-01", "amount": 10_000_000.0, "kind": "contribution"},
        {
            "date": "2022-06-15",
            "amount": 8_000_000.0,
            "kind": "proceeds",
            "deal_id": "Deal_Alpha",
        },
        {
            "date": "2024-03-10",
            "amount": 12_000_000.0,
            "kind": "proceeds",
            "deal_id": "Deal_Beta",
        },
        {
            "date": "2025-12-31",
            "amount": 15_000_000.0,
            "kind": "proceeds",
            "deal_id": "Deal_Gamma",
        },
    ]

    print(f"\nExample Events ({len(example_events)} total):")
    for event in example_events:
        event_date = event["date"]
        amount = event["amount"]
        kind = event["kind"]
        deal = event.get("deal_id", "Fund-level")
        print(f"  {event_date}: ${amount:,.0f} {kind} ({deal})")

    print(
        f"\nTotal Contributions: ${sum(e['amount'] for e in example_events if e['kind'] == 'contribution'):,.0f}"
    )
    print(
        f"Total Proceeds: ${sum(e['amount'] for e in example_events if e['kind'] == 'proceeds'):,.0f}"
    )
    total_multiple = sum(
        e["amount"] for e in example_events if e["kind"] == "proceeds"
    ) / sum(e["amount"] for e in example_events if e["kind"] == "contribution")
    print(f"Gross Multiple: {total_multiple:.2f}x")

    # Example 3: Export formats
    print("\n3. Data Export Capabilities")
    print("-" * 30)

    print("Available export formats:")
    print("  • JSON: Complete ledger with metadata")
    print("  • Tabular: Column headers + row data for DataFrame creation")
    print("  • LP Cashflows: Date/amount pairs for NPV calculations")
    print("  • Performance Metrics: IRR, MOIC, DPI, TVPI, Carry")

    # Show example JSON structure
    example_ledger = {
        "rows": [
            {
                "date": "2025-01-01",
                "tranche": "Return of Capital",
                "to_lp": 1000000.0,
                "to_gp": 0.0,
                "lp_unreturned": 0.0,
                "gp_carry_cum": 0.0,
                "lp_irr_to_date": 0.0,
            },
            {
                "date": "2025-01-01",
                "tranche": "Preferred Return 8.0%",
                "to_lp": 469328.0,
                "to_gp": 0.0,
                "lp_unreturned": 0.0,
                "gp_carry_cum": 0.0,
                "lp_irr_to_date": 0.08,
            },
            {
                "date": "2025-01-01",
                "tranche": "Promote 0.0%+ (80%/20%)",
                "to_lp": 424537.6,
                "to_gp": 106134.4,
                "lp_unreturned": 0.0,
                "gp_carry_cum": 106134.4,
                "lp_irr_to_date": 0.08,
            },
        ],
        "meta": {"numeric_mode": "f64", "deterministic": True},
    }

    print(f"\nExample allocation ledger structure:")
    print(json.dumps(example_ledger, indent=2))


if __name__ == "__main__":
    main()
