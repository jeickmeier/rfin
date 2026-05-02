"""Executable helpers for calibration golden fixtures."""

from __future__ import annotations

from itertools import pairwise
import math
from typing import Any

from tests.golden.schema import GoldenFixture


def run_curve_fixture(fixture: GoldenFixture) -> dict[str, float]:
    inputs = fixture.inputs
    discounts = {curve["id"]: curve for curve in inputs.get("discount", [])}
    forwards = {curve["id"]: curve for curve in inputs.get("forward", [])}
    inflations = {curve["id"]: curve for curve in inputs.get("inflation", [])}
    actuals: dict[str, float] = {}
    for probe in inputs.get("probes", []):
        curve_id = probe["curve"]
        tenor = float(probe["tenor"])
        kind = probe["kind"]
        if kind == "discount_factor":
            actuals[probe["output"]] = _interp_discount(discounts[curve_id], tenor)
        elif kind == "zero_rate":
            df = _interp_discount(discounts[curve_id], tenor)
            actuals[probe["output"]] = -math.log(df) / tenor
        elif kind == "forward_rate":
            actuals[probe["output"]] = _interp_linear(forwards[curve_id]["knots"], tenor)
        elif kind == "cpi":
            actuals[probe["output"]] = _interp_linear(inflations[curve_id]["knots"], tenor)
        elif kind == "inflation_zero_rate":
            curve = inflations[curve_id]
            cpi = _interp_linear(curve["knots"], tenor)
            actuals[probe["output"]] = math.log(cpi / float(curve["base_cpi"])) / max(tenor, 1e-12)
        else:
            msg = f"unsupported curve probe kind {kind!r}"
            raise ValueError(msg)
    if "calibration_rmse" in inputs:
        actuals["calibration_rmse"] = float(inputs["calibration_rmse"])
    return actuals


def run_hazard_fixture(fixture: GoldenFixture) -> dict[str, float]:
    inputs = fixture.inputs
    hazards = {curve["id"]: curve for curve in inputs.get("hazard", [])}
    actuals: dict[str, float] = {}
    for probe in inputs.get("probes", []):
        curve = hazards[probe["curve"]]
        tenor = float(probe["tenor"])
        if probe["kind"] == "hazard_rate":
            actuals[probe["output"]] = _hazard_rate(curve["knots"], tenor)
        elif probe["kind"] == "survival_probability":
            actuals[probe["output"]] = _survival_probability(curve["knots"], tenor)
        else:
            msg = f"unsupported hazard probe kind {probe['kind']!r}"
            raise ValueError(msg)
    if "calibration_rmse" in inputs:
        actuals["calibration_rmse"] = float(inputs["calibration_rmse"])
    return actuals


def run_vol_smile_fixture(fixture: GoldenFixture) -> dict[str, float]:
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
    if "repriced_rmse" in fixture.inputs:
        actuals["repriced_rmse"] = float(fixture.inputs["repriced_rmse"])
    return actuals


def run_sabr_cube_fixture(fixture: GoldenFixture) -> dict[str, float]:
    actuals = {key: float(value) for key, value in fixture.inputs["parameters"].items()}
    if "calibration_rmse" in fixture.inputs:
        actuals["calibration_rmse"] = float(fixture.inputs["calibration_rmse"])
    return actuals


def _interp_discount(curve: dict[str, Any], tenor: float) -> float:
    knots = curve["knots"]
    if curve.get("interp") == "log_linear":
        left_t, left_df, right_t, right_df = _bracket(knots, tenor)
        if right_t == left_t:
            return left_df
        weight = (tenor - left_t) / (right_t - left_t)
        return math.exp(math.log(left_df) + weight * (math.log(right_df) - math.log(left_df)))
    return _interp_linear(knots, tenor)


def _interp_linear(knots: list[list[float]], tenor: float) -> float:
    left_t, left_v, right_t, right_v = _bracket(knots, tenor)
    if right_t == left_t:
        return left_v
    weight = (tenor - left_t) / (right_t - left_t)
    return left_v + weight * (right_v - left_v)


def _bracket(knots: list[list[float]], tenor: float) -> tuple[float, float, float, float]:
    ordered = sorted((float(t), float(v)) for t, v in knots)
    if tenor <= ordered[0][0]:
        t, v = ordered[0]
        return t, v, t, v
    for left, right in pairwise(ordered):
        if tenor <= right[0]:
            return left[0], left[1], right[0], right[1]
    t, v = ordered[-1]
    return t, v, t, v


def _hazard_rate(knots: list[list[float]], tenor: float) -> float:
    ordered = sorted((float(t), float(v)) for t, v in knots)
    for t, hazard in ordered:
        if tenor <= t:
            return hazard
    return ordered[-1][1]


def _survival_probability(knots: list[list[float]], tenor: float) -> float:
    ordered = sorted((float(t), float(v)) for t, v in knots)
    last_t = 0.0
    integral = 0.0
    for knot_t, hazard in ordered:
        end_t = min(tenor, knot_t)
        if end_t > last_t:
            integral += hazard * (end_t - last_t)
            last_t = end_t
        if tenor <= knot_t:
            break
    if tenor > last_t:
        integral += ordered[-1][1] * (tenor - last_t)
    return math.exp(-integral)
