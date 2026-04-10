"""Visualization functions for IRR analysis results."""

from pathlib import Path
from typing import Any

import numpy as np

from .setup import OUTPUT_DIR

try:
    from matplotlib import gridspec
    from matplotlib.gridspec import GridSpec
    import matplotlib.pyplot as plt
except ImportError:
    pass


def plot_single_scenario_analysis(results: dict[str, Any], output_file: Path | None = None) -> None:
    """Create comprehensive visualization for single scenario analysis."""
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_single_scenario.png"
    fig = plt.figure(figsize=(16, 10))
    gs = GridSpec(3, 2, figure=fig, height_ratios=[2, 1.5, 1.5])

    # 1. IRR Distribution
    ax1 = fig.add_subplot(gs[0, :])
    irr_dist = np.array(results["irr_distribution"]) * 100  # Convert to percentage

    if len(irr_dist) == 0:
        ax1.text(
            0.5,
            0.5,
            "No valid IRRs calculated\n(multiple sign changes in cashflows)",
            ha="center",
            va="center",
            transform=ax1.transAxes,
            fontsize=14,
        )
        ax1.set_title("IRR Distribution \u2014 No data available", fontsize=14)
        plt.tight_layout()
        plt.savefig(output_file, dpi=150, bbox_inches="tight")
        print(f"\nSingle scenario analysis saved to {output_file} (no IRR data)")
        return

    _n, _bins, _patches = ax1.hist(
        irr_dist, bins=50, alpha=0.7, color="steelblue", edgecolor="black", density=True, label="MC Distribution"
    )

    try:
        from scipy.stats import gaussian_kde  # type: ignore[import-untyped]
    except ModuleNotFoundError:
        gaussian_kde = None

    if gaussian_kde is not None and len(irr_dist) > 1:
        kde = gaussian_kde(irr_dist)
        x_range = np.linspace(irr_dist.min(), irr_dist.max(), 200)
        ax1.plot(x_range, kde(x_range), "b-", linewidth=2, label="KDE")

    if results["deterministic_irr"]:
        det_irr_pct = results["deterministic_irr"] * 100
        ax1.axvline(
            det_irr_pct, color="red", linestyle="--", linewidth=2, label=f"Deterministic IRR: {det_irr_pct:.2f}%"
        )

    mean_irr = np.mean(irr_dist)
    p5, p50, p95 = np.percentile(irr_dist, [5, 50, 95])
    ax1.axvline(mean_irr, color="green", linestyle=":", linewidth=2, label=f"Mean: {mean_irr:.2f}%")
    ax1.axvline(p50, color="orange", linestyle=":", linewidth=1.5, label=f"Median: {p50:.2f}%")

    ax1.set_xlabel("IRR (%)", fontsize=12)
    ax1.set_ylabel("Density", fontsize=12)
    ax1.set_title("IRR Distribution (10% Util Vol, 30% CS Vol)", fontsize=14, fontweight="bold")
    ax1.legend(loc="upper right")
    ax1.grid(True, alpha=0.3)

    stats_text = f"Mean: {mean_irr:.2f}%\n"
    stats_text += f"Std Dev: {np.std(irr_dist):.2f}%\n"
    stats_text += f"5th Pctl: {p5:.2f}%\n"
    stats_text += f"95th Pctl: {p95:.2f}%"
    ax1.text(
        0.02,
        0.98,
        stats_text,
        transform=ax1.transAxes,
        fontsize=10,
        verticalalignment="top",
        bbox={"boxstyle": "round", "facecolor": "wheat", "alpha": 0.5},
    )

    # 2. Utilization Paths
    ax2 = fig.add_subplot(gs[1, 0])
    if results["utilization_paths"] and results["time_points"]:
        util_paths = np.array(results["utilization_paths"]) * 100
        time_points = results["time_points"]

        for i in range(min(100, len(util_paths))):
            ax2.plot(time_points, util_paths[i], alpha=0.1, color="blue", linewidth=0.5)

        mean_path = np.mean(util_paths, axis=0)
        p5_path = np.percentile(util_paths, 5, axis=0)
        p95_path = np.percentile(util_paths, 95, axis=0)

        ax2.fill_between(time_points, p5_path, p95_path, alpha=0.3, color="lightblue", label="5th-95th percentile")
        ax2.plot(time_points, mean_path, "b-", linewidth=2, label="Mean")
        ax2.axhline(50, color="red", linestyle="--", alpha=0.5, label="Initial (50%)")

        ax2.set_xlabel("Time (years)", fontsize=11)
        ax2.set_ylabel("Utilization Rate (%)", fontsize=11)
        ax2.set_title("Utilization Rate Paths", fontsize=12, fontweight="bold")
        ax2.legend(loc="upper right", fontsize=9)
        ax2.grid(True, alpha=0.3)
        ax2.set_ylim([0, 100])

    # 3. Credit Spread Paths
    ax3 = fig.add_subplot(gs[1, 1])
    if results["credit_spread_paths"] and results["time_points"]:
        cs_paths = np.array(results["credit_spread_paths"]) * 10000  # Convert to bps

        for i in range(min(100, len(cs_paths))):
            ax3.plot(time_points, cs_paths[i], alpha=0.1, color="orange", linewidth=0.5)

        mean_path = np.mean(cs_paths, axis=0)
        p5_path = np.percentile(cs_paths, 5, axis=0)
        p95_path = np.percentile(cs_paths, 95, axis=0)

        ax3.fill_between(time_points, p5_path, p95_path, alpha=0.3, color="moccasin", label="5th-95th percentile")
        ax3.plot(time_points, mean_path, color="darkorange", linewidth=2, label="Mean")
        ax3.axhline(150, color="red", linestyle="--", alpha=0.5, label="Initial (150 bps)")

        ax3.set_xlabel("Time (years)", fontsize=11)
        ax3.set_ylabel("Credit Spread (bps)", fontsize=11)
        ax3.set_title("Credit Spread Paths", fontsize=12, fontweight="bold")
        ax3.legend(loc="upper right", fontsize=9)
        ax3.grid(True, alpha=0.3)
        ax3.set_ylim([0, max(300, np.max(p95_path) * 1.1)])

    # 4. Path Statistics Over Time
    ax4 = fig.add_subplot(gs[2, :])
    if results["utilization_paths"] and results["credit_spread_paths"] and results["time_points"]:
        util_std = np.std(np.array(results["utilization_paths"]) * 100, axis=0)
        cs_std = np.std(np.array(results["credit_spread_paths"]) * 10000, axis=0)

        ax4_twin = ax4.twinx()

        line1 = ax4.plot(time_points, util_std, "b-", linewidth=2, label="Utilization Std Dev")
        line2 = ax4_twin.plot(time_points, cs_std, "r-", linewidth=2, label="Credit Spread Std Dev")

        ax4.set_xlabel("Time (years)", fontsize=11)
        ax4.set_ylabel("Utilization Std Dev (%)", fontsize=11, color="b")
        ax4_twin.set_ylabel("Credit Spread Std Dev (bps)", fontsize=11, color="r")
        ax4.tick_params(axis="y", labelcolor="b")
        ax4_twin.tick_params(axis="y", labelcolor="r")
        ax4.set_title("Path Volatility Over Time", fontsize=12, fontweight="bold")
        ax4.grid(True, alpha=0.3)

        lines = line1 + line2
        labels = [l.get_label() for l in lines]
        ax4.legend(lines, labels, loc="upper right")

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nSingle scenario analysis saved to {output_file}")


