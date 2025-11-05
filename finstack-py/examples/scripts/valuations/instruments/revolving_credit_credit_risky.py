"""
Credit‑Risky Revolving Credit Example (2‑factor MC)

Demonstrates pricing a revolving credit facility with stochastic utilization
and market‑anchored credit risk (hazard/spread) using the new mc_config.
"""

from datetime import date, timedelta
from typing import Optional, Dict, List, Tuple

import numpy as np
import matplotlib.pyplot as plt

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve, ForwardCurve
from finstack.core.cashflow import xirr
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    """Create minimal market inputs: discount curve + borrower hazard curve."""
    disc = DiscountCurve(
        "USD.OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.97),
            (3.0, 0.91),
        ],
    )

    # Constant hazard ~2% with 40% recovery (approx 120 bps spread)
    hazard = HazardCurve(
        "BORROWER-HZD",
        as_of,
        [
            (1.0, 0.05),
            (5.0, 0.05),
        ],
        recovery_rate=0.40,
    )

    # SOFR 1M forward curve (tenor = 1/12 years)
    sofr_1m = ForwardCurve(
        "USD.SOFR.1M",
        1.0 / 12.0,
        [
            (0.0, 0.0530),
            (1.0, 0.0550),
            (3.0, 0.0570),
        ],
        base_date=as_of,
    )

    market = MarketContext()
    market.insert_discount(disc)
    market.insert_forward(sofr_1m)
    market.insert_hazard(hazard)
    return market


def build_revolver(
    implied_vol: float,
    util_credit_corr: Optional[float] = 0.8,
    num_paths: int = 4000,
    seed: int = 42,
):
    """Create a credit‑risky revolver with market‑anchored credit process.

    implied_vol: CDS (index) option implied vol used to scale credit spread vol.
    util_credit_corr: correlation between utilization and credit (+0.8 typical).
    """
    return RevolvingCredit.builder(
        instrument_id=f"REVOLVER_CR_{implied_vol:.2f}",
        commitment_amount=Money(5_000_000, USD),
        drawn_amount=Money(1_500_000, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={
            "type": "floating",
            "index_id": "USD.SOFR.1M",
            "margin_bp": 0.0,
            "reset_freq": "monthly",
        },
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 150.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 1.00,
                    "speed": 0.50,
                    "volatility": 0.20,
                },
                "num_paths": num_paths,
                "seed": seed,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": "BORROWER-HZD",
                            "kappa": 0.50,
                            "implied_vol": implied_vol,
                            "tenor_years": None,  # default to facility’s maturity horizon
                        }
                    },
                    # Either supply a full 3×3 correlation, or just util‑credit correlation
                    "util_credit_corr": util_credit_corr,
                },
            }
        },
        discount_curve="USD.OIS",
    )


def extract_arrays_from_dataset(ds, recovery_rate: float) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """Convert captured PathDataset into numpy arrays: (times, util_paths, hazard_paths, pvs)."""
    paths = ds.paths
    if len(paths) == 0:
        return np.array([]), np.zeros((0, 0)), np.zeros((0, 0)), np.array([])
    times = np.array([pt.time for pt in paths[0].points], dtype=float)
    num_paths = len(paths)
    num_steps = len(paths[0])
    util = np.zeros((num_paths, num_steps))
    hazard = np.zeros((num_paths, num_steps))
    pvs = np.zeros((num_paths,))
    for i, p in enumerate(paths):
        pvs[i] = p.final_value
        for j, pt in enumerate(p.points):
            # Engine exposes first factor under 'spot' for generic compatibility
            u = pt.get_var("spot") or 0.0
            cs = pt.get_var("credit_spread") or 0.0
            util[i, j] = u
            hazard[i, j] = cs / max(1.0 - recovery_rate, 1e-6)
    return times, util, hazard, pvs


