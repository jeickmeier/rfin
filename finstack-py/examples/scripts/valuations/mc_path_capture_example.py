#!/usr/bin/env python3
"""
Monte Carlo Path Capture Example

Demonstrates practical usage of path capture functionality for:
1. Generating and visualizing GBM paths
2. Analyzing correlation in multi-factor processes
3. Understanding payoff behavior along paths
"""

import sys
from pathlib import Path

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Set up output directory for artifacts
OUTPUT_DIR = Path(__file__).parent.parent.parent / "outputs"
OUTPUT_DIR.mkdir(exist_ok=True)

try:
    from finstack.valuations import (
        MonteCarloPathGenerator,
        PathPoint,
        SimulatedPath,
        PathDataset,
        ProcessParams,
        MonteCarloResult,
    )
    import pandas as pd
    import numpy as np

    HAS_FINSTACK = True
except ImportError as e:
    print(f"Error importing finstack: {e}")
    HAS_FINSTACK = False

try:
    import matplotlib.pyplot as plt
    import matplotlib.cm as cm
    from matplotlib.patches import Rectangle

    HAS_MATPLOTLIB = True
except ImportError:
    print("matplotlib not available - install for visualizations")
    HAS_MATPLOTLIB = False


def example_1_basic_path_generation():
    """Example 1: Basic GBM path generation."""
    print("\n" + "=" * 70)
    print("EXAMPLE 1: Basic GBM Path Generation")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    # Create path generator
    generator = MonteCarloPathGenerator()

    # Generate 500 GBM paths, capturing 50 for visualization
    paths = generator.generate_gbm_paths(
        initial_spot=100.0,
        r=0.05,  # 5% risk-free rate
        q=0.02,  # 2% dividend yield
        sigma=0.25,  # 25% volatility
        time_to_maturity=1.0,  # 1 year
        num_steps=252,  # Daily steps
        num_paths=500,  # Total simulation
        capture_mode="sample",
        sample_count=50,  # Only capture 50 paths
        seed=42,
    )

    print(f"Generated paths: {paths.num_paths_total}")
    print(f"Captured paths: {paths.num_captured()}")
    print(f"Sampling ratio: {paths.sampling_ratio():.1%}")
    print(f"Sampling method: {paths.sampling_method}")

    # Get state variable keys
    print(f"State variables: {paths.state_var_keys()}")

    # Access first path
    first_path = paths.path(0)
    if first_path:
        print(f"\nFirst path (ID {first_path.path_id}):")
        print(f"  Number of steps: {first_path.num_steps()}")
        print(f"  Initial spot: {first_path.initial_point().spot():.2f}")
        print(f"  Terminal spot: {first_path.terminal_point().spot():.2f}")
        print(f"  Final value: {first_path.final_value:.2f}")


def example_2_dataframe_conversion():
    """Example 2: Converting paths to pandas DataFrame."""
    print("\n" + "=" * 70)
    print("EXAMPLE 2: DataFrame Conversion")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    generator = MonteCarloPathGenerator()
    paths = generator.generate_gbm_paths(
        initial_spot=100.0,
        r=0.05,
        q=0.02,
        sigma=0.25,
        time_to_maturity=1.0,
        num_steps=100,
        num_paths=200,
        capture_mode="sample",
        sample_count=20,
        seed=42,
    )

    # Convert to long-format DataFrame
    df_long = pd.DataFrame(paths.to_dict())
    print("\nLong format DataFrame:")
    print(df_long.head(10))
    print(f"\nShape: {df_long.shape}")
    print(f"Columns: {list(df_long.columns)}")

    # Calculate statistics at each time step
    stats = df_long.groupby("time")["spot"].agg(["mean", "std", "min", "max"])
    print("\nStatistics over time (first 5 steps):")
    print(stats.head())

    # Convert to wide format
    df_wide = pd.DataFrame(paths.to_wide_dict("spot"))
    print(f"\nWide format DataFrame shape: {df_wide.shape}")
    print(f"Columns: {list(df_wide.columns)[:5]}...")  # First 5 columns