def plot_extreme_paths_analysis(path_irr_pairs: list[tuple[Any, float]], output_file: Path | None = None) -> None:
    """Analyze and visualize cashflow patterns for top 5 and bottom 5 IRR paths.
    Each path gets two panels: cashflow bars and cumulative cashflows.

    Args:
        path_irr_pairs: List of (path_result, irr) tuples
        output_file: Output filename for the chart
    """
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_extreme_paths.png"
    if not path_irr_pairs:
        print("No path data available for extreme paths analysis")
        return

    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    bottom_5 = sorted_pairs[:5]
    top_5 = sorted_pairs[-5:]

    fig = plt.figure(figsize=(20, 35))
    gs = gridspec.GridSpec(10, 2, figure=fig, hspace=0.3, wspace=0.25)

    fig.suptitle(
        "Cashflow Analysis: Top 5 vs Bottom 5 IRR Paths\n(With Cumulative Cashflows)",
        fontsize=16,
        fontweight="bold",
        y=0.995,
    )

    colors = {
        "Notional": "blue",
        "Fees": "orange",
        "Fixed Interest": "green",
        "Floating Interest": "lime",
    }
    default_colors = ["purple", "red", "cyan", "gold", "pink"]

    # Process bottom 5 (left column)
    for idx, (path_result, irr) in enumerate(bottom_5):
        date_cashflows, sorted_dates, x_positions, categories = _prepare_cashflow_data(
            path_result, colors, default_colors
        )
        if not sorted_dates:
            continue

        ax_bar = fig.add_subplot(gs[idx * 2, 0])
        _draw_stacked_bars(ax_bar, date_cashflows, sorted_dates, x_positions, categories, colors)
        ax_bar.set_title(f"Bottom #{idx + 1}: IRR = {irr:.2%}", fontweight="bold", fontsize=10)
        ax_bar.set_xlabel("Days from Start", fontsize=8)
        ax_bar.set_ylabel("Cashflow ($000s)", fontsize=8)
        ax_bar.grid(True, alpha=0.3)
        ax_bar.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_bar.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)

        ax_cum = fig.add_subplot(gs[idx * 2 + 1, 0])
        _draw_cumulative(ax_cum, date_cashflows, sorted_dates, x_positions, categories, colors)

    # Process top 5 (right column)
    for idx, (path_result, irr) in enumerate(top_5):
        date_cashflows, sorted_dates, x_positions, categories = _prepare_cashflow_data(
            path_result, colors, default_colors
        )
        if not sorted_dates:
            continue

        ax_bar = fig.add_subplot(gs[idx * 2, 1])
        _draw_stacked_bars(ax_bar, date_cashflows, sorted_dates, x_positions, categories, colors)
        ax_bar.set_title(f"Top #{idx + 1}: IRR = {irr:.2%}", fontweight="bold", fontsize=10)
        ax_bar.set_xlabel("Days from Start", fontsize=8)
        ax_bar.set_ylabel("Cashflow ($000s)", fontsize=8)
        ax_bar.grid(True, alpha=0.3)
        ax_bar.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_bar.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)

        ax_cum = fig.add_subplot(gs[idx * 2 + 1, 1])
        _draw_cumulative(ax_cum, date_cashflows, sorted_dates, x_positions, categories, colors)

    # Add column headers
    fig.text(0.3, 0.99, "Bottom 5 Performers", fontsize=14, fontweight="bold", ha="center")
    fig.text(0.7, 0.99, "Top 5 Performers", fontsize=14, fontweight="bold", ha="center")

    # Add summary statistics
    bottom_avg_irr = np.mean([irr for _, irr in bottom_5]) * 100
    top_avg_irr = np.mean([irr for _, irr in top_5]) * 100
    spread = top_avg_irr - bottom_avg_irr

    stats_text = f"Bottom 5 Avg: {bottom_avg_irr:.2f}%  |  Top 5 Avg: {top_avg_irr:.2f}%  |  Spread: {spread:.2f}%"
    fig.text(
        0.5, 0.005, stats_text, ha="center", fontsize=11, bbox={"boxstyle": "round", "facecolor": "wheat", "alpha": 0.5}
    )

    plt.tight_layout(rect=[0, 0.01, 1, 0.99])
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nExtreme paths analysis saved to {output_file}")


