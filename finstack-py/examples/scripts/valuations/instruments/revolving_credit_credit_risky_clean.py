"""
Credit-Risky Revolving Credit Example (Clean, DRY version)

Demonstrates pricing a revolving credit facility with stochastic utilization
and market-anchored credit risk using centralized configuration and
DRY scenario sweeps.

This version features:
- Centralized configuration in immutable dataclasses
- No parameter duplication
- Uses day-count helpers from core/dates module
- Compact scenario sweep utilities
- Simplified plotting signatures
- PROPER cashflow extraction from monte carlo simulation (not manual reconstruction!)

IRR calculations use cashflows directly from RevolvingCreditPayoff via
PathPoint.cashflows and SimulatedPath.get_cashflows_with_dates() methods.
"""

import argparse
from dataclasses import dataclass, replace
from datetime import date, timedelta
from typing import Optional, Dict, List, Tuple, Callable, TypeVar

import numpy as np
import matplotlib.pyplot as plt

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve, ForwardCurve
from finstack.core.cashflow import xirr
from finstack.core.dates.daycount import DayCount
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.pricer import create_standard_registry

# ============================================================================
# CONFIGURATION (single source of truth)
# ============================================================================

# Market data IDs
USD_DISC_ID = "USD.OIS"
SOFR_1M_ID = "USD.SOFR.1M"
BORROWER_HZD_ID = "BORROWER-HZD"

# Instrument parameters
COMMITMENT_AMOUNT = 5_000_000
DRAWN_AMOUNT = 1_500_000
COMMITMENT_DATE = date(2025, 1, 1)
MATURITY_DATE = date(2035, 1, 1)


@dataclass(frozen=True)
class FeeConfig:
    """Fee structure for revolving credit facility."""
    commitment_fee_bp: float = 25.0
    usage_fee_bp: float = 150.0  # Used in build_revolver_from
    facility_fee_bp: float = 0.0
    upfront_fee: float = 0.0


@dataclass(frozen=True)
class RateConfig:
    """Base rate and margin configuration."""
    margin_bp: float = 0.0
    # For IRR calculations when using fixed rate
    base_rate_annual: float = 0.055


@dataclass(frozen=True)
class UtilProcess:
    """Utilization process parameters (mean-reverting)."""
    target_rate: float = 0.25
    speed: float = 0.50
    volatility: float = 0.20


@dataclass(frozen=True)
class CreditAnchored:
    """Market-anchored credit spread process parameters."""
    hazard_curve_id: str = BORROWER_HZD_ID
    kappa: float = 0.50
    implied_vol: float = 0.25
    tenor_years: Optional[float] = None


@dataclass(frozen=True)
class McKnobs:
    """Monte Carlo simulation configuration."""
    recovery_rate: float = 0.40
    util_credit_corr: float = 0.80
    num_paths: int = 25000
    seed: int = 42
    util: UtilProcess = UtilProcess()
    credit: CreditAnchored = CreditAnchored()


# Default configurations
FEE_DEFAULT = FeeConfig()
RATE_DEFAULT = RateConfig()
MC_DEFAULT = McKnobs()

# ============================================================================
# MARKET DATA
# ============================================================================


