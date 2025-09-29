#!/usr/bin/env python3
"""Demonstrate full-market calibration including forward and volatility surfaces."""
from __future__ import annotations

from datetime import date, timedelta
from typing import Dict, List, Sequence, Tuple, Optional

from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import MarketContext
from finstack.core.market_data.surfaces import VolSurface as MarketVolSurface
from finstack.valuations import calibration as cal

RatesPoint = Tuple[str, Dict[str, object]]
SwaptionPoint = Tuple[date, date, float]


def build_market_quotes(
    base_date: date,
) -> Tuple[List[cal.MarketQuote], Dict[str, List[cal.RatesQuote]], Sequence[SwaptionPoint]]:
    """Create a representative quote set spanning rates, credit, inflation, and vols."""

    # Discount curve anchors (SOFR OIS swaps)
    discount_quotes = [
        cal.RatesQuote.deposit(base_date + timedelta(days=30), 0.0450, "ACT/360"),
        cal.RatesQuote.deposit(base_date + timedelta(days=90), 0.0465, "ACT/360"),
        cal.RatesQuote.swap(
            base_date + timedelta(days=365),
            0.0475,
            Frequency.ANNUAL,
            Frequency.QUARTERLY,
            "30/360",
            "ACT/360",
            "USD-SOFR",
        ),
        cal.RatesQuote.swap(
            base_date + timedelta(days=365 * 3),
            0.0485,
            Frequency.ANNUAL,
            Frequency.QUARTERLY,
            "30/360",
            "ACT/360",
            "USD-SOFR",
        ),
    ]

    # Quotes supporting 3M forward curve calibration
    forward_3m_raw: List[RatesPoint] = [
        (
            "fra",
            {
                "start": base_date + timedelta(days=30),
                "end": base_date + timedelta(days=120),
                "rate": 0.0470,
                "day_count": "ACT/360",
            },
        ),
        (
            "fra",
            {
                "start": base_date + timedelta(days=90),
                "end": base_date + timedelta(days=180),
                "rate": 0.0480,
                "day_count": "ACT/360",
            },
        ),
        (
            "swap",
            {
                "maturity": base_date + timedelta(days=365 * 2),
                "rate": 0.0495,
                "fixed_freq": Frequency.ANNUAL,
                "float_freq": Frequency.QUARTERLY,
                "fixed_dc": "30/360",
                "float_dc": "ACT/360",
                "index": "USD-SOFR-3M",
            },
        ),
    ]

    forward_6m_raw: List[RatesPoint] = [
        (
            "swap",
            {
                "maturity": base_date + timedelta(days=365 * 4),
                "rate": 0.0505,
                "fixed_freq": Frequency.ANNUAL,
                "float_freq": Frequency.SEMI_ANNUAL,
                "fixed_dc": "30/360",
                "float_dc": "ACT/360",
                "index": "USD-SOFR-6M",
            },
        ),
        (
            "basis",
            {
                "maturity": base_date + timedelta(days=365 * 5),
                "primary_index": "USD-SOFR-3M",
                "reference_index": "USD-SOFR-6M",
                "spread_bp": 6.0,
                "primary_freq": Frequency.QUARTERLY,
                "reference_freq": Frequency.SEMI_ANNUAL,
                "primary_dc": "ACT/360",
                "reference_dc": "ACT/360",
                "currency": "USD",
            },
        ),
    ]

    def expand_rates(points: List[RatesPoint]) -> List[cal.RatesQuote]:
        expanded: List[cal.RatesQuote] = []
        for kind, info in points:
            if kind == "fra":
                expanded.append(
                    cal.RatesQuote.fra(info["start"], info["end"], info["rate"], info["day_count"])
                )
            elif kind == "swap":
                expanded.append(
                    cal.RatesQuote.swap(
                        info["maturity"],
                        info["rate"],
                        info["fixed_freq"],
                        info["float_freq"],
                        info["fixed_dc"],
                        info["float_dc"],
                        info["index"],
                    )
                )
            elif kind == "basis":
                expanded.append(
                    cal.RatesQuote.basis_swap(
                        info["maturity"],
                        info["primary_index"],
                        info["reference_index"],
                        info["spread_bp"],
                        info["primary_freq"],
                        info["reference_freq"],
                        info["primary_dc"],
                        info["reference_dc"],
                        info["currency"],
                    )
                )
        return expanded

    forward_3m_quotes = expand_rates(forward_3m_raw)
    forward_6m_quotes = expand_rates(forward_6m_raw)

    # Credit quotes
    credit_quotes = [
        cal.CreditQuote.cds("ACME", base_date + timedelta(days=365 * 3), 110.0, 0.40, "USD"),
        cal.CreditQuote.cds("ACME", base_date + timedelta(days=365 * 5), 125.0, 0.40, "USD"),
    ]

    # Inflation quotes
    inflation_quotes = [
        cal.InflationQuote.inflation_swap(base_date + timedelta(days=365 * 3), 0.021, "US-CPI-U"),
        cal.InflationQuote.inflation_swap(base_date + timedelta(days=365 * 6), 0.023, "US-CPI-U"),
    ]

    # Equity option vol quotes
    vol_expiry_6m = base_date + timedelta(days=180)
    vol_expiry_1y = base_date + timedelta(days=365)
    equity_vol_quotes = [
        cal.VolQuote.option_vol("ACME", vol_expiry_6m, 90.0, 0.24, "Call"),
        cal.VolQuote.option_vol("ACME", vol_expiry_6m, 100.0, 0.22, "Call"),
        cal.VolQuote.option_vol("ACME", vol_expiry_6m, 110.0, 0.23, "Call"),
        cal.VolQuote.option_vol("ACME", vol_expiry_1y, 90.0, 0.26, "Call"),
        cal.VolQuote.option_vol("ACME", vol_expiry_1y, 100.0, 0.24, "Call"),
        cal.VolQuote.option_vol("ACME", vol_expiry_1y, 110.0, 0.25, "Call"),
    ]

    swaption_specs: List[SwaptionPoint] = [
        (base_date + timedelta(days=365), base_date + timedelta(days=365 * 6), 0.24),
        (base_date + timedelta(days=365), base_date + timedelta(days=365 * 8), 0.242),
        (base_date + timedelta(days=365 * 2), base_date + timedelta(days=365 * 6), 0.235),
        (base_date + timedelta(days=365 * 2), base_date + timedelta(days=365 * 8), 0.231),
    ]
    swaption_quotes = [
        cal.VolQuote.swaption_vol(expiry, tenor, 0.03, vol, "ATM")
        for expiry, tenor, vol in swaption_specs
    ]

    rates_quotes = discount_quotes + forward_3m_quotes + forward_6m_quotes
    market_quotes: List[cal.MarketQuote] = []
    market_quotes.extend(q.to_market_quote() for q in rates_quotes)
    market_quotes.extend(q.to_market_quote() for q in credit_quotes)
    market_quotes.extend(q.to_market_quote() for q in inflation_quotes)
    market_quotes.extend(q.to_market_quote() for q in equity_vol_quotes)
    market_quotes.extend(q.to_market_quote() for q in swaption_quotes)

    forward_inputs = {"3M": forward_3m_quotes, "6M": forward_6m_quotes}

    return market_quotes, forward_inputs, swaption_specs


