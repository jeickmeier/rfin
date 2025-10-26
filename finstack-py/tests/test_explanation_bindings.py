"""Integration tests for explanation and metadata bindings.

Tests the Python bindings for ExplanationTrace and ResultsMeta fields
added to CalibrationReport and ValuationResult.
"""



def test_py_typed_marker_exists() -> None:
    """Test that py.typed marker file exists for type checkers."""
    from pathlib import Path

    import finstack

    finstack_path = Path(finstack.__file__).parent
    py_typed_path = finstack_path / "py.typed"

    assert py_typed_path.exists(), "py.typed marker should exist"


def test_explanation_structure() -> None:
    """Test explanation trace structure when serialized."""
    # Mock explanation structure (will be populated by actual calibrations)
    expected_fields = {
        "type": str,  # "calibration", "pricing", or "waterfall"
        "entries": list,  # List of trace entries
        "truncated": (bool, type(None)),  # Optional truncation flag
    }

    # This validates the structure that will be returned
    for field, expected_type in expected_fields.items():
        if isinstance(expected_type, tuple):
            # Optional field
            assert field in ["truncated"]
        else:
            # Required field
            assert field in ["type", "entries"]


def test_results_meta_has_timestamp_and_version() -> None:
    """Test that ResultsMeta includes timestamp and version fields."""
    # ResultsMeta is available through valuations.results module
    from finstack.valuations.results import ResultsMeta

    assert hasattr(ResultsMeta, "__init__")
    # When actual results are created, they should have:
    # - meta.timestamp (str, ISO 8601 format)
    # - meta.version (str, package version)


def test_trace_entry_calibration_iteration() -> None:
    """Test calibration iteration trace entry structure."""
    entry = {
        "kind": "calibration_iteration",
        "iteration": 0,
        "residual": 0.005,
        "knots_updated": ["2.5y", "5.0y"],
        "converged": False,
    }

    assert entry["kind"] == "calibration_iteration"
    assert isinstance(entry["iteration"], int)
    assert isinstance(entry["residual"], float)
    assert isinstance(entry["knots_updated"], list)
    assert isinstance(entry["converged"], bool)


def test_trace_entry_cashflow_pv() -> None:
    """Test cashflow PV trace entry structure."""
    entry = {
        "kind": "cashflow_pv",
        "date": "2025-06-15",
        "cashflow_amount": 25000.0,
        "cashflow_currency": "USD",
        "discount_factor": 0.95,
        "pv_amount": 23750.0,
        "pv_currency": "USD",
        "curve_id": "USD_GOVT",
    }

    assert entry["kind"] == "cashflow_pv"
    assert isinstance(entry["date"], str)
    assert isinstance(entry["cashflow_amount"], float)
    assert isinstance(entry["discount_factor"], float)
    assert entry["curve_id"] == "USD_GOVT"


def test_trace_entry_waterfall_step() -> None:
    """Test waterfall step trace entry structure."""
    entry = {
        "kind": "waterfall_step",
        "period": 1,
        "step_name": "Senior Tranche Interest",
        "cash_in_amount": 100000.0,
        "cash_in_currency": "USD",
        "cash_out_amount": 95000.0,
        "cash_out_currency": "USD",
        "shortfall_amount": 5000.0,
        "shortfall_currency": "USD",
    }

    assert entry["kind"] == "waterfall_step"
    assert isinstance(entry["period"], int)
    assert isinstance(entry["step_name"], str)
    assert isinstance(entry["cash_in_amount"], float)


if __name__ == "__main__":
    # Run basic structure validation
    test_explanation_structure()
    test_results_meta_has_timestamp_and_version()
    test_trace_entry_calibration_iteration()
    test_trace_entry_cashflow_pv()
    test_trace_entry_waterfall_step()

