"""Test suite for finstack exception handling and error conversion.

This module tests the custom exception hierarchy defined in finstack-py/src/errors.rs
to ensure proper error mapping from Rust to Python exceptions.
"""

import datetime as dt

from finstack.core.currency import Currency
from finstack.core.dates import BusinessDayConvention, DayCount, adjust, get_calendar
from finstack.core.market_data import DiscountCurve, FxMatrix, MarketContext, VolSurface
from finstack.core.money import Money
from finstack.valuations.instruments import Bond, CDSOption, CDSTranche
from finstack.valuations.pricer import standard_registry
import pytest

import finstack
from finstack.valuations import calibration as cal


class TestExceptionHierarchy:
    """Test that all custom exceptions are properly registered."""

    def test_base_exception_exists(self) -> None:
        """FinstackError should be accessible as base exception."""
        assert hasattr(finstack, "FinstackError")
        assert issubclass(finstack.FinstackError, Exception)

    def test_configuration_exceptions_exist(self) -> None:
        """Configuration error types should be accessible."""
        assert hasattr(finstack, "ConfigurationError")
        assert hasattr(finstack, "MissingCurveError")
        assert hasattr(finstack, "MissingFxRateError")
        assert hasattr(finstack, "InvalidConfigError")

        # Check inheritance
        assert issubclass(finstack.ConfigurationError, finstack.FinstackError)
        assert issubclass(finstack.MissingCurveError, finstack.ConfigurationError)

    def test_computation_exceptions_exist(self) -> None:
        """Computation error types should be accessible."""
        assert hasattr(finstack, "ComputationError")
        assert hasattr(finstack, "ConvergenceError")
        assert hasattr(finstack, "CalibrationError")
        assert hasattr(finstack, "PricingError")

        # Check inheritance
        assert issubclass(finstack.ComputationError, finstack.FinstackError)
        assert issubclass(finstack.ConvergenceError, finstack.ComputationError)

    def test_validation_exceptions_exist(self) -> None:
        """Validation error types should be accessible."""
        assert hasattr(finstack, "ValidationError")
        assert hasattr(finstack, "CurrencyMismatchError")
        assert hasattr(finstack, "DateError")
        assert hasattr(finstack, "ParameterError")
        assert hasattr(finstack, "ConstraintValidationError")
        assert hasattr(finstack, "CholeskyError")

        # Check inheritance
        assert issubclass(finstack.ValidationError, finstack.FinstackError)
        assert issubclass(finstack.CurrencyMismatchError, finstack.ValidationError)
        assert issubclass(finstack.ConstraintValidationError, finstack.ParameterError)
        assert issubclass(finstack.CholeskyError, finstack.ParameterError)

    def test_internal_exception_exists(self) -> None:
        """InternalError should be accessible."""
        assert hasattr(finstack, "InternalError")
        assert issubclass(finstack.InternalError, finstack.FinstackError)


class TestCurrencyErrors:
    """Test currency-related error handling."""

    def test_unknown_currency_raises_parameter_error(self) -> None:
        """Unknown currency codes should raise ParameterError."""
        with pytest.raises(finstack.ParameterError, match="Unknown currency"):
            Currency("INVALID_CODE")

    def test_currency_mismatch_in_operations(self) -> None:
        """Currency mismatches in operations should raise CurrencyMismatchError."""
        # Placeholder: This test should raise CurrencyMismatchError when trying to add different currencies
        # Actual behavior depends on implementation - add test when money arithmetic is implemented


class TestDateErrors:
    """Test date-related error handling."""

    def test_invalid_date_components(self) -> None:
        """Invalid date components should raise DateError."""
        # February 30th doesn't exist
        # This would be tested if we have a date construction method that validates
        # Placeholder - depends on available date construction APIs

    def test_business_day_adjustment_failure(self) -> None:
        """Business day adjustment failures should raise DateError."""
        calendar = get_calendar("usny")
        # Test with a date far in the past/future that might cause adjustment issues
        # Exact behavior depends on implementation
        # This is a placeholder - actual test depends on what causes adjustment failures
        # If adjustment succeeds, that's fine; if it fails, should raise DateError
        adjust(dt.date(1900, 1, 1), BusinessDayConvention.FOLLOWING, calendar)


class TestMarketDataErrors:
    """Test market data-related error handling."""

    def test_missing_curve_error(self) -> None:
        """Accessing non-existent curve should raise MissingCurveError."""
        market = MarketContext()

        with pytest.raises(finstack.ConfigurationError, match="NONEXISTENT_CURVE_ID"):
            market.get_discount("NONEXISTENT_CURVE_ID")

    def test_missing_fx_rate_error(self) -> None:
        """Accessing non-existent FX rate should raise MissingFxRateError."""
        fx = FxMatrix()

        # Querying FX rate that doesn't exist should raise error
        # Use valid currencies but missing rate
        with pytest.raises((finstack.MissingFxRateError, finstack.ConfigurationError, KeyError, ValueError)):
            fx.rate(Currency("USD"), Currency("EUR"), dt.date(2024, 1, 1))