def build_market(as_of: date) -> MarketContext:
    """Create minimal market inputs: discount curve + borrower hazard curve + forward curve."""
    disc = DiscountCurve(
        USD_DISC_ID,
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.97),
            (3.0, 0.91),
        ],
    )

    # Constant hazard ~5% with 40% recovery
    hazard = HazardCurve(
        BORROWER_HZD_ID,
        as_of,
        [
            (1.0, 0.05),
            (5.0, 0.05),
        ],
        recovery_rate=0.40,
    )

    # SOFR 1M forward curve (tenor = 1/12 years)
    sofr_1m = ForwardCurve(
        SOFR_1M_ID,
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


# ============================================================================
# INSTRUMENT BUILDER
# ============================================================================


def build_revolver_from(
    mc: McKnobs,
    fees: FeeConfig = FEE_DEFAULT,
    rate: RateConfig = RATE_DEFAULT,
) -> RevolvingCredit:
    """Create a credit-risky revolver from Monte Carlo configuration.
    
    Only the MC parameters vary across scenarios; all other instrument
    parameters are centralized above.
    """
    return RevolvingCredit.builder(
        instrument_id=f"REVOLVER_CR_{mc.credit.implied_vol:.2f}",
        commitment_amount=Money(COMMITMENT_AMOUNT, USD),
        drawn_amount=Money(DRAWN_AMOUNT, USD),
        commitment_date=COMMITMENT_DATE,
        maturity_date=MATURITY_DATE,
        base_rate_spec={
            "type": "floating",
            "index_id": SOFR_1M_ID,
            "margin_bp": rate.margin_bp,
            "reset_freq": "monthly",
        },
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": fees.commitment_fee_bp,
            "usage_fee_bp": fees.usage_fee_bp,
            "facility_fee_bp": fees.facility_fee_bp,
            "upfront_fee": fees.upfront_fee,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": mc.util.target_rate,
                    "speed": mc.util.speed,
                    "volatility": mc.util.volatility,
                },
                "num_paths": mc.num_paths,
                "seed": mc.seed,
                "mc_config": {
                    "recovery_rate": mc.recovery_rate,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": mc.credit.hazard_curve_id,
                            "kappa": mc.credit.kappa,
                            "implied_vol": mc.credit.implied_vol,
                            "tenor_years": mc.credit.tenor_years,
                        }
                    },
                    "util_credit_corr": mc.util_credit_corr,
                },
            }
        },
        discount_curve=USD_DISC_ID,
    )


# ============================================================================
# DATE HANDLING (using core day count functionality)
# ============================================================================

# Default day count convention for the facility
DEFAULT_DAYCOUNT = DayCount.ACT_360


def year_frac_to_date(base_date: date, t: float) -> date:
    """Convert year fraction to calendar date.
    
    Note: This is an approximation. Ideally we'd have the inverse of
    year_fraction in the daycount module.
    """
    # Approximation using 365 days per year
    return base_date + timedelta(days=int(round(t * 365.0)))


# ============================================================================
# CASHFLOW EXTRACTION (Implemented!)
# ============================================================================
# 
# Cashflows are now properly extracted from monte carlo simulations using:
# - PathPoint.cashflows: Cashflows at each timestep
# - SimulatedPath.extract_cashflows(): All cashflows from a path
# - SimulatedPath.get_cashflows_with_dates(base_date): Cashflows with calendar dates
#
# See compute_irr_per_path_from_cashflows() for usage.
#
# ============================================================================
# PATH EXTRACTION & ANALYTICS
# ============================================================================


def extract_arrays_from_dataset(
    ds, recovery_rate: float
) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
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
    daycount: DayCount = DEFAULT_DAYCOUNT,
) -> np.ndarray:
    """Compute lender economic NPV per path by adding capital deployment flows.

    NPV_economic = EnginePV + PV(sum of principal draw/repay flows), where
    principal CF at step k is -(P_k - P_{k-1}) and at step 0 is -P_0.
    """
    disc = market.discount(USD_DISC_ID)
    # Use the DayCount from core/dates for year fraction calculation
    t_start = daycount.year_fraction(disc.base_date, inst.commitment_date, None)

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


def compute_irr_per_path_from_cashflows(
    ds,
    inst: RevolvingCredit,
) -> np.ndarray:
    """Calculate IRR per path using cashflows extracted from monte carlo simulation.
    
    This is the PROPER implementation that extracts cashflows directly from
    the Rust RevolvingCreditPayoff, avoiding manual reconstruction.
    
    The cashflows are captured during monte carlo simulation and include:
    - Interest on drawn amounts
    - Commitment fees on undrawn amounts
    - Usage fees on drawn amounts
    - Facility fees on total commitment
    - Recovery cashflows (if default occurs)
    """
    paths = ds.paths
    if len(paths) == 0:
        return np.array([])

    base_date = inst.commitment_date
    irrs = []

    for path in paths:
        # Extract cashflows directly from the path!
        # This uses the cashflows that were recorded by RevolvingCreditPayoff
        cashflow_list = path.get_cashflows_with_dates(base_date)
        
        if len(cashflow_list) < 2:
            irrs.append(np.nan)
            continue

        # Check sign changes
        cfs = [cf[1] for cf in cashflow_list]
        signs = [np.sign(cf) for cf in cfs if abs(cf) > 1e-6]
        if len(set(signs)) < 2:
            irrs.append(np.nan)
            continue

        try:
            irr_val = xirr(cashflow_list, guess=0.10)
            irrs.append(irr_val)
        except Exception:
            irrs.append(np.nan)

    return np.array(irrs)