def compute_capital_adjusted_npv(
    inst: RevolvingCredit,
    market: MarketContext,
    as_of: date,
    times: np.ndarray,
    util_paths: np.ndarray,
    engine_pvs: np.ndarray,
) -> np.ndarray:
    """Compute lender economic NPV per path by adding capital deployment flows.

    NPV_economic = EnginePV + PV(sum of principal draw/repay flows), where
    principal CF at step k is -(P_k - P_{k-1}) and at step 0 is -P_0.
    """
    disc = market.discount("USD.OIS")
    dc = disc.day_count
    t_start = dc.year_fraction(disc.base_date, inst.commitment_date, None)

    commitment = float(inst.commitment_amount.amount)
    num_paths, num_steps = util_paths.shape

    npv = np.zeros((num_paths,), dtype=float)
    for i in range(num_paths):
        u = util_paths[i]
        P = u * commitment
        pv_cap = 0.0
        # step 0 CF
        t_abs0 = t_start + float(times[0])
        pv_cap += (-P[0]) * disc.df(t_abs0)
        # subsequent steps
        for k in range(1, num_steps):
            t_absk = t_start + float(times[k])
            dP = P[k] - P[k - 1]
            cf = -dP
            pv_cap += cf * disc.df(t_absk)
        npv[i] = float(engine_pvs[i]) + pv_cap
    return npv


def extract_cashflows_from_dataset(
    ds,
    inst: RevolvingCredit,
    market: MarketContext,
    as_of: date,
    commitment: float,
) -> Tuple[List[np.ndarray], List[np.ndarray]]:
    """Extract time and cashflow arrays for each path for IRR calculation.
    
    Returns:
        times_list: List of time arrays (one per path)
        cashflows_list: List of cashflow arrays (one per path), includes:
            - Initial capital outlay (negative)
            - Interest and fee payments (positive)
            - Principal draws/repayments (negative for draws, positive for repayments)
            - Final principal return (positive)
    """
    paths = ds.paths
    if len(paths) == 0:
        return [], []
    
    disc = market.discount("USD.OIS")
    dc = disc.day_count
    t_start = dc.year_fraction(disc.base_date, inst.commitment_date, None)
    
    times_list = []
    cashflows_list = []
    
    for path in paths:
        times = []
        cashflows = []
        prev_util = None
        
        for pt in path.points:
            t = float(pt.time)
            util = pt.get_var("spot") or 0.0
            P = util * commitment
            
            # At each step, record the time
            times.append(t)
            
            # Capital flow (draw/repayment)
            if prev_util is None:
                # Initial deployment
                cf_cap = -P
            else:
                prev_P = prev_util * commitment
                cf_cap = -(P - prev_P)  # negative for draws, positive for repayments
            
            # Interest/fee payments (from payoff_value delta, representing lender receipts)
            # payoff_value is cumulative discounted receipts, so we need to infer undiscounted
            # For simplicity, use the cumulative payoff delta as a proxy for receipts
            # Note: This is approximate; exact cashflows would need step-by-step fee/interest calc
            
            cashflows.append(cf_cap)  # For now, just capital flows
            prev_util = util
        
        times_list.append(np.array(times))
        cashflows_list.append(np.array(cashflows))
    
    return times_list, cashflows_list


