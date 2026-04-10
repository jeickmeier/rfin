from finstack.valuations import bond_schema, instrument_schema, instrument_types


def test_instrument_types_include_supported_tags() -> None:
    types = instrument_types()

    assert "bond" in types
    assert "cms_swap" in types


def test_instrument_schema_returns_dedicated_schema_when_available() -> None:
    schema = instrument_schema("bond")

    assert schema["title"] == "Bond"
    assert schema["$id"] == "https://finstack.dev/schemas/instrument/1/fixed_income/bond.schema.json"
    assert schema == bond_schema()


def test_instrument_schema_returns_fallback_for_missing_dedicated_schema() -> None:
    schema = instrument_schema("cms_swap")

    assert schema["properties"]["instrument"]["properties"]["type"]["const"] == "cms_swap"
    assert schema["$id"] == "https://finstack.dev/schemas/instrument/1/rates/cms_swap.schema.json"
    assert "Constant maturity swap" in schema["description"]


def test_instrument_schema_without_argument_returns_envelope() -> None:
    schema = instrument_schema()

    assert schema["title"] == "Finstack Instrument"
    assert "bond" in schema["properties"]["instrument"]["properties"]["type"]["enum"]