def example_3_visualization():
    """Example 3: Visualizing Monte Carlo paths."""
    print("\n" + "=" * 70)
    print("EXAMPLE 3: Path Visualization")
    print("=" * 70)

    if not HAS_FINSTACK or not HAS_MATPLOTLIB:
        print("Required libraries not available - skipping")
        return

    generator = MonteCarloPathGenerator()

    # Generate paths with different volatilities for comparison
    vol_low = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.15, 1.0, 252, 500, "sample", 30, 42
    )
    vol_high = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.35, 1.0, 252, 500, "sample", 30, 43
    )

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Low volatility paths
    ax = axes[0, 0]
    for path in vol_low.paths:
        times = [pt.time for pt in path.points]
        spots = [pt.get_var("spot") or 0 for pt in path.points]
        ax.plot(times, spots, alpha=0.4, linewidth=0.8, color="steelblue")
    ax.axhline(y=100, color="red", linestyle="--", linewidth=1, label="Initial")
    ax.set_title("Low Volatility (σ=15%)")
    ax.set_xlabel("Time (years)")
    ax.set_ylabel("Spot Price")
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Plot 2: High volatility paths
    ax = axes[0, 1]
    for path in vol_high.paths:
        times = [pt.time for pt in path.points]
        spots = [pt.get_var("spot") or 0 for pt in path.points]
        ax.plot(times, spots, alpha=0.4, linewidth=0.8, color="coral")
    ax.axhline(y=100, color="red", linestyle="--", linewidth=1, label="Initial")
    ax.set_title("High Volatility (σ=35%)")
    ax.set_xlabel("Time (years)")
    ax.set_ylabel("Spot Price")
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Plot 3: Distribution comparison at maturity
    ax = axes[1, 0]
    final_low = [path.terminal_point().spot() for path in vol_low.paths]
    final_high = [path.terminal_point().spot() for path in vol_high.paths]

    ax.hist(final_low, bins=20, alpha=0.6, label="Low Vol", color="steelblue")
    ax.hist(final_high, bins=20, alpha=0.6, label="High Vol", color="coral")
    ax.axvline(x=100, color="red", linestyle="--", linewidth=1, label="Initial")
    ax.set_title("Terminal Distribution Comparison")
    ax.set_xlabel("Spot Price at Maturity")
    ax.set_ylabel("Frequency")
    ax.legend()
    ax.grid(True, alpha=0.3, axis="y")

    # Plot 4: Mean path with confidence bands
    ax = axes[1, 1]
    df_high = pd.DataFrame(vol_high.to_dict())
    grouped = df_high.groupby("time")["spot"]

    mean = grouped.mean()
    std = grouped.std()

    ax.plot(mean.index, mean.values, "b-", linewidth=2, label="Mean Path")
    ax.fill_between(
        mean.index, mean - 2 * std, mean + 2 * std, alpha=0.3, label="±2σ"
    )
    ax.axhline(y=100, color="red", linestyle="--", linewidth=1, label="Initial")
    ax.set_title("Mean Path with Confidence Bands (High Vol)")
    ax.set_xlabel("Time (years)")
    ax.set_ylabel("Spot Price")
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    output_file = OUTPUT_DIR / "mc_path_examples.png"
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"Saved visualization to: {output_file}")


def example_4_process_parameters():
    """Example 4: Analyzing process parameters."""
    print("\n" + "=" * 70)
    print("EXAMPLE 4: Process Parameters Analysis")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    generator = MonteCarloPathGenerator()
    paths = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.25, 1.0, 252, 100, "all", seed=42
    )

    # Access process parameters (available via internal API)
    # In production, this would come from result.paths.process_params
    print("Process parameters would include:")
    print("  - Process type: GBM")
    print("  - r (risk-free rate): 0.05")
    print("  - q (dividend yield): 0.02")
    print("  - sigma (volatility): 0.25")
    print("  - Factor names: ['spot']")
    print("  - Correlation: None (single factor)")