def compute_irr_per_path(
    ds,
    inst: RevolvingCredit,
    *,
    base_rate_annual: float = 0.055,
    margin_bp: float = 150.0,
    commitment_fee_bp: float = 25.0,
    usage_fee_bp: float = 50.0,
    facility_fee_bp: float = 0.0,
    upfront_fee: float = 0.0,
) -> np.ndarray:
    """Calculate IRR per path using xirr with explicitly reconstructed undiscounted cashflows.

    Cashflows per step (lender perspective):
      CF = -(ΔDrawn) + (drawn * (base_rate + margin) * dt)
           + (undrawn * commitment_fee * dt) + (drawn * usage_fee * dt) + (commitment * facility_fee * dt)
      At final step, add outstanding principal (positive).
    """
    paths = ds.paths
    if len(paths) == 0:
        return np.array([])

    base_date = inst.commitment_date
    commitment = float(inst.commitment_amount.amount)

    base_rate = float(base_rate_annual)
    margin_rate = margin_bp * 1e-4
    cfee_rate = commitment_fee_bp * 1e-4
    ufee_rate = usage_fee_bp * 1e-4
    ffee_rate = facility_fee_bp * 1e-4

    irrs = []

    for i, path in enumerate(paths):
        cashflow_list: List[Tuple[date, float]] = []
        prev_util = None
        prev_time = None

        # Upfront fee at start if provided (sign per caller)
        if abs(upfront_fee) > 0.0:
            cashflow_list.append((base_date, float(upfront_fee)))

        for idx, pt in enumerate(path.points):
            t_years = float(pt.time)
            util = pt.get_var("spot") or 0.0
            P = util * commitment

            # Convert year fraction (from commitment date) to actual date
            cf_date = base_date + timedelta(days=int(t_years * 365.25))

            # Capital flow
            if prev_util is None:
                cf_cap = -P  # Initial deployment
            else:
                prev_P = prev_util * commitment
                cf_cap = -(P - prev_P)

            # Interest and fees over the period since previous point (undiscounted)
            receipts = 0.0
            if prev_time is not None:
                dt = max(t_years - prev_time, 0.0)
                rate = base_rate + margin_rate
                drawn = P
                undrawn = max(commitment - drawn, 0.0)
                interest = drawn * rate * dt
                cfee = undrawn * cfee_rate * dt
                ufee = drawn * ufee_rate * dt
                ffee = commitment * ffee_rate * dt
                receipts = interest + cfee + ufee + ffee

            # Net cashflow at this timestep
            cf_total = cf_cap + receipts

            if abs(cf_total) > 1e-6:  # Only record non-zero cashflows
                cashflow_list.append((cf_date, cf_total))

            prev_util = util
            prev_time = t_years
        # Final principal return
        if prev_util is not None and prev_util > 1e-9:
            final_P = prev_util * commitment
            final_date = base_date + timedelta(days=int(float(path.points[-1].time) * 365.25))
            if len(cashflow_list) > 0 and cashflow_list[-1][0] == final_date:
                cashflow_list[-1] = (final_date, cashflow_list[-1][1] + final_P)
            else:
                cashflow_list.append((final_date, final_P))

        # Calculate XIRR using finstack's function
        if len(cashflow_list) < 2:
            irrs.append(np.nan)
            continue

        # Check sign changes
        cfs = [cf[1] for cf in cashflow_list]
        signs = [np.sign(cf) for cf in cfs if abs(cf) > 1e-6]
        if len(set(signs)) < 2:
            # No sign change, no IRR
            irrs.append(np.nan)
            continue

        try:
            irr_val = xirr(cashflow_list, guess=0.10)
            irrs.append(irr_val)
        except Exception:
            # XIRR calculation failed
            irrs.append(np.nan)

    return np.array(irrs)


