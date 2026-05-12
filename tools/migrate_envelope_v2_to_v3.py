"""One-shot migration: CalibrationEnvelope v2 → v3.

Reads a v2 JSON envelope, emits v3. Idempotent on already-v3 envelopes
(detected by presence of "market_data" / "prior_market" keys).

Usage:
    python3 tools/migrate_envelope_v2_to_v3.py path/to/v2.json > path/to/v3.json
    python3 tools/migrate_envelope_v2_to_v3.py path/to/v2.json --in-place
"""
import json
import sys
from pathlib import Path

# Curve `"type"` field → kind tag in v3 prior_market entries.
CURVE_KIND = {
    "discount":              "discount_curve",
    "forward":               "forward_curve",
    "hazard":                "hazard_curve",
    "inflation":             "inflation_curve",
    "base_correlation":      "base_correlation_curve",
    "basis_spread":          "basis_spread_curve",
    "parametric":            "parametric_curve",
    "price":                 "price_curve",
    "volatility_index":      "volatility_index_curve",
    "vol_index":             "volatility_index_curve",
}

QUOTE_CLASS_TO_KIND = {
    "rates":       "rate_quote",
    "cds":         "cds_quote",
    "cds_tranche": "cds_tranche_quote",
    "fx":          "fx_quote",
    "inflation":   "inflation_quote",
    "vol":         "vol_quote",
    "xccy":        "xccy_quote",
    "bond":        "bond_quote",
}

def migrate(env: dict) -> dict:
    # No-op if already v3.
    if "market_data" in env or "prior_market" in env:
        return env

    initial = env.pop("initial_market", None) or {}
    quote_sets_v2 = env.get("plan", {}).get("quote_sets", {})

    # 1. Flatten quotes: build the addressable bag + per-set ID lists.
    market_data = []
    seen_quote_ids = set()
    quote_sets_v3 = {}
    for set_name, quotes in quote_sets_v2.items():
        id_list = []
        for q in quotes:
            qid = q["id"]
            if qid not in seen_quote_ids:
                seen_quote_ids.add(qid)
                # Strip "class" key; turn it into the v3 "kind" tag.
                entry = {k: v for k, v in q.items() if k != "class"}
                entry = {"kind": QUOTE_CLASS_TO_KIND[q["class"]], **entry}
                market_data.append(entry)
            id_list.append(qid)
        quote_sets_v3[set_name] = id_list
    env.setdefault("plan", {})["quote_sets"] = quote_sets_v3

    # 2. Translate initial_market into market_data + prior_market.
    prior_market = []

    for curve in initial.get("curves", []) or []:
        ctype = curve.pop("type")
        prior_market.append({"kind": CURVE_KIND[ctype], **curve})

    for surface in initial.get("surfaces", []) or []:
        prior_market.append({"kind": "vol_surface", **surface})

    fx = initial.get("fx")
    if fx:
        # Move FxConfig into plan.settings.fx; quotes become fx_spot entries.
        env["plan"].setdefault("settings", {})["fx"] = fx.get("config", {})
        for entry in fx.get("quotes", []):
            # FxMatrixState.quotes serializes as either [from, to, rate] tuples
            # OR {"from": ..., "to": ..., "rate": ...} (newer shape). Handle both.
            if isinstance(entry, list):
                from_ccy, to_ccy, rate = entry
            else:
                from_ccy, to_ccy, rate = entry["from"], entry["to"], entry["rate"]
            market_data.append({
                "kind": "fx_spot",
                "id":   f"{from_ccy}/{to_ccy}",
                "from": from_ccy,
                "to":   to_ccy,
                "rate": rate,
            })

    for pid, scalar in (initial.get("prices") or {}).items():
        # PriceDatum is { id, scalar: MarketScalar }
        market_data.append({"kind": "price", "id": pid, "scalar": scalar})

    for series in initial.get("series", []) or []:
        market_data.append({"kind": "fixing_series", **series})

    for idx in initial.get("inflation_indices", []) or []:
        market_data.append({"kind": "inflation_fixings", **idx})

    for div in initial.get("dividends", []) or []:
        # DividendScheduleDatum is { schedule: DividendSchedule }
        market_data.append({"kind": "dividend_schedule", "schedule": div})

    for ci in initial.get("credit_indices", []) or []:
        market_data.append({"kind": "credit_index", **ci})

    for fxs in initial.get("fx_delta_vol_surfaces", []) or []:
        market_data.append({"kind": "fx_vol_surface", **fxs})

    for cube in initial.get("vol_cubes", []) or []:
        market_data.append({"kind": "vol_cube", **cube})

    for ccy, csa in (initial.get("collateral") or {}).items():
        market_data.append({"kind": "collateral", "id": ccy, "csa_currency": csa})

    hierarchy = initial.get("hierarchy")
    if hierarchy is not None:
        env["plan"].setdefault("settings", {})["hierarchy"] = hierarchy

    env["market_data"] = market_data
    env["prior_market"] = prior_market

    # Bump $schema path if it points at v2.
    if "$schema" in env and "calibration/2/" in env["$schema"]:
        env["$schema"] = env["$schema"].replace("calibration/2/", "calibration/3/")

    return env

def main():
    args = sys.argv[1:]
    in_place = "--in-place" in args
    args = [a for a in args if a != "--in-place"]
    if not args:
        sys.exit("usage: migrate_envelope_v2_to_v3.py <path> [--in-place]")
    path = Path(args[0])
    env = json.loads(path.read_text())
    out = migrate(env)
    text = json.dumps(out, indent=2)
    if in_place:
        path.write_text(text + "\n")
    else:
        sys.stdout.write(text + "\n")

if __name__ == "__main__":
    main()