def plot_volatility_grid_comparison(
    grid_results: dict[tuple[float, float], list[float]], output_file: Path | None = None
) -> None:
    """Create overlay plot of IRR distributions for different volatility combinations."""
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_volatility_grid.png"
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle("IRR Distributions Across Volatility Grid", fontsize=16, fontweight="bold")

    colors = plt.cm.Set2(np.linspace(0, 1, 9))
    scenario_colors = {}
    color_idx = 0

    sorted_scenarios = sorted(grid_results.keys())

    # 1. Main overlay plot (top left)
    ax1 = axes[0, 0]
    for scenario, irr_dist in grid_results.items():
        if irr_dist:
            irr_pct = np.array(irr_dist) * 100
            util_vol, cs_vol = scenario
            label = f"U:{util_vol:.0%}, CS:{cs_vol:.0%}"
            color = colors[color_idx % len(colors)]
            scenario_colors[scenario] = color
            color_idx += 1

            try:
                from scipy.stats import gaussian_kde  # type: ignore[import-untyped]
            except ModuleNotFoundError:
                gaussian_kde = None

            if gaussian_kde is not None:
                kde = gaussian_kde(irr_pct)
                x_range = np.linspace(min(irr_pct) - 1, max(irr_pct) + 1, 200)
                ax1.plot(x_range, kde(x_range), linewidth=2, label=label, color=color, alpha=0.8)

    ax1.set_xlabel("IRR (%)", fontsize=12)
    ax1.set_ylabel("Density", fontsize=12)
    ax1.set_title("All Scenarios Overlay", fontsize=13, fontweight="bold")
    ax1.legend(loc="upper left", fontsize=9)
    ax1.grid(True, alpha=0.3)

    # 2. Box plots comparison (top right)
    ax2 = axes[0, 1]
    box_data = []
    box_labels = []
    box_colors = []

    for scenario in sorted_scenarios:
        if grid_results.get(scenario):
            irr_pct = np.array(grid_results[scenario]) * 100
            box_data.append(irr_pct)
            util_vol, cs_vol = scenario
            box_labels.append(f"U:{util_vol:.0%}\nCS:{cs_vol:.0%}")
            box_colors.append(scenario_colors[scenario])

    bp = ax2.boxplot(box_data, labels=box_labels, patch_artist=True) if box_data else None
    if bp is None:
        ax2.text(0.5, 0.5, "No IRR data available", ha="center", va="center", transform=ax2.transAxes)
    else:
        for patch, color in zip(bp["boxes"], box_colors, strict=False):
            patch.set_facecolor(color)
            patch.set_alpha(0.7)

    ax2.set_ylabel("IRR (%)", fontsize=12)
    ax2.set_title("Box Plot Comparison", fontsize=13, fontweight="bold")
    ax2.grid(True, alpha=0.3, axis="y")
    plt.setp(ax2.xaxis.get_majorticklabels(), fontsize=8)

    # 3. Mean vs Volatility scatter (bottom left)
    ax3 = axes[1, 0]
    means = []
    stds = []
    util_vols_plot = []
    cs_vols_plot = []

    for scenario, irr_dist in grid_results.items():
        if irr_dist:
            means.append(np.mean(irr_dist) * 100)
            stds.append(np.std(irr_dist) * 100)
            util_vols_plot.append(scenario[0] * 100)
            cs_vols_plot.append(scenario[1] * 100)

    if means:
        scatter = ax3.scatter(
            util_vols_plot, means, c=cs_vols_plot, s=100, cmap="viridis", edgecolors="black", alpha=0.7
        )
        cbar = plt.colorbar(scatter, ax=ax3)
        cbar.set_label("Credit Spread Vol (%)", fontsize=11)
    else:
        ax3.text(0.5, 0.5, "No IRR data available", ha="center", va="center", transform=ax3.transAxes)

    ax3.set_xlabel("Utilization Volatility (%)", fontsize=12)
    ax3.set_ylabel("Mean IRR (%)", fontsize=12)
    ax3.set_title("Mean IRR vs Utilization Volatility", fontsize=13, fontweight="bold")
    ax3.grid(True, alpha=0.3)

    # 4. Statistics table (bottom right)
    ax4 = axes[1, 1]
    ax4.axis("tight")
    ax4.axis("off")

    table_data = [["Scenario", "Mean IRR", "Std Dev", "5th Pctl", "95th Pctl"]]

    for scenario in sorted_scenarios:
        if grid_results.get(scenario):
            irr_pct = np.array(grid_results[scenario]) * 100
            util_vol, cs_vol = scenario
            scenario_label = f"U:{util_vol:.0%}, CS:{cs_vol:.0%}"
            mean = np.mean(irr_pct)
            std = np.std(irr_pct)
            p5 = np.percentile(irr_pct, 5)
            p95 = np.percentile(irr_pct, 95)

            table_data.append([scenario_label, f"{mean:.2f}%", f"{std:.2f}%", f"{p5:.2f}%", f"{p95:.2f}%"])

    table = ax4.table(cellText=table_data, cellLoc="center", loc="center", colWidths=[0.25, 0.15, 0.15, 0.15, 0.15])
    table.auto_set_font_size(False)
    table.set_fontsize(9)
    table.scale(1.2, 1.5)

    for i in range(len(table_data[0])):
        table[(0, i)].set_facecolor("#40466e")
        table[(0, i)].set_text_props(weight="bold", color="white")

    for i in range(1, len(table_data)):
        for j in range(len(table_data[0])):
            if i % 2 == 0:
                table[(i, j)].set_facecolor("#f0f0f0")

    ax4.set_title("Summary Statistics", fontsize=13, fontweight="bold", pad=20)

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nVolatility grid comparison saved to {output_file}")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _categorize_kind(kind_str: str) -> str:
    """Map a cashflow kind string to a display category."""
    kind_normalized = kind_str.lower()
    if "notional" in kind_normalized:
        return "Notional"
    if "fee" in kind_normalized:
        return "Fees"
    if "fixed" in kind_normalized:
        return "Fixed Interest"
    if "float" in kind_normalized or "reset" in kind_normalized:
        return "Floating Interest"
    return kind_str