def ensure_swaption_surface(
    market: MarketContext,
    base_date: date,
    points: Sequence[SwaptionPoint],
) -> bool:
    """Populate a basic swaption surface if calibration did not generate one."""

    try:
        market.surface("SWAPTION-VOL")
        return False
    except (ValueError, KeyError):
        pass

    if not points:
        return False

    expiries = sorted({(expiry - base_date).days / 365.0 for expiry, _, _ in points})
    tenors = sorted({(tenor - expiry).days / 365.0 for expiry, tenor, _ in points})
    grid: List[List[float]] = []
    for expiry_years in expiries:
        row: List[float] = []
        for tenor_years in tenors:
            vols = [vol for e, t, vol in points if (
                (e - base_date).days / 365.0 == expiry_years
                and (t - e).days / 365.0 == tenor_years
            )]
            row.append(sum(vols) / len(vols) if vols else 0.0)
        grid.append(row)

    surface = MarketVolSurface("SWAPTION-VOL", expiries, tenors, grid)
    market.insert_surface(surface)
    return True


def calibrate_forward_curves(
    market: MarketContext,
    base_date: date,
    forward_inputs: Dict[str, List[cal.RatesQuote]],
) -> Dict[str, Dict[str, object]]:
    """Calibrate tenor-specific forward curves using the official calibrators."""

    reports: Dict[str, Dict[str, object]] = {}
    tenor_meta: Dict[str, Tuple[str, float]] = {
        "1M": ("USD-SOFR-1M-FWD", 1.0 / 12.0),
        "3M": ("USD-SOFR-3M-FWD", 0.25),
        "6M": ("USD-SOFR-6M-FWD", 0.50),
    }

    for label in ("3M", "6M"):
        quotes = forward_inputs.get(label)
        if not quotes or len(quotes) < 2:
            continue

        curve_id, tenor_years = tenor_meta[label]
        try:
            market.discount("USD-OIS")
        except (ValueError, KeyError):
            reports[curve_id] = {
                "success": False,
                "error": "USD-OIS discount curve unavailable",
            }
            continue

        calibrator = cal.ForwardCurveCalibrator(curve_id, tenor_years, base_date, "USD", "USD-OIS")
        calibrator = calibrator.with_config(
            cal.CalibrationConfig.multi_curve()
            .with_solver_kind(cal.SolverKind.HYBRID)
            .with_max_iterations(100)
        )

        try:
            curve, report = calibrator.calibrate(quotes, market)
        except RuntimeError as exc:
            reports[curve_id] = {"success": False, "error": str(exc)}
            continue

        market.insert_forward(curve)
        report_dict = report.to_dict()
        if report_dict.get("max_residual", 0.0) >= 1e11:
            report_dict["note"] = "residual capped by penalty (limited quote coverage)"
        report_dict["success"] = True
        reports[curve_id] = report_dict

    return reports