def compute_irr_per_path(
    ds,
    inst: RevolvingCredit,
    fees: FeeConfig = FEE_DEFAULT,
    rate: RateConfig = RATE_DEFAULT,
) -> np.ndarray:
    """Calculate IRR per path using xirr with explicitly reconstructed undiscounted cashflows.
    
    DEPRECATED: This function manually reconstructs cashflows from monte carlo path data.
    Use compute_irr_per_path_from_cashflows() instead which extracts cashflows directly
    from the Rust implementation.
    
    Kept for backwards compatibility and validation purposes.
    """
    paths = ds.paths
    if len(paths) == 0:
        return np.array([])

    base_date = inst.commitment_date
    commitment = float(inst.commitment_amount.amount)
    
    # Extract fee and rate configuration from the instrument
    # Note: We're using the passed configs which should match the instrument's
    base_rate = float(rate.base_rate_annual)
    margin_rate = rate.margin_bp * 1e-4
    cfee_rate = fees.commitment_fee_bp * 1e-4
    ufee_rate = fees.usage_fee_bp * 1e-4
    ffee_rate = fees.facility_fee_bp * 1e-4

    irrs = []

    for path in paths:
        cashflow_list: List[Tuple[date, float]] = []
        prev_util = None
        prev_time = None

        # Upfront fee at start if provided
        if abs(fees.upfront_fee) > 0.0:
            cashflow_list.append((base_date, float(fees.upfront_fee)))

        for pt in path.points:
            t_years = float(pt.time)
            # Extract state variables from monte carlo path point
            # Note: PathPoint only contains state variables, not cashflows
            # This is why we must reconstruct cashflows manually
            util = pt.get_var("spot") or 0.0  # Utilization stored as 'spot'
            P = util * commitment
            credit_spread = pt.get_var("credit_spread") or 0.0  # Stochastic credit component

            # Convert year fraction to date
            cf_date = year_frac_to_date(base_date, t_years)

            # Reconstruct capital flow (principal draw/repayment)
            if prev_util is None:
                cf_cap = -P  # Initial deployment
            else:
                prev_P = prev_util * commitment
                cf_cap = -(P - prev_P)  # Change in principal

            # Reconstruct interest and fee cashflows
            # This mirrors the logic in RevolvingCreditPayoff::compute_cashflow
            receipts = 0.0
            if prev_time is not None:
                dt = max(t_years - prev_time, 0.0)
                # Total rate includes base rate, margin, and stochastic credit spread
                total_rate = base_rate + margin_rate + credit_spread
                drawn = P
                undrawn = max(commitment - drawn, 0.0)
                
                # Cashflows that should ideally come from the instrument's cashflow builder
                interest = drawn * total_rate * dt
                cfee = undrawn * cfee_rate * dt  # Commitment fee on undrawn
                ufee = drawn * ufee_rate * dt    # Usage fee on drawn
                ffee = commitment * ffee_rate * dt  # Facility fee on total
                receipts = interest + cfee + ufee + ffee

            # Net cashflow at this timestep
            cf_total = cf_cap + receipts

            if abs(cf_total) > 1e-6:
                cashflow_list.append((cf_date, cf_total))

            prev_util = util
            prev_time = t_years

        # Final principal return
        if prev_util is not None and prev_util > 1e-9:
            final_P = prev_util * commitment
            final_date = year_frac_to_date(base_date, float(path.points[-1].time))
            if len(cashflow_list) > 0 and cashflow_list[-1][0] == final_date:
                cashflow_list[-1] = (final_date, cashflow_list[-1][1] + final_P)
            else:
                cashflow_list.append((final_date, final_P))

        # Calculate XIRR
        if len(cashflow_list) < 2:
            irrs.append(np.nan)
            continue

        # Check sign changes
        cfs = [cf[1] for cf in cashflow_list]
        signs = [np.sign(cf) for cf in cfs if abs(cf) > 1e-6]
        if len(set(signs)) < 2:
            irrs.append(np.nan)
            continue

        try:
            irr_val = xirr(cashflow_list, guess=0.10)
            irrs.append(irr_val)
        except Exception:
            irrs.append(np.nan)

    return np.array(irrs)