def _prepare_cashflow_data(path_result, colors: dict, default_colors: list):
    """Extract and aggregate cashflows by date for a single path."""
    cashflows = path_result.cashflows.flows()
    date_cashflows: dict = {}

    for flow in cashflows:
        if flow.date not in date_cashflows:
            date_cashflows[flow.date] = {}
        category = _categorize_kind(str(flow.kind))
        amount_in_thousands = flow.amount.amount / 1000
        if category not in date_cashflows[flow.date]:
            date_cashflows[flow.date][category] = 0
        date_cashflows[flow.date][category] += amount_in_thousands

    sorted_dates = sorted(date_cashflows.keys())
    if not sorted_dates:
        return date_cashflows, [], [], []

    start_date = sorted_dates[0]
    x_positions = [(d - start_date).days for d in sorted_dates]

    all_categories: set[str] = set()
    for date_flows in date_cashflows.values():
        all_categories.update(date_flows.keys())
    categories = sorted(all_categories)

    for i, cat in enumerate(categories):
        if cat not in colors:
            colors[cat] = default_colors[i % len(default_colors)]

    return date_cashflows, sorted_dates, x_positions, categories


def _draw_stacked_bars(ax, date_cashflows, sorted_dates, x_positions, categories, colors):
    """Draw stacked bar chart of cashflows."""
    bottom_pos = np.zeros(len(sorted_dates))
    bottom_neg = np.zeros(len(sorted_dates))

    for category in categories:
        values = [date_cashflows[d].get(category, 0) for d in sorted_dates]
        pos_values = [max(0, v) for v in values]
        neg_values = [min(0, v) for v in values]

        if any(v != 0 for v in pos_values):
            ax.bar(
                x_positions,
                pos_values,
                bottom=bottom_pos,
                color=colors[category],
                alpha=0.7,
                edgecolor="black",
                width=20,
            )
            bottom_pos += pos_values

        if any(v != 0 for v in neg_values):
            ax.bar(
                x_positions,
                neg_values,
                bottom=bottom_neg,
                color=colors[category],
                alpha=0.7,
                edgecolor="black",
                width=20,
            )
            bottom_neg += neg_values


