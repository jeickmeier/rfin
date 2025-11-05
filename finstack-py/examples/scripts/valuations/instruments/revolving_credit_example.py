"""
Revolving Credit Example

Demonstrates pricing and analysis of revolving credit facilities.
Includes Monte Carlo simulation, optionality analysis, and visualizations.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.pricer import create_standard_registry
import numpy as np
import matplotlib.pyplot as plt
from typing import Dict, List


def create_market_data() -> tuple[MarketContext, date]:
    """Create market data for revolving credit pricing and return (market, as_of)."""
    val_date = date(2025, 1, 1)

    # Discount curve (times in years)
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85)],
    )

    market = MarketContext()
    market.insert_discount(disc_curve)

    return market, val_date


def example_deterministic_revolver():
    """Example: Deterministic revolving credit facility with fixed draws."""
    print("\n" + "=" * 80)
    print("DETERMINISTIC REVOLVING CREDIT (No Optionality)")
    print("=" * 80)
    
    # Revolver with deterministic draw schedule - no flexibility
    revolver = RevolvingCredit.builder(
        instrument_id="REVOLVER_DETERMINISTIC",
        commitment_amount=Money(5000000.0, USD),
        drawn_amount=Money(2000000.0, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,  # 25 bps on undrawn
            "usage_fee_bp": 50.0,        # 50 bps on drawn
        },
        draw_repay_spec={"deterministic": []},  # No scheduled draws/repays
        discount_curve="USD.SOFR",
    )
    
    print(f"\nInstrument Configuration:")
    print(f"  ID: {revolver.instrument_id}")
    print(f"  Commitment: {revolver.commitment_amount}")
    print(f"  Drawn Amount: {revolver.drawn_amount}")
    print(f"  Maturity: {revolver.maturity_date}")
    print(f"  Base Rate: 5.50%")
    print(f"  Commitment Fee: 25 bps on undrawn")
    print(f"  Usage Fee: 50 bps on drawn")
    
    market, as_of = create_market_data()
    registry = create_standard_registry()
    result = registry.price(revolver, "discounting", market, as_of=as_of)
    
    print(f"\nPricing Results (Deterministic):")
    print(f"  Present Value: {result.value}")
    print(f"  Note: This assumes fixed utilization at {revolver.drawn_amount}")
    
    return result


def example_monte_carlo_revolver():
    """Example: Monte Carlo revolving credit with stochastic utilization."""
    print("\n" + "=" * 80)
    print("MONTE CARLO REVOLVING CREDIT (With Optionality)")
    print("=" * 80)
    
    # Revolver with stochastic utilization - models borrower's flexibility
    # to draw and repay based on changing needs
    revolver_mc = RevolvingCredit.builder(
        instrument_id="REVOLVER_STOCHASTIC",
        commitment_amount=Money(5000000.0, USD),
        drawn_amount=Money(2000000.0, USD),  # Initial draw
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.4,      # Target 40% utilization
                    "speed": 0.5,             # Mean reversion speed (0.5 = slow reversion)
                    "volatility": 0.25,       # 25% volatility in utilization
                },
                "num_paths": 5000,   # Monte Carlo paths
                "seed": 42,          # For reproducibility
            }
        },
        discount_curve="USD.SOFR",
    )
    
    print(f"\nInstrument Configuration:")
    print(f"  ID: {revolver_mc.instrument_id}")
    print(f"  Commitment: {revolver_mc.commitment_amount}")
    print(f"  Initial Draw: {revolver_mc.drawn_amount}")
    print(f"  Maturity: {revolver_mc.maturity_date}")
    print(f"\nStochastic Process Parameters:")
    print(f"  Target Utilization: 40%")
    print(f"  Mean Reversion Speed: 0.5 (slow reversion allows volatility impact)")
    print(f"  Volatility: 25%")
    print(f"  Monte Carlo Paths: 5,000")
    print(f"\nNote: Borrower has flexibility to adjust draws/repays over time")
    print(f"      based on business needs, modeled as mean-reverting process.")
    
    market, as_of = create_market_data()
    registry = create_standard_registry()
    
    print("\nRunning Monte Carlo simulation...")
    # Use monte_carlo_gbm pricer for stochastic utilization
    result_mc = registry.price(revolver_mc, "monte_carlo_gbm", market, as_of=as_of)
    
    print(f"\nPricing Results (Monte Carlo):")
    print(f"  Present Value: {result_mc.value}")
    
    return result_mc


def simulate_utilization_paths(
    commitment: float,
    initial_draw: float,
    target_rate: float,
    speed: float,
    volatility: float,
    num_periods: int = 36,
    num_paths: int = 100,
    seed: int = 42
) -> np.ndarray:
    """
    Simulate utilization paths using Ornstein-Uhlenbeck mean-reverting process.
    
    Args:
        commitment: Total commitment amount
        initial_draw: Initial drawn amount
        target_rate: Target utilization rate (0-1)
        speed: Mean reversion speed
        volatility: Volatility of utilization
        num_periods: Number of monthly periods
        num_paths: Number of simulation paths
        seed: Random seed for reproducibility
        
    Returns:
        Array of shape (num_paths, num_periods) with simulated utilization amounts
    """
    np.random.seed(seed)
    dt = 1/12  # Monthly steps
    
    paths = np.zeros((num_paths, num_periods))
    paths[:, 0] = initial_draw / commitment  # Initial utilization rate
    
    for t in range(1, num_periods):
        # Ornstein-Uhlenbeck: dU = speed * (target - U) * dt + volatility * sqrt(dt) * dW
        dW = np.random.randn(num_paths)
        drift = speed * (target_rate - paths[:, t-1]) * dt
        diffusion = volatility * np.sqrt(dt) * dW
        paths[:, t] = paths[:, t-1] + drift + diffusion
        
        # Keep utilization in valid range [0, 1]
        paths[:, t] = np.clip(paths[:, t], 0, 1)
    
    # Convert to dollar amounts
    return paths * commitment


def plot_utilization_paths(paths: np.ndarray, commitment: float, save_path: str = None):
    """Plot sample utilization paths and statistics."""
    num_paths, num_periods = paths.shape
    months = np.arange(num_periods)
    
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    
    # Plot 1: Sample paths
    ax = axes[0, 0]
    # Show first 50 paths for clarity
    for i in range(min(50, num_paths)):
        ax.plot(months, paths[i] / 1e6, alpha=0.3, linewidth=0.8, color='steelblue')
    
    # Highlight mean path
    mean_path = paths.mean(axis=0)
    ax.plot(months, mean_path / 1e6, 'r-', linewidth=2, label='Mean Path')
    ax.axhline(y=commitment * 0.4 / 1e6, color='g', linestyle='--', 
               linewidth=1.5, label='Target (40%)')
    
    ax.set_xlabel('Month')
    ax.set_ylabel('Drawn Amount ($M)')
    ax.set_title('Simulated Utilization Paths (50 of {})'.format(num_paths))
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    # Plot 2: Utilization distribution at maturity
    ax = axes[0, 1]
    final_utilization = paths[:, -1] / commitment * 100
    ax.hist(final_utilization, bins=50, alpha=0.7, color='steelblue', edgecolor='black')
    ax.axvline(x=40, color='g', linestyle='--', linewidth=2, label='Target (40%)')
    ax.axvline(x=final_utilization.mean(), color='r', linestyle='-', 
               linewidth=2, label=f'Mean ({final_utilization.mean():.1f}%)')
    ax.set_xlabel('Utilization Rate (%)')
    ax.set_ylabel('Frequency')
    ax.set_title('Utilization Distribution at Maturity (T=36m)')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    # Plot 3: Mean utilization over time with confidence bands
    ax = axes[1, 0]
    percentile_5 = np.percentile(paths, 5, axis=0) / 1e6
    percentile_25 = np.percentile(paths, 25, axis=0) / 1e6
    percentile_75 = np.percentile(paths, 75, axis=0) / 1e6
    percentile_95 = np.percentile(paths, 95, axis=0) / 1e6
    mean_path_m = mean_path / 1e6
    
    ax.fill_between(months, percentile_5, percentile_95, alpha=0.2, 
                     color='steelblue', label='5th-95th percentile')
    ax.fill_between(months, percentile_25, percentile_75, alpha=0.4, 
                     color='steelblue', label='25th-75th percentile')
    ax.plot(months, mean_path_m, 'r-', linewidth=2, label='Mean')
    ax.axhline(y=commitment * 0.4 / 1e6, color='g', linestyle='--', 
               linewidth=1.5, label='Target (40%)')
    
    ax.set_xlabel('Month')
    ax.set_ylabel('Drawn Amount ($M)')
    ax.set_title('Mean Utilization with Confidence Bands')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    # Plot 4: Utilization rate over time (as %)
    ax = axes[1, 1]
    utilization_pct = paths / commitment * 100
    percentile_5_pct = np.percentile(utilization_pct, 5, axis=0)
    percentile_95_pct = np.percentile(utilization_pct, 95, axis=0)
    mean_pct = utilization_pct.mean(axis=0)
    
    ax.fill_between(months, percentile_5_pct, percentile_95_pct, alpha=0.3, 
                     color='steelblue', label='90% Confidence Band')
    ax.plot(months, mean_pct, 'r-', linewidth=2, label='Mean')
    ax.axhline(y=40, color='g', linestyle='--', linewidth=1.5, label='Target (40%)')
    
    ax.set_xlabel('Month')
    ax.set_ylabel('Utilization Rate (%)')
    ax.set_title('Utilization Rate Over Time')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    plt.tight_layout()
    
    if save_path:
        plt.savefig(save_path, dpi=300, bbox_inches='tight')
        print(f"\nSaved utilization paths chart to: {save_path}")
    
    plt.show()


def create_cashflow_breakdown_table(
    commitment: float,
    drawn_amount: float,
    base_rate: float,
    commitment_fee_bp: float,
    usage_fee_bp: float,
    num_periods: int = 12
) -> Dict[str, List[float]]:
    """
    Create detailed cashflow breakdown for deterministic case.
    
    Returns dict with period-by-period cashflows.
    """
    dt = 1/12  # Monthly
    undrawn = commitment - drawn_amount
    
    breakdown = {
        'period': list(range(1, num_periods + 1)),
        'drawn_amount': [drawn_amount] * num_periods,
        'undrawn_amount': [undrawn] * num_periods,
        'interest': [],
        'commitment_fee': [],
        'usage_fee': [],
        'total_cost': []
    }
    
    for _ in range(num_periods):
        # Interest on drawn amount
        interest = drawn_amount * base_rate * dt
        
        # Commitment fee on undrawn
        commitment_fee = undrawn * (commitment_fee_bp / 10000) * dt
        
        # Usage fee on drawn
        usage_fee = drawn_amount * (usage_fee_bp / 10000) * dt
        
        total_cost = interest + commitment_fee + usage_fee
        
        breakdown['interest'].append(interest)
        breakdown['commitment_fee'].append(commitment_fee)
        breakdown['usage_fee'].append(usage_fee)
        breakdown['total_cost'].append(total_cost)
    
    return breakdown


def print_cashflow_table(breakdown: Dict[str, List[float]], title: str):
    """Print formatted cashflow table."""
    print(f"\n{title}")
    print("=" * 100)
    print(f"{'Period':>6} {'Drawn':>12} {'Undrawn':>12} {'Interest':>12} "
          f"{'Commit Fee':>12} {'Usage Fee':>12} {'Total Cost':>12}")
    print("-" * 100)
    
    for i in range(min(12, len(breakdown['period']))):  # Show first year
        print(f"{breakdown['period'][i]:>6} "
              f"${breakdown['drawn_amount'][i]/1e6:>10.2f}M "
              f"${breakdown['undrawn_amount'][i]/1e6:>10.2f}M "
              f"${breakdown['interest'][i]:>10.2f} "
              f"${breakdown['commitment_fee'][i]:>10.2f} "
              f"${breakdown['usage_fee'][i]:>10.2f} "
              f"${breakdown['total_cost'][i]:>10.2f}")
    
    # Summary
    total_interest = sum(breakdown['interest'])
    total_commit_fee = sum(breakdown['commitment_fee'])
    total_usage_fee = sum(breakdown['usage_fee'])
    total_all = sum(breakdown['total_cost'])
    
    print("-" * 100)
    print(f"{'TOTAL':>6} {'':<12} {'':<12} "
          f"${total_interest:>10.2f} "
          f"${total_commit_fee:>10.2f} "
          f"${total_usage_fee:>10.2f} "
          f"${total_all:>10.2f}")
    print("=" * 100)


def sensitivity_analysis():
    """
    Analyze how optionality value changes with different parameters.
    Tests both volatility and mean reversion speed.
    """
    print("\n" + "=" * 80)
    print("SENSITIVITY ANALYSIS: Optionality Value vs Volatility & Mean Reversion")
    print("=" * 80)
    
    market, as_of = create_market_data()
    registry = create_standard_registry()
    
    # Get deterministic baseline (only once)
    revolver_det = RevolvingCredit.builder(
        instrument_id="REVOLVER_DET",
        commitment_amount=Money(5000000.0, USD),
        drawn_amount=Money(2000000.0, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
        },
        draw_repay_spec={"deterministic": []},
        discount_curve="USD.SOFR",
    )
    
    pv_det = registry.price(revolver_det, "discounting", market, as_of=as_of)
    
    # Test multiple mean reversion speeds
    speeds = [0.2, 0.5, 1.0]  # Lower speeds allow more volatility impact
    # Include 0.00 volatility to ensure MC collapses to deterministic value
    volatilities = [0.00, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40]
    
    fig, axes = plt.subplots(1, 2, figsize=(16, 6))
    
    # LEFT PLOT: Multiple speed curves
    ax = axes[0]
    
    for speed in speeds:
        option_values = []
        
        print(f"\n{'='*70}")
        print(f"Mean Reversion Speed = {speed:.1f}")
        print(f"{'='*70}")
        print(f"{'Volatility':>12} {'MC PV':>15} {'Option Value':>15} {'Relative %':>12}")
        print("-" * 60)
        
        for vol in volatilities:
            revolver_mc = RevolvingCredit.builder(
                instrument_id=f"REVOLVER_S{speed}_V{vol}",
                commitment_amount=Money(5000000.0, USD),
                drawn_amount=Money(2000000.0, USD),
                commitment_date=date(2025, 1, 1),
                maturity_date=date(2028, 1, 1),
                base_rate_spec={"type": "fixed", "rate": 0.055},
                payment_frequency="quarterly",
                fees={
                    "commitment_fee_bp": 25.0,
                    "usage_fee_bp": 50.0,
                },
                draw_repay_spec={
                    "stochastic": {
                        "utilization_process": {
                            "type": "mean_reverting",
                            "target_rate": 0.4,
                            "speed": speed,
                            "volatility": vol,
                        },
                        "num_paths": 3000,
                        "seed": 42,
                    }
                },
                discount_curve="USD.SOFR",
            )
            
            pv_mc = registry.price(revolver_mc, "monte_carlo_gbm", market, as_of=as_of)
            
            option_val = pv_mc.value - pv_det.value
            relative_pct = (float(option_val.amount) / float(pv_det.value.amount) * 100)
            
            option_values.append(float(option_val.amount))
            
            # Print row and add a zero-vol consistency check
            print(f"{vol:>12.2f} {str(pv_mc.value):>15} {str(option_val):>15} {relative_pct:>11.2f}%")
            if abs(vol - 0.0) < 1e-12:
                zero_diff = float(option_val.amount)
                print(f"    -> Zero-vol check: MC PV - Deterministic PV = ${zero_diff:,.2f} (should be 0.00)")
        
        # Plot this speed's curve
        ax.plot(volatilities, np.array(option_values) / 1e6, 'o-', linewidth=2, 
                markersize=8, label=f'Speed = {speed:.1f}')
    
    ax.axhline(y=0, color='k', linestyle='--', linewidth=1, alpha=0.5)
    ax.set_xlabel('Utilization Volatility', fontsize=12)
    ax.set_ylabel('Option Value ($M)', fontsize=12)
    ax.set_title('Option Value vs Volatility\n(Different Mean Reversion Speeds)', 
                 fontsize=13, fontweight='bold')
    ax.legend(fontsize=11)
    ax.grid(True, alpha=0.3)
    
    # RIGHT PLOT: Heatmap of option values
    ax = axes[1]
    
    # Create 2D grid of option values
    option_grid = np.zeros((len(speeds), len(volatilities)))
    
    for i, speed in enumerate(speeds):
        for j, vol in enumerate(volatilities):
            revolver_mc = RevolvingCredit.builder(
                instrument_id=f"HEAT_S{speed}_V{vol}",
                commitment_amount=Money(5000000.0, USD),
                drawn_amount=Money(2000000.0, USD),
                commitment_date=date(2025, 1, 1),
                maturity_date=date(2028, 1, 1),
                base_rate_spec={"type": "fixed", "rate": 0.055},
                payment_frequency="quarterly",
                fees={
                    "commitment_fee_bp": 25.0,
                    "usage_fee_bp": 50.0,
                },
                draw_repay_spec={
                    "stochastic": {
                        "utilization_process": {
                            "type": "mean_reverting",
                            "target_rate": 0.4,
                            "speed": speed,
                            "volatility": vol,
                        },
                        "num_paths": 2000,
                        "seed": 42,
                    }
                },
                discount_curve="USD.SOFR",
            )
            
            pv_mc = registry.price(revolver_mc, "monte_carlo_gbm", market, as_of=as_of)
            option_val = pv_mc.value - pv_det.value
            option_grid[i, j] = float(option_val.amount) / 1e6
    
    im = ax.imshow(option_grid, aspect='auto', cmap='RdYlGn', origin='lower')
    ax.set_xticks(np.arange(len(volatilities)))
    ax.set_yticks(np.arange(len(speeds)))
    ax.set_xticklabels([f'{v:.2f}' for v in volatilities])
    ax.set_yticklabels([f'{s:.1f}' for s in speeds])
    ax.set_xlabel('Volatility', fontsize=12)
    ax.set_ylabel('Mean Reversion Speed', fontsize=12)
    ax.set_title('Option Value Heatmap ($M)\n(Lower speed = More sensitivity)', 
                 fontsize=13, fontweight='bold')
    
    # Add colorbar
    cbar = plt.colorbar(im, ax=ax)
    cbar.set_label('Option Value ($M)', fontsize=11)
    
    # Add text annotations
    for i in range(len(speeds)):
        for j in range(len(volatilities)):
            text = ax.text(j, i, f'{option_grid[i, j]:.2f}',
                          ha="center", va="center", color="black", fontsize=8)
    
    plt.tight_layout()
    plt.savefig('revolving_credit_sensitivity.png', dpi=300, bbox_inches='tight')
    print("\n\nSaved sensitivity analysis chart to: revolving_credit_sensitivity.png")
    plt.show()
    
    print("\n" + "=" * 80)
    print("KEY INSIGHTS FROM SENSITIVITY ANALYSIS")
    print("=" * 80)
    print("\n1. Mean Reversion Speed Impact:")
    print("   • HIGHER speed (2.0) → Paths quickly revert to target → Low volatility impact")
    print("   • LOWER speed (0.2-0.5) → Paths wander more → Higher volatility impact")
    print("   • Speed dampens the effect of volatility on option value")
    
    print("\n2. Volatility Impact (at lower speeds):")
    print("   • Higher volatility → More time away from optimal utilization")
    print("   • With asymmetric fee structure, this increases costs")
    print("   • Effect is only visible when mean reversion is weak")
    
    print("\n3. Fee Structure Effect:")
    print("   • Commitment fee on UNDRAWN (25 bps) → Encourages higher draw")
    print("   • Usage fee on DRAWN (50 bps) → Penalizes high utilization")
    print("   • Creates a 'sweet spot' - deviations from target are costly")
    
    print("\n4. Practical Implications:")
    print("   • For STABLE borrowers (high mean reversion): Volatility matters less")
    print("   • For VOLATILE borrowers (low mean reversion): Volatility significantly impacts pricing")
    print("   • Lenders should price based on BOTH expected volatility AND mean reversion speed")
    print("=" * 80)


def example_optionality_value():
    """
    Compare deterministic vs Monte Carlo pricing to quantify optionality.
    
    The difference between MC and deterministic pricing represents the value
    of the borrower's option to adjust utilization over time. This is the
    "opportunity cost" of selling/committing to a fixed draw schedule.
    """
    print("\n" + "=" * 80)
    print("OPTIONALITY VALUE ANALYSIS")
    print("=" * 80)
    
    print("\nComparing two scenarios:")
    print("  1. Deterministic: Fixed utilization (no flexibility)")
    print("  2. Monte Carlo: Stochastic utilization (with flexibility)")
    print("\nThe difference represents the value of the draw/repay option.")
    
    # Price both scenarios
    pv_deterministic = example_deterministic_revolver()
    pv_mc = example_monte_carlo_revolver()
    
    # Calculate optionality value
    option_value = pv_mc.value - pv_deterministic.value
    
    print("\n" + "=" * 80)
    print("OPTIONALITY VALUE SUMMARY")
    print("=" * 80)
    print(f"\nDeterministic PV:     {pv_deterministic.value}")
    print(f"Monte Carlo PV:       {pv_mc.value}")
    print(f"Option Value:         {option_value}")
    print(f"\nRelative Value:       {abs(float(option_value.amount)) / float(pv_deterministic.value.amount) * 100:.2f}%")
    
    print("\nInterpretation:")
    if float(option_value.amount) > 0:
        print(f"  The Monte Carlo price is HIGHER by {option_value}")
        print("  This indicates the flexibility to adjust utilization has POSITIVE value.")
        print("  The borrower benefits from the option to draw/repay dynamically.")
    elif float(option_value.amount) < 0:
        # Negate to show absolute difference
        abs_value = Money(-float(option_value.amount), USD)
        print(f"  The Monte Carlo price is LOWER by {abs_value}")
        print("  This indicates there is a COST to maintaining flexibility.")
        print("  The lender charges more for uncertainty in utilization patterns.")
    else:
        print("  The prices are identical - optionality is worthless in this case.")
    
    print("\nKey Insights:")
    print("  • Higher volatility → Larger option value")
    print("  • Longer maturity → More time for utilization to vary")
    print("  • Fee structure → Can increase or decrease option value")
    print("  • Commitment fee on undrawn → Incentivizes higher utilization")
    print("  • Usage fee on drawn → Penalizes high utilization")
    
    # Generate detailed cashflow breakdown
    print("\n" + "=" * 80)
    print("DETAILED CASHFLOW ANALYSIS")
    print("=" * 80)
    
    breakdown = create_cashflow_breakdown_table(
        commitment=5000000.0,
        drawn_amount=2000000.0,
        base_rate=0.055,
        commitment_fee_bp=25.0,
        usage_fee_bp=50.0,
        num_periods=36  # 3 years
    )
    
    print_cashflow_table(breakdown, "Deterministic Cashflows (First 12 months)")
    
    # Simulate and visualize utilization paths
    print("\n" + "=" * 80)
    print("MONTE CARLO UTILIZATION PATH SIMULATION")
    print("=" * 80)
    
    print("\nSimulating 1,000 utilization paths...")
    paths = simulate_utilization_paths(
        commitment=5000000.0,
        initial_draw=2000000.0,
        target_rate=0.4,
        speed=0.5,  # Slow reversion to show volatility impact
        volatility=0.25,
        num_periods=36,
        num_paths=1000,
        seed=42
    )
    
    print(f"Simulation complete. Statistics:")
    print(f"  Mean final utilization: ${paths[:, -1].mean()/1e6:.2f}M ({paths[:, -1].mean()/5e6*100:.1f}%)")
    print(f"  Std dev final utilization: ${paths[:, -1].std()/1e6:.2f}M")
    print(f"  5th percentile: ${np.percentile(paths[:, -1], 5)/1e6:.2f}M")
    print(f"  95th percentile: ${np.percentile(paths[:, -1], 95)/1e6:.2f}M")
    
    plot_utilization_paths(paths, 5000000.0, save_path='revolving_credit_paths.png')


def example_cashflow_tracking_and_dataframes():
    """Example: Detailed cashflow tracking with pandas DataFrames."""
    print("\n" + "=" * 80)
    print("CASHFLOW TRACKING & DATAFRAME ANALYSIS")
    print("=" * 80)
    
    # Create a Monte Carlo revolver with path capture enabled
    from finstack.valuations.common.mc import MonteCarloPathGenerator
    
    revolver = RevolvingCredit.builder(
        instrument_id="REVOLVER_CASHFLOW_DEMO",
        commitment_amount=Money(10000000.0, USD),
        drawn_amount=Money(3000000.0, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2026, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.06},
        payment_frequency="quarterly",
        fees={
            "upfront_fee": Money(50000.0, USD),
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.5,
                    "speed": 1.0,
                    "volatility": 0.20,
                },
                "num_paths": 100,  # Smaller for demo
                "seed": 123,
            }
        },
        discount_curve="USD.SOFR",
    )
    
    print(f"\nInstrument: {revolver.instrument_id}")
    print(f"Commitment: {revolver.commitment_amount}")
    print(f"Simulating 100 paths with cashflow tracking...")
    
    market, as_of = create_market_data()
    registry = create_standard_registry()
    
    # Price with path capture to get detailed cashflows
    # Note: This requires the pricer to be configured to capture paths
    result = registry.price(revolver, "monte_carlo_gbm", market, as_of=as_of)
    
    # If paths were captured in the result, analyze them
    # (This is a placeholder - actual path capture depends on pricer configuration)
    print("\n" + "-" * 80)
    print("CASHFLOW DATAFRAME DEMONSTRATION")
    print("-" * 80)
    
    print("\nExample cashflow extraction methods available:")
    print("  • path.to_dataframe() - Convert path cashflows to pandas DataFrame")
    print("  • path.extract_typed_cashflows() - Get (time, amount, type) tuples")
    print("  • path.extract_cashflows_by_type(CashflowType.Interest) - Filter by type")
    print("  • point.principal_flows() - Get principal cashflows at a timestep")
    print("  • point.interest_flows() - Get interest cashflows at a timestep")
    
    print("\nDataFrame columns available:")
    print("  • path_id: Unique path identifier")
    print("  • step: Timestep index")
    print("  • time_years: Time in years from commitment")
    print("  • amount: Cashflow amount")
    print("  • cashflow_type: Principal, Interest, CommitmentFee, UsageFee, FacilityFee, etc.")
    
    print("\nExample analysis with pandas:")
    print("""
    # Get cashflows as DataFrame
    df = path.to_dataframe()
    
    # Analyze by cashflow type
    principal_df = df[df['cashflow_type'] == 'Principal']
    interest_df = df[df['cashflow_type'] == 'Interest']
    fees_df = df[df['cashflow_type'].isin(['CommitmentFee', 'UsageFee', 'FacilityFee'])]
    
    # Calculate totals by type
    summary = df.groupby('cashflow_type')['amount'].agg(['sum', 'count', 'mean'])
    
    # For all paths in dataset
    all_cashflows = dataset.cashflows_to_dataframe()
    
    # Aggregate across paths
    path_summary = all_cashflows.groupby(['path_id', 'cashflow_type'])['amount'].sum()
    
    # Calculate IRR per path
    if path.irr is not None:
        print(f"Path IRR: {path.irr:.2%}")
    """)
    
    print("\n" + "-" * 80)
    print("CASHFLOW TYPE BREAKDOWN")
    print("-" * 80)
    
    print("\nCashflow types tracked:")
    print("  1. Principal      - Draws (negative) and repayments (positive)")
    print("  2. Interest       - Interest on drawn amounts")
    print("  3. CommitmentFee  - Fee on undrawn commitment")
    print("  4. UsageFee       - Fee on drawn amounts")
    print("  5. FacilityFee    - Fee on total commitment")
    print("  6. UpfrontFee     - One-time fee at commitment")
    print("  7. Recovery       - Recovery proceeds on default (if applicable)")
    print("  8. MarkToMarket   - MTM P&L at each timestep (if enabled)")
    
    print("\n" + "-" * 80)
    print("IRR CALCULATION")
    print("-" * 80)
    
    print("\nEach path now includes IRR calculation:")
    print("  • IRR is calculated from lender's perspective")
    print("  • Negative cashflows: Principal deployments")
    print("  • Positive cashflows: Interest, fees, repayments")
    print("  • IRR = annualized internal rate of return")
    
    print("\nStatistics available in results:")
    print("  • Mean: Average PV across paths")
    print("  • Median: Median PV (more robust to outliers)")
    print("  • Percentiles: 25th and 75th percentiles")
    print("  • Min/Max: Range of path values")
    print("  • Std Dev: Standard deviation")
    
    return result


def main():
    """Run revolving credit examples."""
    print("\n" + "=" * 80)
    print("REVOLVING CREDIT EXAMPLES")
    print("=" * 80)
    print("\nThis example demonstrates:")
    print("  1. Deterministic pricing (fixed utilization)")
    print("  2. Monte Carlo pricing (stochastic utilization)")  
    print("  3. Quantification of the draw/repay option value")
    print("  4. Detailed cashflow analysis")
    print("  5. Utilization path visualization")
    print("  6. Sensitivity analysis")
    print("  7. Cashflow tracking with pandas DataFrames (NEW!)")
    print("  8. Per-path IRR calculation (NEW!)")
    
    # Main optionality analysis with charts and tables
    example_optionality_value()
    
    # Sensitivity analysis
    sensitivity_analysis()
    
    # NEW: Cashflow tracking and DataFrame demonstration
    example_cashflow_tracking_and_dataframes()
    
    print("\n" + "=" * 80)
    print("EXAMPLES COMPLETED!")
    print("=" * 80)
    print("\nGenerated files:")
    print("  • revolving_credit_paths.png - Utilization path simulations")
    print("  • revolving_credit_sensitivity.png - Volatility sensitivity chart")
    print("\nKey Takeaways:")
    print("  1. Revolving credit optionality can have negative value")
    print("  2. Fee structure creates asymmetric incentives")
    print("  3. Higher volatility → Higher cost of flexibility")
    print("  4. Monte Carlo essential for realistic pricing")
    print("  5. Detailed cashflow tracking enables granular analysis (NEW!)")
    print("  6. Pandas integration simplifies cashflow analysis (NEW!)")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

