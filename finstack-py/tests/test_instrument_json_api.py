import datetime as dt
import json

from finstack.core.dates.schedule import Frequency
from finstack.valuations.instruments import (
    Bond,
    InterestRateSwap,
    instrument_from_dict,
    instrument_to_dict,
)

from finstack.valuations import (
    instrument_from_json as valuations_instrument_from_json,
    instrument_to_json as valuations_instrument_to_json,
)


def build_bond() -> Bond:
    return (
        Bond
        .builder("BOND_JSON_001")
        .notional(1_000_000.0)
        .currency("USD")
        .coupon_rate(0.05)
        .frequency("semiannual")
        .issue(dt.date(2025, 1, 15))
        .maturity(dt.date(2030, 1, 15))
        .disc_id("USD-OIS")
        .build()
    )


def build_swap() -> InterestRateSwap:
    return (
        InterestRateSwap
        .builder("SWAP_JSON_001")
        .notional(10_000_000.0)
        .currency("USD")
        .fixed_rate(0.03)
        .float_spread_bp(25.0)
        .frequency(Frequency.QUARTERLY)
        .start(dt.date(2025, 1, 15))
        .maturity(dt.date(2030, 1, 15))
        .disc_id("USD-OIS")
        .fwd_id("USD-LIBOR-3M")
        .build()
    )


def test_instrument_dict_roundtrip_uses_versioned_envelope() -> None:
    bond = build_bond()

    payload = instrument_to_dict(bond)

    assert payload["schema"] == "finstack.instrument/1"
    assert payload["instrument"]["type"] == "bond"

    restored = instrument_from_dict(payload)

    assert isinstance(restored, Bond)
    assert restored.instrument_id == bond.instrument_id
    assert restored.notional.amount == bond.notional.amount


def test_instrument_from_dict_accepts_bare_tagged_payload() -> None:
    swap = build_swap()

    envelope = instrument_to_dict(swap)
    restored = instrument_from_dict(envelope["instrument"])

    assert isinstance(restored, InterestRateSwap)
    assert restored.instrument_id == swap.instrument_id
    assert restored.notional.amount == swap.notional.amount


def test_top_level_json_helpers_roundtrip() -> None:
    bond = build_bond()

    payload = valuations_instrument_to_json(bond)
    restored = valuations_instrument_from_json(payload)

    assert isinstance(restored, Bond)
    assert restored.instrument_id == bond.instrument_id

    decoded = json.loads(payload)
    assert decoded["schema"] == "finstack.instrument/1"
    assert decoded["instrument"]["type"] == "bond"