def print_punitive_path_tables_from_dataset(
    ds,
    market: MarketContext,
    inst: RevolvingCredit,
    as_of: date,
    top_k: int = 3,
    npv_economic: Optional[np.ndarray] = None,
) -> None:
    """Print tables for punitive paths with engine PV and capital-adjusted NPV columns."""
    paths = ds.paths
    if len(paths) == 0:
        print("No captured paths.")
        return
    pvs = np.array([p.final_value for p in paths])
    idx_sorted = np.argsort(pvs)

    # Setup for capital-adjusted flows
    disc = market.discount("USD.OIS")
    dc = disc.day_count
    t_start = dc.year_fraction(disc.base_date, inst.commitment_date, None)
    commitment = float(inst.commitment_amount.amount)

    for rank in range(min(top_k, len(idx_sorted))):
        pi = int(idx_sorted[rank])
        p = paths[pi]
        extra = ""
        if npv_economic is not None and pi < len(npv_economic):
            extra = f", Econ NPV=${npv_economic[pi]:,.2f}"
        print(f"\nPunitive Path #{rank + 1} (path_id={pi}, PV=${p.final_value:,.2f}{extra})")
        print("=" * 100)
        print(
            f"{'Step':>4} {'t(yr)':>6} {'Util%':>8} {'Spread(bps)':>12} "
            f"{'Eng CumPV':>14} {'Eng ΔPV':>12} {'Cap ΔPV':>12} {'Cap CumPV':>14} "
            f"{'Econ ΔPV':>12} {'Econ CumPV':>14} {'Eng RemPV':>14} {'P Outstnd':>12} {'State NPV':>14}"
        )
        print("-" * 100)
        prev_engine_cum = 0.0
        prev_P = None
        cap_cum = 0.0
        for pt in p.points[:48]:  # cap display
            util = (pt.get_var("spot") or 0.0)
            spread_bps = (pt.get_var("credit_spread") or 0.0) * 1e4

            # Engine PVs
            eng_cum = pt.payoff_value or 0.0
            eng_d = eng_cum - prev_engine_cum
            prev_engine_cum = eng_cum

            # Capital flows PVs
            P = util * commitment
            t_abs = t_start + float(pt.time)
            if prev_P is None:
                cap_flow = -P
            else:
                cap_flow = -(P - prev_P)
            prev_P = P
            cap_d = cap_flow * disc.df(t_abs)
            cap_cum += cap_d

            # Economic PVs
            econ_d = eng_d + cap_d
            econ_cum = eng_cum + cap_cum

            # Remaining PV and state NPV at this step
            eng_rem_pv = p.final_value - eng_cum
            state_npv = eng_rem_pv - P

            print(
                f"{pt.step:>4d} {pt.time:>6.2f} {100.0*util:>8.2f} {spread_bps:>12.1f} "
                f"{eng_cum:>14.2f} {eng_d:>12.2f} {cap_d:>12.2f} {cap_cum:>14.2f} "
                f"{econ_d:>12.2f} {econ_cum:>14.2f} {eng_rem_pv:>14.2f} {P:>12.2f} {state_npv:>14.2f}"
            )
        print("=" * 100)


def plot_path_analytics(
    *,
    util_paths: np.ndarray,
    hazard_paths: np.ndarray,
    pvs: List[float],
    commitment: float,
    pvs_npv: Optional[List[float]] = None,
    save_prefix: str = "revolver_credit_risky",
) -> None:
    """Create path graphs and PV analytics to illustrate optionality impacts."""
    num_paths, num_steps = util_paths.shape
    months = np.arange(num_steps)

    # Figure 1: Sample utilization and hazard paths
    cols = 3 if pvs_npv is not None else 2
    fig, axes = plt.subplots(1, cols, figsize=(20 if cols == 3 else 14, 5))

    ax = axes[0]
    for i in range(min(40, num_paths)):
        ax.plot(months, util_paths[i] * 100.0, color="steelblue", alpha=0.25, linewidth=0.8)
    ax.set_title("Utilization Paths (%)")
    ax.set_xlabel("Month")
    ax.set_ylabel("Utilization %")
    ax.grid(True, alpha=0.3)

    ax = axes[1]
    for i in range(min(40, num_paths)):
        ax.plot(months, hazard_paths[i] * 1e4, color="indianred", alpha=0.25, linewidth=0.8)
    ax.set_title("Hazard Rate Paths (bps)")
    ax.set_xlabel("Month")
    ax.set_ylabel("Hazard (annual, bps)")
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig(f"{save_prefix}_paths.png", dpi=300, bbox_inches="tight")
    print(f"Saved path charts to: {save_prefix}_paths.png")
    plt.show()

    # Figure 2: PV distribution and relationships
    cols = 3 if pvs_npv is not None else 2
    fig, axes = plt.subplots(1, cols, figsize=((20, 5) if cols == 3 else (14, 5)))

    ax = axes[0]
    ax.hist(np.array(pvs) / 1e6, bins=40, color="slateblue", alpha=0.8, edgecolor="black")
    ax.set_title("Distribution of Pathwise PV ($M)")
    ax.set_xlabel("PV ($M)")
    ax.set_ylabel("Frequency")
    ax.grid(True, alpha=0.3)

    # PV vs average utilization (proxy for cost exposure)
    avg_util = util_paths.mean(axis=1)
    ax = axes[1]
    ax.scatter(avg_util * 100.0, np.array(pvs) / 1e6, color="steelblue", alpha=0.8)
    ax.set_title("PV vs Avg Utilization")
    ax.set_xlabel("Average Utilization %")
    ax.set_ylabel("PV ($M)")
    ax.grid(True, alpha=0.3)

    if pvs_npv is not None:
        ax = axes[2]
        ax.hist(np.array(pvs_npv) / 1e6, bins=40, color="darkred", alpha=0.8, edgecolor="black")
        ax.set_title("Distribution of Economic NPV ($M)\n(capital-adjusted)")
        ax.set_xlabel("Economic NPV ($M)")
        ax.set_ylabel("Frequency")
        ax.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.savefig(f"{save_prefix}_pv_analytics.png", dpi=300, bbox_inches="tight")
    print(f"Saved PV analytics to: {save_prefix}_pv_analytics.png")
    plt.show()