def summarize_context(
    context: MarketContext,
    forward_reports: Optional[Dict[str, Dict[str, object]]] = None,
) -> None:
    stats = context.stats()
    print("Curves:", stats["curve_counts"])
    print("Vol surfaces:", stats["surface_count"])

    if forward_reports:
        for curve_id, data in forward_reports.items():
            if data.get("success"):
                meta = data.get("metadata", {})
                note = data.get("note") or meta.get("note")
                msg = (
                    f"{curve_id} calibrated (iterations={data.get('iterations', 0)},"
                    f" max residual={float(data.get('max_residual', 0.0)):.6f})"
                )
                print(msg)
                if note:
                    print(f"  note: {note}")
            else:
                print(f"{curve_id} calibration failed: {data.get('error')}")

    try:
        usd_ois = context.discount("USD-OIS")
        print("USD-OIS df(5y):", round(usd_ois.df(5.0), 6))
    except (ValueError, KeyError):
        print("USD-OIS discount curve missing")

    for curve_id, sample in [
        ("USD-SOFR-3M-FWD", 2.0),
        ("USD-SOFR-6M-FWD", 2.0),
    ]:
        try:
            fwd_curve = context.forward(curve_id)
            print(f"{curve_id} rate(2y):", round(fwd_curve.rate(sample), 6))
        except (ValueError, KeyError):
            print(f"{curve_id} missing")

    try:
        hazard = context.hazard("ACME-Senior")
        print("ACME senior survival(5y):", round(hazard.survival(5.0), 6))
    except (ValueError, KeyError):
        print("ACME hazard curve missing")

    try:
        inflation = context.inflation("US-CPI-U")
        print("US-CPI-U level(5y):", round(inflation.cpi(5.0), 4))
    except (ValueError, KeyError):
        print("US-CPI-U inflation curve missing")

    for surface_id, label in [("ACME-VOL", "ACME equity"), ("SWAPTION-VOL", "Swaption")]:
        try:
            surface = context.surface(surface_id)
            expiries = list(surface.expiries)
            strikes = list(surface.strikes)
            if expiries and strikes:
                sample = (expiries[min(1, len(expiries) - 1)], strikes[len(strikes) // 2])
                print(
                    f"{label} vol {round(sample[0], 3)}y / strike {round(sample[1], 2)}:",
                    round(surface.value(sample[0], sample[1]), 4),
                )
            else:
                print(f"{surface_id} surface missing grid data")
        except (ValueError, KeyError):
            print(f"{surface_id} surface missing")


def main() -> None:
    base_date = date(2024, 1, 2)
    market_quotes, forward_inputs, swaption_specs = build_market_quotes(base_date)

    config = (
        cal.CalibrationConfig.multi_curve()
        .with_solver_kind(cal.SolverKind.HYBRID)
        .with_max_iterations(40)
        .with_verbose(False)
    )

    calibration = cal.SimpleCalibration(base_date, "USD", config=config)
    calibration.set_multi_curve_config(cal.MultiCurveConfig(True, True))
    calibration.add_entity_seniority("ACME", "senior")

    market, report = calibration.calibrate(market_quotes)
    forward_reports = calibrate_forward_curves(market, base_date, forward_inputs)
    fallback_surface = ensure_swaption_surface(market, base_date, swaption_specs)
    report_info = report.to_dict()

    print("=== Simple Calibration Summary ===")
    print("Success:", report_info["success"])
    print("Report type:", report_info["metadata"].get("type"))
    print("Iterations:", report_info["iterations"])
    print("Max residual:", round(report_info["max_residual"], 8))
    # Print top-N residuals by absolute value for quick diagnosis
    TOP_N = 10
    residuals = report_info.get("residuals", {})
    if residuals:
        print(f"Top {min(TOP_N, len(residuals))} residuals (by |value|):")
        for name, value in sorted(residuals.items(), key=lambda kv: abs(kv[1]), reverse=True)[:TOP_N]:
            print(f"  {name}: {value:.8f}")
    print("Convergence reason:", report_info["convergence_reason"])
    print("Residual count:", len(report_info["residuals"]))
    print("Quotes used:", len(market_quotes))

    print("\n=== Calibrated Market Snapshot ===")
    summarize_context(market, forward_reports)
    if fallback_surface:
        print("Swaption surface: populated from sample ATM grid")


if __name__ == "__main__":
    main()
