"""Instrument pricing: bonds, swaps, options, calibration, attribution.

Bindings for the ``finstack-valuations`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import valuations as _valuations

ValuationResult = _valuations.ValuationResult
validate_instrument_json = _valuations.validate_instrument_json
price_instrument = _valuations.price_instrument
price_instrument_with_metrics = _valuations.price_instrument_with_metrics
list_standard_metrics = _valuations.list_standard_metrics

__all__: list[str] = [
    "ValuationResult",
    "list_standard_metrics",
    "price_instrument",
    "price_instrument_with_metrics",
    "validate_instrument_json",
]