def _draw_cumulative(ax, date_cashflows, sorted_dates, x_positions, categories, colors):
    """Draw cumulative cashflow lines."""
    cumulative_by_category = {}
    for category in categories:
        cumulative = []
        running_total = 0
        for d in sorted_dates:
            running_total += date_cashflows[d].get(category, 0)
            cumulative.append(running_total)
        if any(v != 0 for v in cumulative):
            cumulative_by_category[category] = cumulative

    for category, cumulative in cumulative_by_category.items():
        ax.plot(
            x_positions,
            cumulative,
            color=colors[category],
            linewidth=1.5,
            alpha=0.8,
            label=category,
            marker="o",
            markersize=2,
        )

    total_cumulative = []
    running_total = 0
    for d in sorted_dates:
        daily_total = sum(date_cashflows[d].values())
        running_total += daily_total
        total_cumulative.append(running_total)

    ax.plot(x_positions, total_cumulative, color="black", linewidth=2.5, alpha=0.9, label="Total Net", linestyle="--")

    ax.set_xlabel("Days from Start", fontsize=8)
    ax.set_ylabel("Cumulative ($000s)", fontsize=8)
    ax.grid(True, alpha=0.3)
    ax.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
    ax.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)
    ax.legend(fontsize=6, loc="best", ncol=2)