def example_5_barrier_analysis():
    """Example 5: Analyzing barrier hits in paths."""
    print("\n" + "=" * 70)
    print("EXAMPLE 5: Barrier Hit Analysis")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    generator = MonteCarloPathGenerator()
    paths = generator.generate_gbm_paths(
        initial_spot=100.0,
        r=0.05,
        q=0.02,
        sigma=0.30,
        time_to_maturity=1.0,
        num_steps=252,
        num_paths=500,
        capture_mode="sample",
        sample_count=100,
        seed=42,
    )

    # Define barrier levels
    upper_barrier = 120.0
    lower_barrier = 85.0

    # Analyze paths
    paths_hit_upper = []
    paths_hit_lower = []
    paths_in_range = []

    for path in paths.paths:
        max_spot = max((pt.get_var("spot") or 0) for pt in path.points)
        min_spot = min((pt.get_var("spot") or 0) for pt in path.points)

        if max_spot >= upper_barrier:
            paths_hit_upper.append(path)
        elif min_spot <= lower_barrier:
            paths_hit_lower.append(path)
        else:
            paths_in_range.append(path)

    print(f"Total captured paths: {paths.num_captured()}")
    print(f"Paths hitting upper barrier ({upper_barrier}): {len(paths_hit_upper)}")
    print(f"Paths hitting lower barrier ({lower_barrier}): {len(paths_hit_lower)}")
    print(f"Paths staying in range: {len(paths_in_range)}")
    print(
        f"Estimated knock-out rate: {(len(paths_hit_upper) + len(paths_hit_lower)) / paths.num_captured():.1%}"
    )

    if HAS_MATPLOTLIB:
        # Visualize paths by category
        fig, ax = plt.subplots(figsize=(12, 7))

        # Plot paths in range (green)
        for path in paths_in_range:
            times = [pt.time for pt in path.points]
            spots = [pt.get_var("spot") or 0 for pt in path.points]
            ax.plot(times, spots, color="green", alpha=0.3, linewidth=0.6)

        # Plot paths hitting upper barrier (red)
        for path in paths_hit_upper:
            times = [pt.time for pt in path.points]
            spots = [pt.get_var("spot") or 0 for pt in path.points]
            ax.plot(times, spots, color="red", alpha=0.5, linewidth=0.8)

        # Plot paths hitting lower barrier (orange)
        for path in paths_hit_lower:
            times = [pt.time for pt in path.points]
            spots = [pt.get_var("spot") or 0 for pt in path.points]
            ax.plot(times, spots, color="orange", alpha=0.5, linewidth=0.8)

        # Add barrier lines
        ax.axhline(
            y=upper_barrier,
            color="darkred",
            linestyle="--",
            linewidth=2,
            label="Upper Barrier",
        )
        ax.axhline(
            y=lower_barrier,
            color="darkorange",
            linestyle="--",
            linewidth=2,
            label="Lower Barrier",
        )
        ax.axhline(
            y=100, color="black", linestyle="-", linewidth=1, alpha=0.5, label="Initial"
        )

        ax.set_title("Barrier Option: Path Classification")
        ax.set_xlabel("Time (years)")
        ax.set_ylabel("Spot Price")
        ax.legend()
        ax.grid(True, alpha=0.3)

        output_file = OUTPUT_DIR / "barrier_analysis.png"
        plt.savefig(output_file, dpi=150, bbox_inches="tight")
        print(f"\nSaved barrier analysis to: {output_file}")