def print_punitive_path_tables_from_dataset(
    ds,
    market: MarketContext,
    inst: RevolvingCredit,
    as_of: date,
    top_k: int = 3,
    npv_economic: Optional[np.ndarray] = None,
    daycount: DayCount = DEFAULT_DAYCOUNT,
) -> None:
    """Print tables for punitive paths with engine PV and capital-adjusted NPV columns."""
    paths = ds.paths
    if len(paths) == 0:
        print("No captured paths.")
        return
    
    pvs = np.array([p.final_value for p in paths])
    idx_sorted = np.argsort(pvs)

    # Setup for capital-adjusted flows
    disc = market.discount(USD_DISC_ID)
    # Use core day count functionality
    t_start = daycount.year_fraction(disc.base_date, inst.commitment_date, None)
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
            util = pt.get_var("spot") or 0.0
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


# ============================================================================
# PLOTTING
# ============================================================================


def plot_path_analytics(
    *,
    inst: RevolvingCredit,
    util_paths: np.ndarray,
    hazard_paths: np.ndarray,
    pvs: List[float],
    pvs_npv: Optional[List[float]] = None,
    save_prefix: str = "revolver_credit_risky",
) -> None:
    """Create path graphs and PV analytics to illustrate optionality impacts."""
    num_paths, num_steps = util_paths.shape
    months = np.arange(num_steps)

    # Figure 1: Sample utilization and hazard paths
    cols = 2 #if pvs_npv is not None else 2
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

    # PV vs average utilization
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
    """Plot IRR distributions for comparison across different scenarios."""
    n_scenarios = len(irr_datasets)
    fig, axes = plt.subplots(1, min(n_scenarios, 3), figsize=(18, 5))
    if n_scenarios == 1:
        axes = [axes]
    
    colors = plt.cm.viridis(np.linspace(0, 0.9, n_scenarios))
    
    for idx, (label, irrs) in enumerate(list(irr_datasets.items())[:3]):
        ax = axes[idx]
        valid_irrs = irrs[~np.isnan(irrs)] * 100
        
        ax.hist(valid_irrs, bins=40, color=colors[idx], alpha=0.7, edgecolor="black")
        ax.axvline(np.mean(valid_irrs), color="red", linestyle="--", linewidth=2, 
                   label=f"Mean: {np.mean(valid_irrs):.1f}%")
        ax.axvline(np.median(valid_irrs), color="orange", linestyle=":", linewidth=2, 
                   label=f"Median: {np.median(valid_irrs):.1f}%")
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
    """Plot IRR statistics vs parameter values for sensitivity analysis."""
    fig, axes = plt.subplots(1, 2, figsize=(16, 6))
    
    labels = list(irr_datasets.keys())
    means = []
    medians = []
    p5s = []
    p95s = []
    stds = []
    
    for label in labels:
        irrs = irr_datasets[label]
        valid_irrs = irrs[~np.isnan(irrs)] * 100
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
    
    # Plot 2: Std dev vs parameter
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


# ============================================================================
# SCENARIO SWEEP UTILITIES (DRY)
# ============================================================================

T = TypeVar('T')


def sweep(
    market: MarketContext,
    as_of: date,
    label_fmt: str,
    values: List[T],
    build_knobs: Callable[[T], McKnobs],
    fees: FeeConfig = FEE_DEFAULT,
    rate: RateConfig = RATE_DEFAULT,
) -> Dict[str, np.ndarray]:
    """Generic scenario sweep helper.
    
    For each value in values:
    1. Build McKnobs via build_knobs(value)
    2. Create instrument
    3. Run MC with capture_mode="all"
    4. Compute IRRs
    5. Store in dict with formatted label
    
    Returns dict mapping label -> IRR array.
    """
    out = {}
    
    for v in values:
        mc_knobs = build_knobs(v)
        inst = build_revolver_from(mc_knobs)
        mc = inst.mc_paths(market, as_of=as_of, capture_mode="all", seed=mc_knobs.seed)
        
        if mc.has_paths():
            ds = mc.paths
            # Use the new cashflow extraction method
            irrs = compute_irr_per_path_from_cashflows(ds, inst)
            label = label_fmt.format(v)
            out[label] = irrs
            
            valid = irrs[~np.isnan(irrs)]
            if len(valid) > 0:
                print(f"   {label}: Mean IRR={valid.mean()*100:.2f}%, "
                      f"Median={np.median(valid)*100:.2f}%, "
                      f"Valid={len(valid)}/{len(irrs)}")
    
    return out


