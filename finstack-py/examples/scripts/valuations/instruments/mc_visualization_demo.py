#!/usr/bin/env python3
"""
Monte Carlo Path Visualization Demo

This script demonstrates how to capture and visualize Monte Carlo simulation paths
from the finstack library. It shows how to:
1. Configure path capture (all paths or sample)
2. Extract path data and convert to DataFrames
3. Visualize paths with matplotlib
4. Plot payoff evolution along paths
5. Analyze correlation structures
"""

import sys
from pathlib import Path

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

try:
    import finstack
    from finstack import Money
    from finstack.core.currency import Currency
    from finstack.valuations import PathPoint, SimulatedPath, PathDataset
    import pandas as pd
    import numpy as np
    
    # Optional: matplotlib for plotting
    try:
        import matplotlib.pyplot as plt
        import matplotlib.cm as cm
        HAS_MATPLOTLIB = True
    except ImportError:
        print("matplotlib not available - skipping plots")
        HAS_MATPLOTLIB = False

except ImportError as e:
    print(f"Error importing finstack: {e}")
    print("Please ensure finstack is installed")
    sys.exit(1)


def demo_basic_path_capture():
    """Demonstrate basic path capture with all paths."""
    print("\n" + "="*70)
    print("DEMO 1: Basic Path Capture (All Paths)")
    print("="*70)
    
    # Note: This is a placeholder showing the intended API
    # The actual pricer integration will be completed in the next phase
    
    print("""
    # Example code (once pricer integration is complete):
    
    from finstack.valuations.pricer import PathDependentPricerConfig
    from finstack.valuations.instruments import AsianOption
    
    # Configure pricer to capture all paths
    config = PathDependentPricerConfig(
        num_paths=1000,
        seed=42,
        path_capture={
            'enabled': True,
            'mode': 'all',
            'capture_payoffs': True
        }
    )
    
    # Create and price instrument
    asian = AsianOption.new(...)
    result = asian.price_with_paths(config)
    
    # Access paths
    print(f"Estimate: {result.estimate}")
    print(f"Captured {result.paths.num_captured()} paths")
    
    # Convert to DataFrame for analysis
    df = pd.DataFrame(result.paths.to_dict())
    print(df.head())
    """)


def demo_sampled_path_capture():
    """Demonstrate sampled path capture for efficiency."""
    print("\n" + "="*70)
    print("DEMO 2: Sampled Path Capture (100 out of 10,000 paths)")
    print("="*70)
    
    print("""
    # Example code for sampling paths:
    
    # Configure pricer to capture a sample of paths
    config = PathDependentPricerConfig(
        num_paths=10000,  # Run 10k paths
        seed=42,
        path_capture={
            'enabled': True,
            'mode': 'sample',
            'sample_size': 100,  # But only capture 100
            'sample_seed': 123,
            'capture_payoffs': True
        }
    )
    
    # Price and get result
    result = barrier_option.price_with_paths(config)
    
    # Sampling ratio
    ratio = result.paths.sampling_ratio()
    print(f"Sampled {ratio:.1%} of paths")
    
    # The estimate uses ALL paths, but only 100 are stored for visualization
    print(f"Estimate based on {result.estimate.num_paths} paths")
    print(f"Visualizing {result.paths.num_captured()} captured paths")
    """)


def demo_dataframe_conversion():
    """Demonstrate DataFrame conversion for analysis."""
    print("\n" + "="*70)
    print("DEMO 3: DataFrame Conversion and Analysis")
    print("="*70)
    
    print("""
    # Convert paths to long-format DataFrame
    df_long = pd.DataFrame(result.paths.to_dict())
    
    # Columns: path_id, step, time, spot, variance, payoff_value, final_value
    print("Long format (one row per timestep per path):")
    print(df_long.head(10))
    
    # Group by time to get statistics at each timestep
    stats = df_long.groupby('time')['spot'].agg(['mean', 'std', 'min', 'max'])
    print("\\nStatistics over time:")
    print(stats)
    
    # Convert to wide format for specific state variable
    wide_dict = result.paths.to_wide_dict('spot')
    df_wide = pd.DataFrame(wide_dict)
    
    # Columns: time, step, path_0, path_1, ..., path_N
    print("\\nWide format (paths as columns):")
    print(df_wide.head())
    """)


