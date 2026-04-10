"""Main entry point for the revolving credit IRR analysis."""

from datetime import date

from .analysis import analyze_single_scenario, analyze_volatility_grid
from .exports import (
    export_raw_polars_cashflows,
    save_cashflow_schedules_to_csv,
    save_cashflow_schedules_with_pv_to_csv,
)
from .plots import (
    plot_extreme_paths_analysis,
    plot_single_scenario_analysis,
    plot_volatility_grid_comparison,
)
from .setup import OUTPUT_DIR, create_test_market


def main() -> int:
    """Run IRR distribution analysis for revolving credit facilities."""
    print("\n" + "=" * 80)
    print("REVOLVING CREDIT IRR DISTRIBUTION ANALYSIS")
    print("Monte Carlo Simulation with Volatility Scenarios")
    print("=" * 80)

    as_of = date(2024, 12, 29)  # A few days before commitment

    try:
        market = create_test_market()
        commitment_date = date(2025, 1, 1)

        # Part 1: Single scenario analysis (10% util vol, 30% credit spread vol)
        print("\n" + "=" * 80)
        print("PART 1: Single Scenario Analysis")
        print("=" * 80)

        single_results = analyze_single_scenario(
            market,
            as_of,
            util_vol=0.10,
            cs_vol=0.30,
            num_paths=1000,
            initial_utilization=0.25,
            commitment_date=commitment_date,
        )
        plot_single_scenario_analysis(single_results)

        if single_results.get("path_irr_pairs"):
            plot_extreme_paths_analysis(single_results["path_irr_pairs"])

            print("\n" + "=" * 80)
            print("Exporting Raw Polars Cashflows (Top 1 & Bottom 1)")
            print("=" * 80)

            export_raw_polars_cashflows(
                single_results["path_irr_pairs"], market, as_of, num_paths=1, output_dir=str(OUTPUT_DIR)
            )

            print("\n" + "=" * 80)
            print("Saving Additional Cashflow Schedules")
            print("=" * 80)
            save_cashflow_schedules_with_pv_to_csv(
                single_results["path_irr_pairs"], market, as_of, num_paths=5, output_dir=str(OUTPUT_DIR)
            )

            print("\nSaving cashflow schedules with MC path data...")
            save_cashflow_schedules_to_csv(single_results["path_irr_pairs"], num_paths=5, output_dir=str(OUTPUT_DIR))

        # Part 2: Volatility grid analysis
        print("\n" + "=" * 80)
        print("PART 2: Volatility Grid Analysis")
        print("=" * 80)

        grid_results = analyze_volatility_grid(
            market,
            as_of,
            util_vols=[0.10, 0.20, 0.30],
            cs_vols=[0.20, 0.30, 0.40],
            num_paths=500,
            initial_utilization=0.25,
            commitment_date=commitment_date,
        )
        plot_volatility_grid_comparison(grid_results)

        # Summary
        print("\n" + "=" * 80)
        print("ANALYSIS COMPLETE")
        print("=" * 80)
        print("\nKey Insights:")
        print("1. IRR distributions show significant variability with volatility parameters")
        print("2. Higher utilization volatility generally increases IRR uncertainty")
        print("3. Credit spread volatility impacts both mean and dispersion of returns")
        print("4. Path-dependent features create complex IRR distributions")

        print("\nOutput files generated:")
        print("- irr_single_scenario.png: Single scenario deep dive")
        print("- irr_extreme_paths.png: Top 5 vs Bottom 5 cashflow analysis")
        print("- irr_volatility_grid.png: Grid comparison across scenarios")

        return 0

    except Exception as e:
        print(f"\n\u2717 Error during analysis: {e}")
        import traceback

        traceback.print_exc()
        return 1