def plot_irr_distributions(
    irr_datasets: Dict[str, np.ndarray],
    save_prefix: str = "revolver_irr",
) -> None:
    """Plot IRR distributions for comparison across different scenarios.
    
    Args:
        irr_datasets: Dict mapping scenario label to IRR array
        save_prefix: Prefix for saved figure filename
    """
    n_scenarios = len(irr_datasets)
    fig, axes = plt.subplots(1, min(n_scenarios, 3), figsize=(18, 5))
    if n_scenarios == 1:
        axes = [axes]
    
    colors = plt.cm.viridis(np.linspace(0, 0.9, n_scenarios))
    
    # Plot individual histograms
    for idx, (label, irrs) in enumerate(list(irr_datasets.items())[:3]):
        ax = axes[idx]
        valid_irrs = irrs[~np.isnan(irrs)] * 100  # Convert to percentage
        
        ax.hist(valid_irrs, bins=40, color=colors[idx], alpha=0.7, edgecolor="black")
        ax.axvline(np.mean(valid_irrs), color="red", linestyle="--", linewidth=2, label=f"Mean: {np.mean(valid_irrs):.1f}%")
        ax.axvline(np.median(valid_irrs), color="orange", linestyle=":", linewidth=2, label=f"Median: {np.median(valid_irrs):.1f}%")
        ax.set_title(f"IRR Distribution\n{label}")
        ax.set_xlabel("IRR (%)")
        ax.set_ylabel("Frequency")
        ax.legend(loc="upper right")
        ax.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(f"{save_prefix}_distributions.png", dpi=300, bbox_inches="tight")
    print(f"Saved IRR distribution charts to: {save_prefix}_distributions.png")
    plt.show()