def demo_visualization():
    """Demonstrate visualization with matplotlib."""
    print("\n" + "="*70)
    print("DEMO 4: Path Visualization")
    print("="*70)
    
    if not HAS_MATPLOTLIB:
        print("Matplotlib not available - skipping visualization demo")
        return
    
    print("""
    # Plot all captured paths
    import matplotlib.pyplot as plt
    
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    
    # 1. Plot individual paths
    ax = axes[0, 0]
    for path in result.paths.paths:
        times = [pt.time for pt in path.points]
        spots = [pt.get_var('spot') for pt in path.points]
        ax.plot(times, spots, alpha=0.3, linewidth=0.5)
    ax.set_title('All Simulated Paths')
    ax.set_xlabel('Time (years)')
    ax.set_ylabel('Spot Price')
    ax.grid(True, alpha=0.3)
    
    # 2. Plot mean path with confidence bands
    ax = axes[0, 1]
    df = pd.DataFrame(result.paths.to_dict())
    grouped = df.groupby('time')['spot']
    mean = grouped.mean()
    std = grouped.std()
    
    ax.plot(mean.index, mean.values, 'b-', linewidth=2, label='Mean')
    ax.fill_between(mean.index, 
                     mean - 2*std, mean + 2*std, 
                     alpha=0.3, label='±2 std')
    ax.set_title('Mean Path with Confidence Bands')
    ax.set_xlabel('Time (years)')
    ax.set_ylabel('Spot Price')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    # 3. Plot payoff evolution
    ax = axes[1, 0]
    for path in result.paths.paths:
        times = [pt.time for pt in path.points]
        payoffs = [pt.payoff_value or 0 for pt in path.points]
        ax.plot(times, payoffs, alpha=0.3, linewidth=0.5)
    ax.set_title('Payoff Evolution Along Paths')
    ax.set_xlabel('Time (years)')
    ax.set_ylabel('Payoff Value')
    ax.grid(True, alpha=0.3)
    
    # 4. Distribution of final values
    ax = axes[1, 1]
    final_values = [path.final_value for path in result.paths.paths]
    ax.hist(final_values, bins=50, alpha=0.7, edgecolor='black')
    ax.axvline(result.estimate.mean.amount(), color='r', 
               linestyle='--', linewidth=2, label='Mean')
    ax.set_title('Distribution of Final Values')
    ax.set_xlabel('Discounted Payoff')
    ax.set_ylabel('Frequency')
    ax.legend()
    ax.grid(True, alpha=0.3, axis='y')
    
    plt.tight_layout()
    plt.savefig('mc_paths_visualization.png', dpi=150, bbox_inches='tight')
    print("Saved visualization to mc_paths_visualization.png")
    """)


def demo_correlation_analysis():
    """Demonstrate correlation matrix analysis."""
    print("\n" + "="*70)
    print("DEMO 5: Correlation and Multi-Factor Analysis")
    print("="*70)
    
    print("""
    # For multi-factor processes (e.g., Heston, RevolvingCredit)
    
    # Extract process parameters
    params = result.paths.process_params
    print(f"Process type: {params.process_type}")
    print(f"Parameters: {params.parameters}")
    print(f"Factors: {params.factor_names}")
    
    # Get correlation matrix
    corr_matrix = params.correlation_matrix()
    if corr_matrix:
        import numpy as np
        corr = np.array(corr_matrix)
        print("\\nCorrelation Matrix:")
        print(corr)
        
        # Visualize correlation matrix
        import matplotlib.pyplot as plt
        
        fig, ax = plt.subplots(figsize=(8, 6))
        im = ax.imshow(corr, cmap='RdBu_r', vmin=-1, vmax=1)
        
        # Add colorbar
        cbar = plt.colorbar(im, ax=ax)
        cbar.set_label('Correlation', rotation=270, labelpad=15)
        
        # Add labels
        factors = params.factor_names
        ax.set_xticks(range(len(factors)))
        ax.set_yticks(range(len(factors)))
        ax.set_xticklabels(factors)
        ax.set_yticklabels(factors)
        
        # Add correlation values
        for i in range(len(factors)):
            for j in range(len(factors)):
                text = ax.text(j, i, f'{corr[i, j]:.2f}',
                             ha="center", va="center", color="black")
        
        ax.set_title('Process Correlation Matrix')
        plt.tight_layout()
        plt.savefig('correlation_matrix.png', dpi=150)
        print("Saved correlation matrix to correlation_matrix.png")
    """)


