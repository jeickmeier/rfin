"""Calibrate a USD OIS discount curve using `execute_calibration_v2`.

This script is a standalone equivalent of the notebook:
`finstack-py/examples/notebooks/valuations/18_valuations_calibration.ipynb`.

Run:
  uv run python finstack-py/examples/scripts/valuations/calibration/discount_curve_calibration_example.py
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import date

from finstack.core.market_data.context import MarketContext

from finstack.valuations import calibration as cal

# pyright: reportMissingImports=false, reportMissingModuleSource=false


@dataclass(frozen=True)
class BloombergZeroPoint:
    maturity: date
    zero_pct: float
    df: float


DEFAULT_OIS_INDEX_ID = "USD-FEDFUNDS-OIS"


def _maybe_get_calendar(code: str):
    # finstack exports dates as a flat module: finstack.core.dates.get_calendar(...)
    from finstack.core import dates as fdates

    return fdates.get_calendar(code)


def _add_payment_delay_business_days(d: date, delay_days: int, calendar_id: str | None) -> date:
    """Replicate `irs::dates::add_payment_delay` behavior (business days, fallback to weekdays)."""
    if delay_days <= 0:
        return d

    from finstack.core import dates as fdates

    if calendar_id:
        try:
            cal_ = _maybe_get_calendar(calendar_id)
            return fdates.add_business_days(d, delay_days, cal_)
        except Exception:
            # Match Rust fallback semantics (Mon-Fri, ignore holidays).
            return fdates.add_weekdays(d, delay_days)

    return fdates.add_weekdays(d, delay_days)


def _settlement_date(
    base_date: date,
    *,
    settlement_days: int,
    calendar_id: str,
    bdc: str,
) -> date:
    """Replicate `CalibrationPricer::settlement_date_for_quote` (happy path)."""
    from finstack.core import dates as fdates

    cal_ = _maybe_get_calendar(calendar_id)
    bdc_ = fdates.BusinessDayConvention.from_name(bdc)
    if settlement_days == 0:
        return fdates.adjust(base_date, bdc_, cal_)
    spot = fdates.add_business_days(base_date, settlement_days, cal_)
    return fdates.adjust(spot, bdc_, cal_)


def build_usd_ois_quotes(base_date: date, *, ois_index_id: str = DEFAULT_OIS_INDEX_ID) -> list[cal.RatesQuote]:
    """Build USD OIS deposit + swap quotes (as shown in the calibration notebook)."""
    quotes: list[cal.RatesQuote] = [
        cal.RatesQuote.deposit("DEP-1W", ois_index_id, date(2025, 12, 19), 0.0364447),
        cal.RatesQuote.deposit("DEP-2W", ois_index_id, date(2025, 12, 26), 0.0364455),
        cal.RatesQuote.deposit("DEP-3W", ois_index_id, date(2026, 1, 2), 0.0365300),
        cal.RatesQuote.deposit("DEP-1M", ois_index_id, date(2026, 1, 12), 0.0364950),
        cal.RatesQuote.deposit("DEP-2M", ois_index_id, date(2026, 2, 12), 0.0364050),
        cal.RatesQuote.deposit("DEP-3M", ois_index_id, date(2026, 3, 12), 0.0363477),
        cal.RatesQuote.deposit("DEP-4M", ois_index_id, date(2026, 4, 13), 0.0361400),
        cal.RatesQuote.deposit("DEP-5M", ois_index_id, date(2026, 5, 12), 0.0359544),
        cal.RatesQuote.deposit("DEP-6M", ois_index_id, date(2026, 6, 12), 0.0358000),
        cal.RatesQuote.deposit("DEP-7M", ois_index_id, date(2026, 7, 13), 0.0355310),
        cal.RatesQuote.deposit("DEP-8M", ois_index_id, date(2026, 8, 12), 0.0352500),
        cal.RatesQuote.deposit("DEP-9M", ois_index_id, date(2026, 9, 14), 0.0350225),
        cal.RatesQuote.deposit("DEP-10M", ois_index_id, date(2026, 10, 13), 0.0347742),
        cal.RatesQuote.deposit("DEP-11M", ois_index_id, date(2026, 11, 12), 0.0345356),
    ]

    swap_points = [
        ("SWAP-1Y", date(2026, 12, 14), 0.0343446),
        ("SWAP-18M", date(2027, 6, 14), 0.0332849),
        ("SWAP-2Y", date(2027, 12, 13), 0.0329864),
        ("SWAP-3Y", date(2028, 12, 12), 0.0330190),
        ("SWAP-4Y", date(2029, 12, 12), 0.0333823),
        ("SWAP-5Y", date(2030, 12, 12), 0.0338799),
        ("SWAP-6Y", date(2031, 12, 12), 0.0344608),
        ("SWAP-7Y", date(2032, 12, 13), 0.0350619),
        ("SWAP-8Y", date(2033, 12, 12), 0.0356592),
        ("SWAP-9Y", date(2034, 12, 12), 0.0362453),
        ("SWAP-10Y", date(2035, 12, 12), 0.0368206),
        ("SWAP-12Y", date(2037, 12, 14), 0.0378975),
        ("SWAP-15Y", date(2040, 12, 12), 0.0391717),
        ("SWAP-20Y", date(2045, 12, 12), 0.0402348),
        ("SWAP-25Y", date(2050, 12, 12), 0.0403809),
        ("SWAP-30Y", date(2055, 12, 13), 0.0401000),
        ("SWAP-40Y", date(2065, 12, 14), 0.0390413),
        ("SWAP-50Y", date(2075, 12, 12), 0.0378761),
    ]

    for swap_id, maturity, rate in swap_points:
        quotes.append(cal.RatesQuote.swap(swap_id, ois_index_id, maturity, rate))

    return quotes


def print_conventions_debug(
    base_date: date,
    *,
    curve_day_count: str | None,
    strict_step_pricing: bool,
    ois_index_id: str,
) -> None:
    """Print the simplified conventions used in the v2 quote API."""
    build_discount_step_conventions(
        curve_day_count=curve_day_count,
        strict_pricing=strict_step_pricing,
    )


def build_discount_step_conventions(
    *,
    curve_day_count: str | None,
    strict_pricing: bool,
) -> dict:
    """Build the JSON conventions payload for the v2 discount step."""

    def normalize_dc(dc: str) -> str:
        key = dc.strip().lower().replace("_", "").replace("-", "").replace(" ", "")
        # Accept common human labels and map to schema enum variants.
        aliases = {
            "act/360": "act360",
            "act360": "act360",
            "act/365f": "act365f",
            "act365f": "act365f",
            "act_365f": "act365f",
            "act_365_fixed": "act365f",
            "act/365l": "act365l",
            "act365l": "act365l",
            "actact": "act_act",
            "act/act": "act_act",
            "thirty360": "thirty360",
            "30/360": "thirty360",
            "30e/360": "thirty_e360",
            "thirtye360": "thirty_e360",
            "bus252": "bus252",
        }
        return aliases.get(key, dc)

    out: dict = {}
    if curve_day_count is not None:
        out["curve_day_count"] = normalize_dc(curve_day_count)

    # No additional fields are accepted in the v2 rates step conventions; the engine
    # applies instrument-level conventions internally.
    if strict_pricing:
        # Keep the flag visible in logs for callers but do not surface in payload.
        pass

    return out


def bloomberg_zero_points() -> Sequence[BloombergZeroPoint]:
    """Bloomberg zero table used in the notebook (rates in percent)."""
    raw: Sequence[tuple[date, float, float]] = [
        (date(2025, 12, 19), 3.69398, 0.999090),
        (date(2025, 12, 26), 3.69282, 0.998383),
        (date(2026, 1, 2), 3.69935, 0.997672),
        (date(2026, 1, 12), 3.69440, 0.996665),
        (date(2026, 2, 12), 3.68001, 0.993568),
        (date(2026, 3, 12), 3.66918, 0.990794),
        (date(2026, 4, 13), 3.64279, 0.987701),
        (date(2026, 5, 12), 3.61916, 0.984944),
        (date(2026, 6, 12), 3.59832, 0.982024),
        (date(2026, 7, 13), 3.56631, 0.979212),
        (date(2026, 8, 12), 3.53343, 0.976562),
        (date(2026, 9, 14), 3.50543, 0.973654),
        (date(2026, 10, 13), 3.47621, 0.971185),
        (date(2026, 11, 12), 3.44791, 0.968667),
        (date(2026, 12, 14), 3.42406, 0.965976),
        (date(2027, 6, 14), 3.32790, 0.951003),
        (date(2027, 12, 13), 3.28850, 0.936093),
        (date(2028, 12, 12), 3.29221, 0.905709),
        (date(2029, 12, 12), 3.32985, 0.875056),
        (date(2030, 12, 12), 3.38188, 0.844195),
        (date(2031, 12, 12), 3.44337, 0.813113),
        (date(2032, 12, 13), 3.50778, 0.781903),
        (date(2033, 12, 12), 3.57278, 0.751102),
        (date(2034, 12, 12), 3.63749, 0.720527),
        (date(2035, 12, 12), 3.70206, 0.690312),
        (date(2037, 12, 14), 3.82585, 0.631387),
        (date(2040, 12, 12), 3.97745, 0.550311),
        (date(2045, 12, 12), 4.10343, 0.439783),
        (date(2050, 12, 12), 4.10412, 0.358105),
        (date(2055, 12, 13), 4.03819, 0.297434),
        (date(2065, 12, 14), 3.81263, 0.217291),
        (date(2075, 12, 12), 3.55798, 0.168578),
    ]
    return [BloombergZeroPoint(maturity=m, zero_pct=z, df=df) for m, z, df in raw]


def top_residuals(report: object, limit: int = 10) -> list[tuple[str, float]]:
    """Extract the largest absolute residuals from a step report (best-effort)."""
    residuals = getattr(report, "residuals", None)
    if not isinstance(residuals, dict):
        return []
    pairs: list[tuple[str, float]] = []
    for k, v in residuals.items():
        try:
            pairs.append((str(k), float(v)))
        except (TypeError, ValueError):
            continue
    pairs.sort(key=lambda kv: abs(kv[1]), reverse=True)
    return pairs[:limit]


def print_bloomberg_comparison(
    base_date: date,
    curve: object,
    points: Iterable[BloombergZeroPoint],
) -> None:

    for pt in points:
        calc_df = float(curve.df_on_date(pt.maturity))
        calc_zero_pct = float(curve.zero_on_date(pt.maturity)) * 100.0
        (calc_zero_pct - pt.zero_pct) * 100.0
        calc_df - pt.df
        _ = base_date  # retained for symmetry with notebook, useful if you extend to tenor plots


def print_bbg_df_debug(
    *,
    base_date: date,
    curve: object,
    points: Sequence[BloombergZeroPoint],
    swap_payment_lag_days: int,
    swap_payment_calendar_id: str,
) -> None:
    """Debug DF mismatches by comparing maturity-date vs pillar-date discount factors."""
    try:
        curve_dc = curve.day_count
        curve.base_date()
    except Exception:
        curve_dc = None

    from finstack.core import dates as fdates

    dc_act360 = fdates.DayCount.ACT_360
    dc_act365f = fdates.DayCount.ACT_365F

    if curve_dc is not None:
        pass

    for pt in points:
        df_maturity = float(curve.df_on_date(pt.maturity))
        pillar = _add_payment_delay_business_days(
            pt.maturity,
            swap_payment_lag_days,
            swap_payment_calendar_id,
        )
        df_pillar = float(curve.df_on_date(pillar))
        abs(df_maturity - pt.df)
        abs(df_pillar - pt.df)

        dc_act365f.year_fraction(base_date, pt.maturity, None)
        dc_act360.year_fraction(base_date, pt.maturity, None)


def try_plot_zero_comparison(
    base_date: date,
    curve: object,
    points: Sequence[BloombergZeroPoint],
) -> None:
    try:
        import matplotlib.pyplot as plt  # type: ignore
    except Exception:  # pragma: no cover
        return

    tenors = []
    bbg_zeros = []
    calc_zeros = []
    bbg_dfs = []
    calc_dfs = []

    for pt in points:
        t = (pt.maturity - base_date).days / 365.0
        tenors.append(t)
        bbg_zeros.append(pt.zero_pct)
        calc_zeros.append(float(curve.zero_on_date(pt.maturity)) * 100.0)
        bbg_dfs.append(pt.df)
        calc_dfs.append(float(curve.df_on_date(pt.maturity)))

    _fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    ax1, ax2, ax3, ax4 = axes.flatten()

    ax1.plot(tenors, bbg_zeros, "o-", color="orange", label="Bloomberg", markersize=5, linewidth=1.5)
    ax1.plot(tenors, calc_zeros, "s--", color="blue", label="Finstack", markersize=4, linewidth=1.5)
    ax1.set_ylabel("Zero Rate (%)")
    ax1.set_title("Zero Rate Comparison")
    ax1.legend(loc="lower right")
    ax1.grid(True, alpha=0.3)
    ax1.set_xlim(0, max(tenors) + 1)

    ax2.plot(tenors, bbg_dfs, "o-", color="orange", label="Bloomberg", markersize=5, linewidth=1.5)
    ax2.plot(tenors, calc_dfs, "s--", color="blue", label="Finstack", markersize=4, linewidth=1.5)
    ax2.set_ylabel("Discount Factor")
    ax2.set_title("Discount Factor Comparison")
    ax2.legend(loc="upper right")
    ax2.grid(True, alpha=0.3)
    ax2.set_xlim(0, max(tenors) + 1)

    zero_diffs_bp = [(c - b) * 100.0 for c, b in zip(calc_zeros, bbg_zeros, strict=False)]
    ax3.bar(tenors, zero_diffs_bp, width=0.4, color="steelblue", alpha=0.7, edgecolor="black", linewidth=0.5)
    ax3.axhline(y=0, color="black", linestyle="-", linewidth=0.8)
    ax3.set_xlabel("Tenor (Years)")
    ax3.set_ylabel("Diff (bp)")
    ax3.set_title("Zero Rate Difference (Finstack - Bloomberg)")
    ax3.grid(True, alpha=0.3)
    ax3.set_xlim(0, max(tenors) + 1)

    df_diffs = [(c - b) * 10000.0 for c, b in zip(calc_dfs, bbg_dfs, strict=False)]
    ax4.bar(tenors, df_diffs, width=0.4, color="indianred", alpha=0.7, edgecolor="black", linewidth=0.5)
    ax4.axhline(y=0, color="black", linestyle="-", linewidth=0.8)
    ax4.set_xlabel("Tenor (Years)")
    ax4.set_ylabel("Diff (×10⁻⁴)")
    ax4.set_title("Discount Factor Difference (Finstack - Bloomberg)")
    ax4.grid(True, alpha=0.3)
    ax4.set_xlim(0, max(tenors) + 1)

    plt.suptitle(
        "USD OIS Discount Curve Calibration: Bloomberg vs Finstack Library",
        fontsize=14,
        fontweight="bold",
    )
    plt.tight_layout()
    plt.show()


def calibrate_discount_curve(
    base_date: date,
    *,
    use_global_solve: bool,
    use_analytical_jacobian: bool,
    tolerance: float,
    curve_day_count: str | None,
    strict_step_pricing: bool,
    ois_index_id: str,
) -> tuple[MarketContext, object, object]:
    quotes = build_usd_ois_quotes(base_date, ois_index_id=ois_index_id)

    method_json: object
    if use_global_solve:
        method_json = {"GlobalSolve": {"use_analytical_jacobian": use_analytical_jacobian}}
    else:
        method_json = "Bootstrap"

    quote_sets = {"ois": [q.to_market_quote() for q in quotes]}
    step_conventions = build_discount_step_conventions(
        curve_day_count=curve_day_count,
        strict_pricing=strict_step_pricing,
    )
    steps = [
        {
            "id": "disc",
            "quote_set": "ois",
            "kind": "discount",
            "curve_id": "USD-OIS",
            "currency": "USD",
            "base_date": str(base_date),
            "method": method_json,
            "conventions": step_conventions,
            "interpolation": "piecewise_quadratic_forward",
            "extrapolation": "flat_forward",
        }
    ]

    # `verbose=True` increases engine-side diagnostics. `explain=True` enables structured traces.
    settings = cal.CalibrationConfig(tolerance=tolerance, verbose=True, explain=True)
    market = MarketContext()
    market, plan_report, step_reports = cal.execute_calibration_v2(
        "example_discount_curve",
        quote_sets,
        steps,
        settings=settings,
    )
    return market, plan_report, step_reports["disc"]


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="USD OIS discount-curve calibration example (execute_calibration_v2).")
    parser.add_argument("--global-solve", action="store_true", help="Use GlobalSolve (otherwise Bootstrap).")
    parser.add_argument(
        "--no-analytical-jacobian",
        action="store_true",
        help="For GlobalSolve only: disable analytical Jacobian.",
    )
    parser.add_argument("--tolerance", type=float, default=1e-8, help="Calibration tolerance (default: 1e-8).")
    parser.add_argument(
        "--curve-day-count",
        type=str,
        default=None,
        help="Override curve day-count for date->time mapping (e.g. 'ACT/360', 'act360', 'ACT/365F', 'act365f').",
    )
    parser.add_argument(
        "--strict-step-pricing",
        action="store_true",
        help="Enable strict pricing mode at the step level (requires explicit step defaults).",
    )
    parser.add_argument(
        "--ois-index",
        type=str,
        default=DEFAULT_OIS_INDEX_ID,
        help="Overnight index id for OIS float leg (e.g. 'USD-FEDFUNDS-OIS', 'USD-EFFR-OIS', 'USD-SOFR-OIS').",
    )
    parser.add_argument(
        "--debug-conventions",
        action="store_true",
        help="Print the quote and step-level conventions used for instrument construction.",
    )
    parser.add_argument(
        "--compare-bloomberg",
        action="store_true",
        help="Print zero-rate / DF comparison against Bloomberg table from the notebook.",
    )
    parser.add_argument(
        "--debug-bbg-df",
        action="store_true",
        help="Extra diagnostics for BBG DF diffs: compare DF at maturity vs pillar date (maturity+payment_delay).",
    )
    parser.add_argument(
        "--plot",
        action="store_true",
        help="Plot Bloomberg vs calibrated curve (requires matplotlib). Implies --compare-bloomberg.",
    )
    args = parser.parse_args()

    # Always define base_date - Bloomberg settle date (per notebook).
    base_date = date(2025, 12, 10)

    if args.debug_conventions:
        print_conventions_debug(
            base_date,
            curve_day_count=args.curve_day_count,
            strict_step_pricing=bool(args.strict_step_pricing),
            ois_index_id=str(args.ois_index),
        )

    use_analytical_jacobian = not args.no_analytical_jacobian
    market, _plan_report, disc_report = calibrate_discount_curve(
        base_date,
        use_global_solve=bool(args.global_solve),
        use_analytical_jacobian=use_analytical_jacobian,
        tolerance=float(args.tolerance),
        curve_day_count=args.curve_day_count,
        strict_step_pricing=bool(args.strict_step_pricing),
        ois_index_id=str(args.ois_index),
    )

    curve = market.discount("USD-OIS")

    residuals = top_residuals(disc_report, limit=10)
    if residuals:
        for _k, _v in residuals:
            pass

    if args.compare_bloomberg or args.plot:
        points = list(bloomberg_zero_points())
        print_bloomberg_comparison(base_date, curve, points)

    if args.debug_bbg_df:
        print_bbg_df_debug(
            base_date=base_date,
            curve=curve,
            points=list(bloomberg_zero_points()),
            swap_payment_lag_days=2,
            swap_payment_calendar_id="usny",
        )

    if args.plot:
        try_plot_zero_comparison(base_date, curve, list(bloomberg_zero_points()))


if __name__ == "__main__":
    main()
