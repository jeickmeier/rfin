from __future__ import annotations

__all__: list[str]

def validate_instrument_json(json: str) -> str: ...
def price_instrument(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
) -> str: ...
def price_instrument_with_metrics(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
    metrics: list[str] = [],
    pricing_options: str | None = None,
) -> str: ...
def list_standard_metrics() -> list[str]: ...
def list_standard_metrics_grouped() -> dict[str, list[str]]: ...