def demo_path_specific_analysis():
    """Demonstrate analyzing specific paths."""
    print("\n" + "="*70)
    print("DEMO 6: Path-Specific Analysis")
    print("="*70)
    
    print("""
    # Access individual paths
    dataset = result.paths
    
    # Find paths that hit barriers, knocked out, etc.
    paths_over_barrier = []
    for path in dataset.paths:
        max_spot = max(pt.get_var('spot') or 0 for pt in path.points)
        if max_spot > barrier_level:
            paths_over_barrier.append(path)
    
    print(f"Paths that crossed barrier: {len(paths_over_barrier)}")
    
    # Analyze path with highest payoff
    best_path = max(dataset.paths, key=lambda p: p.final_value)
    print(f"\\nBest path (ID {best_path.path_id}):")
    print(f"  Final value: {best_path.final_value:.4f}")
    print(f"  Initial spot: {best_path.initial_point().spot():.2f}")
    print(f"  Final spot: {best_path.terminal_point().spot():.2f}")
    
    # Plot specific paths of interest
    import matplotlib.pyplot as plt
    
    fig, ax = plt.subplots(figsize=(10, 6))
    
    # Plot all paths in gray
    for path in dataset.paths:
        times = [pt.time for pt in path.points]
        spots = [pt.get_var('spot') or 0 for pt in path.points]
        ax.plot(times, spots, 'gray', alpha=0.2, linewidth=0.5)
    
    # Highlight specific paths
    for path in paths_over_barrier[:5]:  # Show first 5
        times = [pt.time for pt in path.points]
        spots = [pt.get_var('spot') or 0 for pt in path.points]
        ax.plot(times, spots, 'r-', alpha=0.7, linewidth=1.5)
    
    # Plot barrier level
    ax.axhline(y=barrier_level, color='b', linestyle='--', 
               linewidth=2, label='Barrier')
    
    ax.set_title('Paths Crossing Barrier (highlighted in red)')
    ax.set_xlabel('Time (years)')
    ax.set_ylabel('Spot Price')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    plt.savefig('specific_paths.png', dpi=150)
    print("Saved specific path analysis to specific_paths.png")
    """)


def main():
    """Run all demonstration examples."""
    print("\n" + "="*70)
    print("MONTE CARLO PATH VISUALIZATION DEMONSTRATION")
    print("="*70)
    print("\nThis script demonstrates the Monte Carlo path capture and")
    print("visualization capabilities in the finstack library.")
    print("\nNote: Some examples show the intended API that will be fully")
    print("functional once pricer integration is complete.")
    
    demo_basic_path_capture()
    demo_sampled_path_capture()
    demo_dataframe_conversion()
    demo_visualization()
    demo_correlation_analysis()
    demo_path_specific_analysis()
    
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print("""
    Key Takeaways:
    
    1. Path Capture Configuration
       - Enable via path_capture parameter in pricer config
       - Choose 'all' for full capture or 'sample' for efficiency
       - Option to capture payoff values at each timestep
    
    2. Data Access
       - Paths stored in PathDataset with metadata
       - Easy conversion to pandas DataFrames (long or wide format)
       - Access individual paths, points, and state variables
    
    3. Visualization
       - Plot individual paths or aggregated statistics
       - Analyze payoff evolution along paths
       - Examine correlation structures
       - Identify specific paths of interest (barriers, extremes, etc.)
    
    4. Process Parameters
       - Access correlation matrices
       - Extract all process parameters
       - Useful for validation and sensitivity analysis
    
    Next Steps:
    - See full API documentation in the finstack docs
    - Try with different instruments (Asian, Barrier, Autocallable)
    - Experiment with different sampling strategies
    - Build custom visualizations for your use case
    """)


if __name__ == "__main__":
    main()