def plot_irr_comparison(
    irr_datasets: Dict[str, np.ndarray],
    param_values: List[float],
    param_name: str,
    save_prefix: str = "revolver_irr_comparison",
) -> None:
    """Plot IRR statistics vs parameter values for sensitivity analysis.
    
    Args:
        irr_datasets: Dict mapping scenario label to IRR array
        param_values: List of parameter values corresponding to scenarios
        param_name: Name of the parameter being varied
        save_prefix: Prefix for saved figure filename
    """
    fig, axes = plt.subplots(1, 2, figsize=(16, 6))
    
    labels = list(irr_datasets.keys())
    means = []
    medians = []
    p5s = []
    p95s = []
    stds = []
    
    for label in labels:
        irrs = irr_datasets[label]
        valid_irrs = irrs[~np.isnan(irrs)] * 100  # Percentage
        means.append(np.mean(valid_irrs))
        medians.append(np.median(valid_irrs))
        p5s.append(np.percentile(valid_irrs, 5))
        p95s.append(np.percentile(valid_irrs, 95))
        stds.append(np.std(valid_irrs))
    
    # Plot 1: Mean/Median vs parameter
    ax = axes[0]
    ax.plot(param_values, means, 'o-', color='darkblue', linewidth=2, markersize=8, label='Mean IRR')
    ax.plot(param_values, medians, 's--', color='darkgreen', linewidth=2, markersize=8, label='Median IRR')
    ax.fill_between(param_values, p5s, p95s, alpha=0.2, color='steelblue', label='5th-95th percentile')
    ax.set_xlabel(param_name, fontsize=12)
    ax.set_ylabel("IRR (%)", fontsize=12)
    ax.set_title(f"IRR Central Tendency vs {param_name}", fontsize=13, fontweight='bold')
    ax.legend(loc='best')
    ax.grid(True, alpha=0.3)
    
    # Plot 2: Std dev vs parameter (risk measure)
    ax = axes[1]
    ax.plot(param_values, stds, 'o-', color='darkred', linewidth=2, markersize=8)
    ax.set_xlabel(param_name, fontsize=12)
    ax.set_ylabel("IRR Std Dev (%)", fontsize=12)
    ax.set_title(f"IRR Volatility vs {param_name}", fontsize=13, fontweight='bold')
    ax.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(f"{save_prefix}.png", dpi=300, bbox_inches="tight")
    print(f"Saved IRR comparison chart to: {save_prefix}.png")
    plt.show()


def run_optionality_analytics_from_pricer(inst: RevolvingCredit, market: MarketContext, as_of: date, recovery_rate: float) -> None:
    """Fetch captured paths from Rust pricer, print punitive tables, and plot charts."""
    # Capture a sample for visualization; engine computes cumulative discounted payoff per step
    mc = inst.mc_paths(market, as_of=as_of, capture_mode="sample", sample_count=200, seed=42)
    if not mc.has_paths():
        print("Pricer returned no captured paths.")
        return
    ds = mc.paths
    times, util_paths, hazard_paths, pvs = extract_arrays_from_dataset(ds, recovery_rate)
    npv_econ = compute_capital_adjusted_npv(inst, market, as_of, times, util_paths, pvs)

    # Calculate IRRs using undiscounted cashflows (XIRR)
    irrs = compute_irr_per_path(
        ds,
        inst,
        base_rate_annual=0.055,
        margin_bp=150.0,
        commitment_fee_bp=25.0,
        usage_fee_bp=50.0,
        facility_fee_bp=0.0,
        upfront_fee=0.0,
    )
    valid_irrs = irrs[~np.isnan(irrs)]

    # Summary stats
    print("\nPathwise PV summary (engine):")
    print(f"  Mean PV: ${pvs.mean():,.2f} | Median PV: ${np.median(pvs):,.2f}")
    print(f"  5th/95th pct: ${np.percentile(pvs, 5):,.2f} / ${np.percentile(pvs, 95):,.2f}")

    print("\nEconomic NPV summary (capital-adjusted):")
    print(f"  Mean NPV: ${npv_econ.mean():,.2f} | Median NPV: ${np.median(npv_econ):,.2f}")
    print(f"  5th/95th pct: ${np.percentile(npv_econ, 5):,.2f} / ${np.percentile(npv_econ, 95):,.2f}")

    if len(valid_irrs) > 0:
        print("\nIRR summary:")
        print(f"  Mean IRR: {valid_irrs.mean()*100:.2f}% | Median IRR: {np.median(valid_irrs)*100:.2f}%")
        print(f"  5th/95th pct: {np.percentile(valid_irrs, 5)*100:.2f}% / {np.percentile(valid_irrs, 95)*100:.2f}%")
        print(f"  Valid paths: {len(valid_irrs)}/{len(irrs)}")
    else:
        print("\nNo valid IRRs calculated (check cashflow patterns)")

    # Punitive paths (based on lowest PV) with cumulative payoff deltas
    print_punitive_path_tables_from_dataset(ds, market, inst, as_of, top_k=3, npv_economic=npv_econ)

    # Plots
    plot_path_analytics(
        util_paths=util_paths,
        hazard_paths=hazard_paths,
        pvs=pvs.tolist(),
        commitment=float(inst.commitment_amount.amount),
        pvs_npv=npv_econ.tolist(),
        save_prefix="revolving_credit_credit_risky",
    )