def example_6_export_for_external_analysis():
    """Example 6: Exporting data for external analysis tools."""
    print("\n" + "=" * 70)
    print("EXAMPLE 6: Data Export")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    generator = MonteCarloPathGenerator()
    paths = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.20, 2.0, 500, 100, "sample", 50, 42
    )

    # Convert to DataFrame
    df = pd.DataFrame(paths.to_dict())

    # Export to CSV for external tools
    csv_file = OUTPUT_DIR / "mc_paths_export.csv"
    df.to_csv(csv_file, index=False)
    print(f"Exported {len(df)} rows to {csv_file}")

    # Export to Parquet for efficient storage
    parquet_file = OUTPUT_DIR / "mc_paths_export.parquet"
    df.to_parquet(parquet_file, index=False)
    print(f"Exported to {parquet_file} (compressed)")

    # Show summary statistics
    print("\nSummary Statistics:")
    print(df.groupby("path_id")["spot"].agg(["count", "mean", "std", "min", "max"]).head())


def example_7_sampling_strategies():
    """Example 7: Comparing all vs sample capture."""
    print("\n" + "=" * 70)
    print("EXAMPLE 7: Capture Mode Comparison")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("finstack not available - skipping")
        return

    generator = MonteCarloPathGenerator()

    # Small simulation - capture all
    print("\nSmall simulation (100 paths, capture all):")
    paths_all = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.20, 1.0, 50, 100, "all", seed=42
    )
    print(f"  Captured: {paths_all.num_captured()}/{paths_all.num_paths_total}")
    print(f"  Is complete: {paths_all.is_complete()}")

    # Large simulation - capture sample
    print("\nLarge simulation (2,000 paths, sample 100):")
    paths_sample = generator.generate_gbm_paths(
        100.0, 0.05, 0.02, 0.20, 1.0, 50, 2000, "sample", 100, 42
    )
    print(f"  Captured: {paths_sample.num_captured()}/{paths_sample.num_paths_total}")
    print(f"  Sampling ratio: {paths_sample.sampling_ratio():.2%}")
    print(f"  Is complete: {paths_sample.is_complete()}")

    print("\nMemory efficiency:")
    df_all = pd.DataFrame(paths_all.to_dict())
    df_sample = pd.DataFrame(paths_sample.to_dict())
    print(
        f"  All paths (100): {len(df_all)} rows, ~{df_all.memory_usage(deep=True).sum() / 1024:.1f} KB"
    )
    print(
        f"  Sample (100 of 2k): {len(df_sample)} rows, ~{df_sample.memory_usage(deep=True).sum() / 1024:.1f} KB"
    )
    print(
        "  => Same memory usage, but second simulation has 20x better statistics!"
    )


def main():
    """Run all examples."""
    print("\n" + "=" * 70)
    print("MONTE CARLO PATH CAPTURE - PRACTICAL EXAMPLES")
    print("=" * 70)

    if not HAS_FINSTACK:
        print("\nERROR: finstack module not found")
        print("Please build and install finstack-py first:")
        print("  cd finstack-py")
        print("  pip install -e .")
        return

    example_1_basic_path_generation()
    example_2_dataframe_conversion()
    example_3_visualization()
    example_4_process_parameters()
    example_5_barrier_analysis()
    example_6_export_for_external_analysis()
    example_7_sampling_strategies()

    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print("""
Key Capabilities Demonstrated:

1. Path Generation
   - MonteCarloPathGenerator for standalone path simulation
   - Flexible capture modes (all or sample)
   - Deterministic and reproducible (via seed)

2. Data Access
   - Easy DataFrame conversion (long and wide formats)
   - Access individual paths and points
   - Extract state variables at any timestep

3. Analysis
   - Barrier hit detection
   - Statistical aggregation over time
   - Path classification and filtering

4. Visualization
   - Individual path plotting
   - Mean paths with confidence bands
   - Distribution analysis
   - Multi-scenario comparison

5. Export
   - CSV for spreadsheet tools
   - Parquet for efficient storage
   - Ready for additional analysis in Python ecosystem

Next Steps:
   - Use with actual instrument pricing (via price_with_paths)
   - Analyze multi-factor processes (Heston, RevolvingCredit)
   - Build custom visualizations for specific use cases
   - Integrate into model validation workflows
    """)


if __name__ == "__main__":
    main()

