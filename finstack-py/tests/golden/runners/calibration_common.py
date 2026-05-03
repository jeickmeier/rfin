"""Executable helpers for calibration golden fixtures."""

from __future__ import annotations

from datetime import date
import math
from typing import Any

from finstack.core.market_data import DiscountCurve, ForwardCurve, HazardCurve, InflationCurve

from tests.golden.runners import validate_source_validation_fixture
from tests.golden.schema import GoldenFixture


def run_curve_fixture(fixture: GoldenFixture) -> dict[str, float]:
    validate_source_validation_fixture("curve calibration runner", fixture)
    inputs = fixture.inputs
    discounts = {curve["id"]: curve for curve in inputs.get("discount", [])}
    forwards = {curve["id"]: curve for curve in inputs.get("forward", [])}
    inflations = {curve["id"]: curve for curve in inputs.get("inflation", [])}
    discount_curves = {curve_id: _build_discount_curve(curve) for curve_id, curve in discounts.items()}
    forward_curves = {curve_id: _build_forward_curve(curve) for curve_id, curve in forwards.items()}
    inflation_curves = {curve_id: _build_inflation_curve(curve) for curve_id, curve in inflations.items()}
    actuals: dict[str, float] = {}
    for probe in inputs.get("probes", []):
        curve_id = probe["curve"]
        tenor = float(probe["tenor"])
        kind = probe["kind"]
        if kind == "discount_factor":
            actuals[probe["output"]] = float(discount_curves[curve_id].df(tenor))
        elif kind == "zero_rate":
            actuals[probe["output"]] = float(discount_curves[curve_id].zero(tenor))
        elif kind == "forward_rate":
            actuals[probe["output"]] = float(forward_curves[curve_id].rate(tenor))
        elif kind == "cpi":
            actuals[probe["output"]] = float(inflation_curves[curve_id].cpi(tenor))
        elif kind == "inflation_zero_rate":
            curve = inflations[curve_id]
            cpi = inflation_curves[curve_id].cpi(tenor)
            actuals[probe["output"]] = math.log(cpi / float(curve["base_cpi"])) / max(tenor, 1e-12)
        else:
            msg = f"unsupported curve probe kind {kind!r}"
            raise ValueError(msg)
    return _reject_non_executable_calibration("curve calibration runner", fixture)


def run_hazard_fixture(fixture: GoldenFixture) -> dict[str, float]:
    validate_source_validation_fixture("hazard calibration runner", fixture)
    inputs = fixture.inputs
    hazards = {curve["id"]: curve for curve in inputs.get("hazard", [])}
    hazard_curves = {curve_id: _build_hazard_curve(curve) for curve_id, curve in hazards.items()}
    actuals: dict[str, float] = {}
    for probe in inputs.get("probes", []):
        curve = hazard_curves[probe["curve"]]
        tenor = float(probe["tenor"])
        if probe["kind"] == "hazard_rate":
            actuals[probe["output"]] = float(curve.hazard_rate(tenor))
        elif probe["kind"] == "survival_probability":
            actuals[probe["output"]] = float(curve.survival(tenor))
        else:
            msg = f"unsupported hazard probe kind {probe['kind']!r}"
            raise ValueError(msg)
    return _reject_non_executable_calibration("hazard calibration runner", fixture)


def run_vol_smile_fixture(fixture: GoldenFixture) -> dict[str, float]:
    validate_source_validation_fixture("vol smile calibration runner", fixture)
    actuals: dict[str, float] = {}
    for smile in fixture.inputs["smiles"]:
        prefix = f"{smile['id']}::{smile['expiry']}"
        atm = float(smile["atm_vol"])
        put25 = float(smile["wing_25d_put_vol"])
        call25 = float(smile["wing_25d_call_vol"])
        actuals[f"atm_vol::{prefix}"] = atm
        actuals[f"wing_25d_put_vol::{prefix}"] = put25
        actuals[f"wing_25d_call_vol::{prefix}"] = call25
        actuals[f"risk_reversal_25d::{prefix}"] = call25 - put25
        actuals[f"butterfly_25d::{prefix}"] = 0.5 * (call25 + put25) - atm
        if "wing_10d_put_vol" in smile and "wing_10d_call_vol" in smile:
            put10 = float(smile["wing_10d_put_vol"])
            call10 = float(smile["wing_10d_call_vol"])
            actuals[f"risk_reversal_10d::{prefix}"] = call10 - put10
            actuals[f"butterfly_10d::{prefix}"] = 0.5 * (call10 + put10) - atm
    return _reject_non_executable_calibration("vol smile calibration runner", fixture)


def run_sabr_cube_fixture(fixture: GoldenFixture) -> dict[str, float]:
    validate_source_validation_fixture("SABR calibration runner", fixture)
    parameters = {key: float(value) for key, value in fixture.inputs["parameters"].items()}
    if not parameters:
        msg = "SABR source validation requires committed parameters"
        raise ValueError(msg)
    return _reject_non_executable_calibration("SABR calibration runner", fixture)


def _reject_non_executable_calibration(runner: str, fixture: GoldenFixture) -> dict[str, float]:
    if any(key in fixture.inputs for key in ("quotes", "market_quotes", "calibration_quotes")):
        quote_hint = (
            " quote inputs are present, but this golden runner has not wired them to a product calibration engine yet."
        )
    else:
        quote_hint = (
            " fixture contains final curves/parameters but no quote/source instruments from which to calibrate."
        )
    msg = (
        f"{runner} requires executable quote inputs and must compute RMSE from calibrated outputs;{quote_hint} "
        "Reclassify as source_validation or add a real calibration contract."
    )
    raise ValueError(msg)


def _build_discount_curve(spec: dict[str, Any]) -> DiscountCurve:
    return DiscountCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        knots=_knots(spec["knots"]),
        interp=spec.get("interp", "linear"),
        day_count=spec.get("day_count"),
    )


def _build_forward_curve(spec: dict[str, Any]) -> ForwardCurve:
    return ForwardCurve(
        id=spec["id"],
        tenor=float(spec["tenor"]),
        base_date=date.fromisoformat(spec["base_date"]),
        knots=_knots(spec["knots"]),
        interp=spec.get("interp", "linear"),
        day_count=spec.get("day_count", "act_360"),
    )


def _build_inflation_curve(spec: dict[str, Any]) -> InflationCurve:
    return InflationCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        base_cpi=float(spec["base_cpi"]),
        knots=_knots(spec["knots"]),
        day_count=spec.get("day_count", "act_365f"),
        indexation_lag_months=int(spec.get("indexation_lag_months", 3)),
        interp=spec.get("interp", "log_linear"),
    )


def _build_hazard_curve(spec: dict[str, Any]) -> HazardCurve:
    return HazardCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        knots=_knots(spec["knots"]),
        recovery_rate=spec.get("recovery_rate"),
        day_count=spec.get("day_count", "act_365f"),
    )


def _knots(raw_knots: list[list[float]]) -> list[tuple[float, float]]:
    return [(float(t), float(value)) for t, value in raw_knots]