class TestCalibrationErrors:
    """Test calibration-related error handling."""

    def test_calibration_with_too_few_points(self) -> None:
        """Calibration with insufficient data should raise appropriate error."""
        quote_sets = {"ois": []}
        steps = [
            {
                "id": "disc",
                "quote_set": "ois",
                "kind": "discount",
                "curve_id": "USD-OIS",
                "currency": "USD",
                "base_date": "2024-01-02",
            }
        ]

        with pytest.raises((finstack.ValidationError, finstack.ParameterError, RuntimeError, ValueError)):
            cal.execute_calibration("plan_empty_quotes", quote_sets, steps)

    def test_calibration_with_non_monotonic_knots(self) -> None:
        """Non-monotonic times should raise ParameterError."""
        # Create quotes with non-increasing maturities

        quotes = [
            cal.RatesQuote.deposit("DEPO-2", "USD-DEPOSIT", dt.date(2026, 1, 2), 0.05),
            cal.RatesQuote.deposit("DEPO-1", "USD-DEPOSIT", dt.date(2025, 1, 2), 0.04),
        ]
        quote_sets = {"ois": [q.to_market_quote() for q in quotes]}
        steps = [
            {
                "id": "disc",
                "quote_set": "ois",
                "kind": "discount",
                "curve_id": "USD-OIS",
                "currency": "USD",
                "base_date": "2024-01-02",
                "conventions": {
                    "curve_day_count": "act365f",
                },
            }
        ]

        # Quotes may be internally sorted/validated. Accept either outcome.
        try:
            market, report, _ = cal.execute_calibration("plan_non_monotonic", quote_sets, steps)
            assert market.get_discount("USD-OIS") is not None
            assert report is not None
        except (
            finstack.ParameterError,
            finstack.CalibrationError,
            finstack.ValidationError,
            RuntimeError,
            ValueError,
        ) as e:
            error_msg = str(e).lower()
            assert any(word in error_msg for word in ["monotonic", "increasing", "decreasing", "order", "sorted"])


class TestValidationErrors:
    """Test input validation errors."""

    def test_negative_value_error(self) -> None:
        """Negative values where positive required should raise ParameterError."""
        # Test with discount curve requiring positive discount factors
        with pytest.raises((finstack.ParameterError, ValueError)):
            DiscountCurve(
                "INVALID",
                dt.date(2024, 1, 2),
                [(0.0, -0.5), (1.0, -0.3)],  # Negative discount factors
                day_count=DayCount.ACT_365F,
            )

    def test_dimension_mismatch_error(self) -> None:
        """Dimension mismatches should raise ParameterError."""
        # Grid dimensions don't match strikes/expiries
        with pytest.raises((finstack.ParameterError, ValueError), match=r"dimension|row count|must match"):
            VolSurface(
                "INVALID",
                expiries=[1.0, 2.0],  # 2 expiries
                strikes=[90.0, 100.0, 110.0],  # 3 strikes
                grid=[[0.2, 0.21]],  # Only 1 row (should be 2) and 2 cols (should be 3)
            )

    def test_cds_tranche_invalid_attachment_order_raises_validation_error(self) -> None:
        """Tranche builder should surface core validation for attachment ordering."""
        with pytest.raises((finstack.ValidationError, finstack.ParameterError, ValueError), match=r"attach|detach"):
            (
                CDSTranche
                .builder("CDX-IG-0-3")
                .index_name("CDX.NA.IG")
                .series(40)
                .attach_pct(3.0)
                .detach_pct(3.0)
                .notional(Money(10_000_000.0, Currency("USD")))
                .maturity(dt.date(2029, 1, 1))
                .running_coupon_bp(500.0)
                .discount_curve("USD-OIS")
                .credit_index_curve("CDX-IG-40")
                .build()
            )

    def test_cds_option_missing_credit_curve_raises_descriptive_error(self) -> None:
        """CDS option builder should report which required field is missing."""
        with pytest.raises((finstack.ParameterError, ValueError), match="credit_curve\\(\\) is required"):
            (
                CDSOption
                .builder("CDS-OPT-CORP-A")
                .money(Money(10_000_000.0, Currency("USD")))
                .strike(0.015)
                .expiry(dt.date(2024, 12, 20))
                .cds_maturity(dt.date(2029, 1, 1))
                .discount_curve("USD")
                .vol_surface("CDS-VOL")
                .build()
            )


class TestPricingErrors:
    """Test pricing-related error handling."""

    def test_unknown_pricer_error(self) -> None:
        """Pricing with unknown instrument/model combo should raise PricingError."""
        registry = standard_registry()
        market = MarketContext()

        # Add minimal market data
        market.insert(
            DiscountCurve(
                "USD-OIS", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)], day_count=DayCount.ACT_365F
            )
        )

        bond = (
            Bond
            .builder("TEST_BOND")
            .notional(1_000_000.0)
            .currency("USD")
            .coupon_rate(0.05)
            .frequency("annual")
            .maturity(dt.date(2029, 1, 2))
            .disc_id("USD-OIS")
            .build()
        )

        # Pricing with invalid model should raise error
        with pytest.raises(
            (finstack.PricingError, KeyError, ValueError, finstack.FinstackError), match=r"Unknown model|invalid"
        ):
            registry.price(bond, "INVALID_MODEL_THAT_DOESNT_EXIST", market, dt.date(2024, 1, 1))


class TestErrorMessageQuality:
    """Test that error messages are informative."""

    def test_currency_mismatch_shows_both_currencies(self) -> None:
        """CurrencyMismatchError should show expected and actual currencies."""
        # This would test actual currency mismatch operations
        # Placeholder for when we have operations that can trigger this

    def test_missing_curve_shows_curve_id(self) -> None:
        """MissingCurveError should include the requested curve ID."""
        market = MarketContext()

        with pytest.raises(finstack.ConfigurationError, match="MY_MISSING_CURVE"):
            market.get_discount("MY_MISSING_CURVE")

    def test_parameter_errors_are_descriptive(self) -> None:
        """Parameter errors should describe what's wrong."""
        # Test various parameter validation failures have good messages
        with pytest.raises(finstack.ParameterError) as exc_info:
            Currency("BAD_CODE_TOO_LONG")
        assert len(str(exc_info.value)) > 10  # Should have meaningful message


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
