#!/usr/bin/env python3
"""Monte Carlo Path Capture Example.

Demonstrates practical usage of path capture functionality for:
1. Generating and visualizing GBM paths
2. Analyzing correlation in multi-factor processes
3. Understanding payoff behavior along paths
"""

from pathlib import Path
import sys

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Set up output directory for artifacts
OUTPUT_DIR = Path(__file__).parent.parent.parent.parent / "outputs"
OUTPUT_DIR.mkdir(exist_ok=True)

try:
    from finstack.valuations.common.monte_carlo import (
        MonteCarloPathGenerator,
        MonteCarloResult,
        PathDataset,
        PathPoint,
        ProcessParams,
        SimulatedPath,
    )
    import numpy as np
    import pandas as pd

    HAS_FINSTACK = True
except ImportError:
    HAS_FINSTACK = False

try:
    from matplotlib import cm
    from matplotlib.patches import Rectangle
    import matplotlib.pyplot as plt

    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False


def example_1_basic_path_generation() -> None:
    """Example 1: Basic GBM path generation."""
    if not HAS_FINSTACK:
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

    # Get state variable keys

    # Access first path
    first_path = paths.path(0)
    if first_path:
        pass


def example_2_dataframe_conversion() -> None:
    """Example 2: Converting paths to pandas DataFrame."""
    if not HAS_FINSTACK:
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

    # Calculate statistics at each time step
    df_long.groupby("time")["spot"].agg(["mean", "std", "min", "max"])

    # Convert to wide format
    pd.DataFrame(paths.to_wide_dict("spot"))


def example_3_visualization() -> None:
    """Example 3: Visualizing Monte Carlo paths."""
    if not HAS_FINSTACK or not HAS_MATPLOTLIB:
        return

    generator = MonteCarloPathGenerator()

    # Generate paths with different volatilities for comparison
    vol_low = generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.15, 1.0, 252, 500, "sample", 30, 42)
    vol_high = generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.35, 1.0, 252, 500, "sample", 30, 43)

    _fig, axes = plt.subplots(2, 2, figsize=(14, 10))

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
    ax.fill_between(mean.index, mean - 2 * std, mean + 2 * std, alpha=0.3, label="±2σ")
    ax.axhline(y=100, color="red", linestyle="--", linewidth=1, label="Initial")
    ax.set_title("Mean Path with Confidence Bands (High Vol)")
    ax.set_xlabel("Time (years)")
    ax.set_ylabel("Spot Price")
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    output_file = OUTPUT_DIR / "mc_path_examples.png"
    plt.savefig(output_file, dpi=150, bbox_inches="tight")


def example_4_process_parameters() -> None:
    """Example 4: Analyzing process parameters."""
    if not HAS_FINSTACK:
        return

    generator = MonteCarloPathGenerator()
    generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.25, 1.0, 252, 100, "all", seed=42)

    # Access process parameters (available via internal API)
    # In production, this would come from result.paths.process_params


def example_5_barrier_analysis() -> None:
    """Example 5: Analyzing barrier hits in paths."""
    if not HAS_FINSTACK:
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

    if HAS_MATPLOTLIB:
        # Visualize paths by category
        _fig, ax = plt.subplots(figsize=(12, 7))

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
        ax.axhline(y=100, color="black", linestyle="-", linewidth=1, alpha=0.5, label="Initial")

        ax.set_title("Barrier Option: Path Classification")
        ax.set_xlabel("Time (years)")
        ax.set_ylabel("Spot Price")
        ax.legend()
        ax.grid(True, alpha=0.3)

        output_file = OUTPUT_DIR / "barrier_analysis.png"
        plt.savefig(output_file, dpi=150, bbox_inches="tight")


def example_6_export_for_external_analysis() -> None:
    """Example 6: Exporting data for external analysis tools."""
    if not HAS_FINSTACK:
        return

    generator = MonteCarloPathGenerator()
    paths = generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.20, 2.0, 500, 100, "sample", 50, 42)

    # Convert to DataFrame
    df = pd.DataFrame(paths.to_dict())

    # Export to CSV for external tools
    csv_file = OUTPUT_DIR / "mc_paths_export.csv"
    df.to_csv(csv_file, index=False)

    # Export to Parquet for efficient storage
    parquet_file = OUTPUT_DIR / "mc_paths_export.parquet"
    df.to_parquet(parquet_file, index=False)

    # Show summary statistics


def example_7_sampling_strategies() -> None:
    """Example 7: Comparing all vs sample capture."""
    if not HAS_FINSTACK:
        return

    generator = MonteCarloPathGenerator()

    # Small simulation - capture all
    paths_all = generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.20, 1.0, 50, 100, "all", seed=42)

    # Large simulation - capture sample
    paths_sample = generator.generate_gbm_paths(100.0, 0.05, 0.02, 0.20, 1.0, 50, 2000, "sample", 100, 42)

    pd.DataFrame(paths_all.to_dict())
    pd.DataFrame(paths_sample.to_dict())


def main() -> None:
    """Run all examples."""
    if not HAS_FINSTACK:
        return

    example_1_basic_path_generation()
    example_2_dataframe_conversion()
    example_3_visualization()
    example_4_process_parameters()
    example_5_barrier_analysis()
    example_6_export_for_external_analysis()
    example_7_sampling_strategies()


if __name__ == "__main__":
    main()