def main():
    as_of = date(2025, 1, 1)
    market = build_market(as_of)
    registry = create_standard_registry()
    commitment = 5_000_000

    print("\n=== CREDIT‑RISKY REVOLVER (Market‑Anchored Credit) ===")
    base = build_revolver(implied_vol=0.25)
    pv = registry.price(base, "monte_carlo_gbm", market, as_of=as_of).value
    print(f"Base PV (implied vol 0.25): {pv}")

    print("\nVolatility sensitivity (CDS option vol → PV):")
    for vol in [0.0001, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.5]:
        inst = build_revolver(implied_vol=vol, num_paths=3000)
        val = registry.price(inst, "monte_carlo_gbm", market, as_of=as_of).value
        print(f"  vol={vol:0.2f} → {val}")

    print("\nCorrelation sensitivity (util‑credit ρ → PV):")
    for rho in [0.4, 0.6, 0.8, 0.9]:
        inst = build_revolver(implied_vol=0.25, util_credit_corr=rho, num_paths=3000)
        val = registry.price(inst, "monte_carlo_gbm", market, as_of=as_of).value
        print(f"  rho={rho:0.2f} → {val}")

    # Optionality analytics from Rust pricer (captured paths)
    print("\nRunning path analytics and punitive path tables (captured from pricer)...")
    run_optionality_analytics_from_pricer(base, market, as_of, recovery_rate=0.40)

    # ===== IRR SENSITIVITY ANALYSIS =====
    print("\n\n=== IRR SENSITIVITY ANALYSIS ===")
    
    # 1. Credit spread volatility sensitivity
    print("\n1. IRR vs Credit Spread Volatility:")
    vol_values = [0.05, 0.10, 0.15, 0.20, 0.25, 0.30]
    irr_vol_datasets = {}
    
    for vol in vol_values:
        print(f"   Running MC for implied_vol={vol:.2f}...")
        inst = build_revolver(implied_vol=vol, util_credit_corr=0.8, num_paths=500, seed=42)
        mc = inst.mc_paths(market, as_of=as_of, capture_mode="all", seed=42)
        if mc.has_paths():
            ds = mc.paths
            irrs = compute_irr_per_path(
                ds,
                inst,
                base_rate_annual=0.055,
                margin_bp=150.0,
                commitment_fee_bp=25.0,
                usage_fee_bp=50.0,
                facility_fee_bp=0.0,
                upfront_fee=0.0,
            )
            irr_vol_datasets[f"Vol={vol:.2f}"] = irrs
            valid = irrs[~np.isnan(irrs)]
            if len(valid) > 0:
                print(f"      Mean IRR: {valid.mean()*100:.2f}%, Median: {np.median(valid)*100:.2f}%, Valid: {len(valid)}/{len(irrs)}")
    
    if len(irr_vol_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_vol_datasets.items())[:3]), save_prefix="revolver_irr_vol")
        plot_irr_comparison(irr_vol_datasets, vol_values, "Credit Spread Implied Vol", save_prefix="revolver_irr_vol_sensitivity")
    
    # 2. Utilization volatility sensitivity (varying utilization process volatility)
    print("\n2. IRR vs Utilization Volatility:")
    util_vol_values = [0.10, 0.15, 0.20, 0.25, 0.30]
    irr_util_vol_datasets = {}
    
    for util_vol in util_vol_values:
        print(f"   Running MC for util_vol={util_vol:.2f}...")
        inst = RevolvingCredit.builder(
            instrument_id=f"REVOLVER_UVOL_{util_vol:.2f}",
            commitment_amount=Money(5_000_000, USD),
            drawn_amount=Money(1_500_000, USD),
            commitment_date=date(2025, 1, 1),
            maturity_date=date(2028, 1, 1),
            base_rate_spec={
                "type": "floating",
                "index_id": "USD.SOFR.1M",
                "margin_bp": 0.0,
                "reset_freq": "monthly",
            },
            payment_frequency="quarterly",
            fees={
                "commitment_fee_bp": 25.0,
                "usage_fee_bp": 150.0,
            },
            draw_repay_spec={
                "stochastic": {
                    "utilization_process": {
                        "type": "mean_reverting",
                        "target_rate": 0.20,
                        "speed": 0.50,
                        "volatility": util_vol,
                    },
                    "num_paths": 500,
                    "seed": 42,
                    "mc_config": {
                        "recovery_rate": 0.40,
                        "credit_spread_process": {
                            "market_anchored": {
                                "hazard_curve_id": "BORROWER-HZD",
                                "kappa": 0.50,
                                "implied_vol": 0.20,
                                "tenor_years": None,
                            }
                        },
                        "util_credit_corr": 0.8,
                    },
                }
            },
            discount_curve="USD.OIS",
        )
        mc = inst.mc_paths(market, as_of=as_of, capture_mode="all", seed=42)
        if mc.has_paths():
            ds = mc.paths
            irrs = compute_irr_per_path(
                ds,
                inst,
                base_rate_annual=0.055,
                margin_bp=150.0,
                commitment_fee_bp=25.0,
                usage_fee_bp=50.0,
                facility_fee_bp=0.0,
                upfront_fee=0.0,
            )
            irr_util_vol_datasets[f"UtilVol={util_vol:.2f}"] = irrs
            valid = irrs[~np.isnan(irrs)]
            if len(valid) > 0:
                print(f"      Mean IRR: {valid.mean()*100:.2f}%, Median: {np.median(valid)*100:.2f}%, Valid: {len(valid)}/{len(irrs)}")
    
    if len(irr_util_vol_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_util_vol_datasets.items())[:3]), save_prefix="revolver_irr_util_vol")
        plot_irr_comparison(irr_util_vol_datasets, util_vol_values, "Utilization Volatility", save_prefix="revolver_irr_util_vol_sensitivity")
    
    # 3. Correlation sensitivity
    print("\n3. IRR vs Util-Credit Correlation:")
    corr_values = [0.0, 0.3, 0.5, 0.7, 0.9]
    irr_corr_datasets = {}
    
    for rho in corr_values:
        print(f"   Running MC for corr={rho:.2f}...")
        inst = build_revolver(implied_vol=0.20, util_credit_corr=rho, num_paths=500, seed=42)
        mc = inst.mc_paths(market, as_of=as_of, capture_mode="all", seed=42)
        if mc.has_paths():
            ds = mc.paths
            irrs = compute_irr_per_path(
                ds,
                inst,
                base_rate_annual=0.055,
                margin_bp=150.0,
                commitment_fee_bp=25.0,
                usage_fee_bp=50.0,
                facility_fee_bp=0.0,
                upfront_fee=0.0,
            )
            irr_corr_datasets[f"Corr={rho:.2f}"] = irrs
            valid = irrs[~np.isnan(irrs)]
            if len(valid) > 0:
                print(f"      Mean IRR: {valid.mean()*100:.2f}%, Median: {np.median(valid)*100:.2f}%, Valid: {len(valid)}/{len(irrs)}")
    
    if len(irr_corr_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_corr_datasets.items())[:3]), save_prefix="revolver_irr_corr")
        plot_irr_comparison(irr_corr_datasets, corr_values, "Util-Credit Correlation", save_prefix="revolver_irr_corr_sensitivity")
    
    print("\n✅ IRR sensitivity analysis complete!")
    print("Generated charts:")
    print("  - revolver_irr_vol_distributions.png")
    print("  - revolver_irr_vol_sensitivity.png")
    print("  - revolver_irr_util_vol_distributions.png")
    print("  - revolver_irr_util_vol_sensitivity.png")
    print("  - revolver_irr_corr_distributions.png")
    print("  - revolver_irr_corr_sensitivity.png")


if __name__ == "__main__":
    main()


