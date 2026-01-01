"""Tests for RealEstateAsset instrument."""

import datetime as dt
import pytest
from finstack.valuations.instruments import RealEstateAsset


def test_real_estate_dcf():
    """Test creating a real estate asset with DCF valuation."""
    noi_schedule = [
        (dt.date(2024, 12, 31), 4_500_000.0),
        (dt.date(2025, 12, 31), 4_800_000.0),
        (dt.date(2026, 12, 31), 5_000_000.0),
    ]

    asset = RealEstateAsset.create_dcf(
        "OFFICE-NYC-123",
        currency="USD",
        valuation_date=dt.date(2024, 1, 1),
        noi_schedule=noi_schedule,
        discount_rate=0.08,
        discount_curve_id="USD-OIS",
        terminal_cap_rate=0.065,
    )

    assert asset.instrument_id == "OFFICE-NYC-123"
    assert asset.currency.code == "USD"
    assert asset.valuation_method == "dcf"
    assert asset.discount_rate == 0.08
    assert asset.terminal_cap_rate == 0.065
    assert asset.discount_curve_id == "USD-OIS"
    assert len(asset.noi_schedule) == 3


def test_real_estate_direct_cap():
    """Test creating a real estate asset with direct cap valuation."""
    asset = RealEstateAsset.create_direct_cap(
        "RETAIL-LA-456",
        currency="USD",
        valuation_date=dt.date(2024, 1, 1),
        stabilized_noi=5_000_000.0,
        cap_rate=0.06,
        discount_curve_id="USD-OIS",
    )

    assert asset.instrument_id == "RETAIL-LA-456"
    assert asset.currency.code == "USD"
    assert asset.valuation_method == "direct_cap"
    assert asset.stabilized_noi == 5_000_000.0
    assert asset.cap_rate == 0.06
    assert asset.discount_curve_id == "USD-OIS"
    # Should have a single-entry schedule with stabilized NOI
    assert len(asset.noi_schedule) >= 1


def test_real_estate_direct_cap_with_schedule():
    """Test direct cap with explicit NOI schedule."""
    noi_schedule = [
        (dt.date(2023, 12, 31), 4_000_000.0),
        (dt.date(2024, 12, 31), 4_500_000.0),
        (dt.date(2025, 12, 31), 5_000_000.0),
    ]

    asset = RealEstateAsset.create_direct_cap(
        "MULTIFAMILY-SF-789",
        currency="USD",
        valuation_date=dt.date(2024, 1, 1),
        stabilized_noi=5_000_000.0,
        cap_rate=0.055,
        discount_curve_id="USD-OIS",
        noi_schedule=noi_schedule,
    )

    assert asset.stabilized_noi == 5_000_000.0
    assert asset.cap_rate == 0.055
    assert len(asset.noi_schedule) == 3


def test_real_estate_dcf_with_appraisal():
    """Test DCF with appraisal override."""
    from finstack.core.money import Money
    from finstack.core.currency import Currency

    noi_schedule = [
        (dt.date(2024, 12, 31), 3_000_000.0),
        (dt.date(2025, 12, 31), 3_200_000.0),
    ]

    appraisal = Money(80_000_000.0, Currency("USD"))

    asset = RealEstateAsset.create_dcf(
        "INDUSTRIAL-CHI-999",
        currency="USD",
        valuation_date=dt.date(2024, 1, 1),
        noi_schedule=noi_schedule,
        discount_rate=0.075,
        discount_curve_id="USD-OIS",
        appraisal_value=appraisal,
    )

    assert asset.appraisal_value is not None
    assert asset.appraisal_value.amount == 80_000_000.0


def test_real_estate_repr():
    """Test real estate asset repr."""
    asset = RealEstateAsset.create_direct_cap(
        "OFFICE-NYC-123",
        currency="USD",
        valuation_date=dt.date(2024, 1, 1),
        stabilized_noi=5_000_000.0,
        cap_rate=0.06,
        discount_curve_id="USD-OIS",
    )

    repr_str = repr(asset)
    assert "OFFICE-NYC-123" in repr_str
    assert "direct_cap" in repr_str
    assert "USD" in repr_str


def test_real_estate_invalid_noi_schedule():
    """Test error on empty NOI schedule for DCF."""
    with pytest.raises(ValueError, match="noi_schedule must contain at least one entry"):
        RealEstateAsset.create_dcf(
            "INVALID-001",
            currency="USD",
            valuation_date=dt.date(2024, 1, 1),
            noi_schedule=[],
            discount_rate=0.08,
            discount_curve_id="USD-OIS",
        )