# ============================================================================
# MAIN ANALYTICS
# ============================================================================


def run_optionality_analytics_from_pricer(
    inst: RevolvingCredit,
    market: MarketContext,
    as_of: date,
    recovery_rate: float,
    fees: FeeConfig = FEE_DEFAULT,
    rate: RateConfig = RATE_DEFAULT,
) -> None:
    """Fetch captured paths from Rust pricer, print punitive tables, and plot charts."""
    mc = inst.mc_paths(market, as_of=as_of, capture_mode="sample", sample_count=200, seed=42)
    if not mc.has_paths():
        print("Pricer returned no captured paths.")
        return
    
    ds = mc.paths
    times, util_paths, hazard_paths, pvs = extract_arrays_from_dataset(ds, recovery_rate)
    npv_econ = compute_capital_adjusted_npv(inst, market, as_of, times, util_paths, pvs)

    # Calculate IRRs using proper cashflow extraction
    irrs = compute_irr_per_path_from_cashflows(ds, inst)
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

    # Punitive paths
    print_punitive_path_tables_from_dataset(
        ds, market, inst, as_of, top_k=3, npv_economic=npv_econ
    )

    # Plots
    plot_path_analytics(
        inst=inst,
        util_paths=util_paths,
        hazard_paths=hazard_paths,
        pvs=pvs.tolist(),
        pvs_npv=npv_econ.tolist(),
        save_prefix="revolving_credit_credit_risky",
    )


def run_irr_sensitivity_analysis(
    market: MarketContext,
    as_of: date,
    num_paths: int = 500,
    seed: int = 42,
) -> None:
    """Run comprehensive IRR sensitivity analysis across key parameters."""
    print("\n\n=== IRR SENSITIVITY ANALYSIS ===")
    
    # 1. Credit spread volatility sensitivity
    print("\n1. IRR vs Credit Spread Volatility:")
    vol_values = [0.05, 0.10, 0.15, 0.20, 0.25, 0.30]
    irr_vol_datasets = sweep(
        market, as_of,
        label_fmt="Vol={:.2f}",
        values=vol_values,
        build_knobs=lambda vol: replace(
            MC_DEFAULT,
            num_paths=num_paths,
            seed=seed,
            credit=replace(MC_DEFAULT.credit, implied_vol=vol),
        ),
    )
    
    if len(irr_vol_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_vol_datasets.items())[:3]), save_prefix="revolver_irr_vol")
        plot_irr_comparison(irr_vol_datasets, vol_values, "Credit Spread Implied Vol", 
                          save_prefix="revolver_irr_vol_sensitivity")
    
    # 2. Utilization volatility sensitivity
    print("\n2. IRR vs Utilization Volatility:")
    util_vol_values = [0.10, 0.15, 0.20, 0.25, 0.30]
    irr_util_vol_datasets = sweep(
        market, as_of,
        label_fmt="UtilVol={:.2f}",
        values=util_vol_values,
        build_knobs=lambda uvol: replace(
            MC_DEFAULT,
            num_paths=num_paths,
            seed=seed,
            util=replace(MC_DEFAULT.util, target_rate=0.20, volatility=uvol),
            credit=replace(MC_DEFAULT.credit, implied_vol=0.20),
        ),
    )
    
    if len(irr_util_vol_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_util_vol_datasets.items())[:3]), 
                              save_prefix="revolver_irr_util_vol")
        plot_irr_comparison(irr_util_vol_datasets, util_vol_values, "Utilization Volatility", 
                          save_prefix="revolver_irr_util_vol_sensitivity")
    
    # 3. Correlation sensitivity
    # Tests range from negative (credit improves when utilization increases)
    # to positive (credit worsens when utilization increases - typical/adverse)
    print("\n3. IRR vs Util-Credit Correlation:")
    corr_values = [-0.9, -0.5, 0.0, 0.3, 0.5, 0.7, 0.8, 0.9]
    irr_corr_datasets = sweep(
        market, as_of,
        label_fmt="Corr={:+.2f}",
        values=corr_values,
        build_knobs=lambda rho: replace(
            MC_DEFAULT,
            num_paths=num_paths,
            seed=seed,
            util_credit_corr=rho,
            # Keep implied_vol at default 0.25 to isolate correlation effect
        ),
    )
    
    if len(irr_corr_datasets) >= 2:
        plot_irr_distributions(dict(list(irr_corr_datasets.items())[:3]), 
                              save_prefix="revolver_irr_corr")
        plot_irr_comparison(irr_corr_datasets, corr_values, "Util-Credit Correlation", 
                          save_prefix="revolver_irr_corr_sensitivity")
    
    print("\nIRR sensitivity analysis complete!")
    print("Generated charts:")
    print("  - revolver_irr_vol_distributions.png")
    print("  - revolver_irr_vol_sensitivity.png")
    print("  - revolver_irr_util_vol_distributions.png")
    print("  - revolver_irr_util_vol_sensitivity.png")
    print("  - revolver_irr_corr_distributions.png")
    print("  - revolver_irr_corr_sensitivity.png")


# ============================================================================
# MAIN
# ============================================================================


def main():
    parser = argparse.ArgumentParser(
        description="Credit-risky revolving credit facility pricing and analytics (clean version)"
    )
    parser.add_argument("--paths", type=int, default=3000, help="Number of MC paths for pricing")
    parser.add_argument("--sweep-paths", type=int, default=500, help="Number of MC paths for sweeps")
    parser.add_argument("--seed", type=int, default=42, help="Random seed")
    parser.add_argument("--skip-plots", action="store_true", help="Skip generating plots")
    parser.add_argument("--skip-sweeps", action="store_true", help="Skip IRR sensitivity sweeps")
    
    args = parser.parse_args()
    
    as_of = COMMITMENT_DATE
    market = build_market(as_of)
    registry = create_standard_registry()

    print("\n=== CREDIT-RISKY REVOLVER (Market-Anchored Credit) ===")
    
    # Base pricing
    base_mc = replace(MC_DEFAULT, num_paths=args.paths, seed=args.seed)
    base = build_revolver_from(base_mc)
    pv = registry.price(base, "monte_carlo_gbm", market, as_of=as_of).value
    print(f"Base PV (implied vol 0.25): {pv}")

    # Volatility sensitivity
    print("\nVolatility sensitivity (CDS option vol -> PV):")
    for vol in [0.0001, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.5]:
        mc = replace(base_mc, credit=replace(base_mc.credit, implied_vol=vol))
        inst = build_revolver_from(mc)
        val = registry.price(inst, "monte_carlo_gbm", market, as_of=as_of).value
        print(f"  vol={vol:0.2f} -> {val}")

    # Correlation sensitivity
    print("\nCorrelation sensitivity (util-credit rho -> PV):")
    print("  (Expect PV to decrease as correlation increases from negative to positive)")
    for rho in [-0.9, -0.5, 0.0, 0.3, 0.5, 0.7, 0.8, 0.9]:
        mc = replace(base_mc, util_credit_corr=rho)
        inst = build_revolver_from(mc)
        val = registry.price(inst, "monte_carlo_gbm", market, as_of=as_of).value
        marker = " <- BASE CASE" if abs(rho - 0.8) < 0.01 else ""
        print(f"  rho={rho:+0.2f} -> {val}{marker}")

    # Optionality analytics from Rust pricer (captured paths)
    if not args.skip_plots:
        print("\nRunning path analytics and punitive path tables (captured from pricer)...")
        run_optionality_analytics_from_pricer(
            base, market, as_of, recovery_rate=0.40,
            fees=FEE_DEFAULT, rate=RATE_DEFAULT
        )

    # IRR sensitivity analysis
    if not args.skip_sweeps:
        run_irr_sensitivity_analysis(market, as_of, num_paths=args.sweep_paths, seed=args.seed)


if __name__ == "__main__":
    main()

