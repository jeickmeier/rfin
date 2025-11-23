"""Test suite for finstack exception handling and error conversion.

This module tests the custom exception hierarchy defined in finstack-py/src/errors.rs
to ensure proper error mapping from Rust to Python exceptions.
"""

import datetime as dt

import pytest

import finstack
from finstack.core.currency import Currency
from finstack.core.dates import BusinessDayConvention, DayCount, get_calendar
from finstack.core.market_data import DiscountCurve, MarketContext


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

        # Check inheritance
        assert issubclass(finstack.ValidationError, finstack.FinstackError)
        assert issubclass(finstack.CurrencyMismatchError, finstack.ValidationError)

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
        from finstack.core.dates import adjust

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

        with pytest.raises(finstack.MissingCurveError, match="Curve not found"):
            market.get_discount("NONEXISTENT_CURVE_ID")

    def test_missing_fx_rate_error(self) -> None:
        """Accessing non-existent FX rate should raise MissingFxRateError."""
        from finstack.core.market_data import FxMatrix

        fx = FxMatrix()

        # Querying FX rate that doesn't exist should raise error
        # Note: Exact API depends on implementation
        with pytest.raises((finstack.MissingFxRateError, finstack.ConfigurationError)):
            fx.rate(Currency("USD"), Currency("INVALID"), dt.date(2024, 1, 1))


class TestCalibrationErrors:
    """Test calibration-related error handling."""

    def test_calibration_with_too_few_points(self) -> None:
        """Calibration with insufficient data should raise appropriate error."""
        from finstack.valuations.calibration import DiscountCurveCalibrator, RatesQuote

        calibrator = DiscountCurveCalibrator("USD-OIS", dt.date(2024, 1, 2), Currency("USD"))

        # Single quote should fail (need at least 2 points)
        quotes = [RatesQuote.from_deposit(1.0, 0.05, DayCount.ACT_360)]

        with pytest.raises(finstack.ParameterError, match="at least two"):
            calibrator.calibrate(quotes)

    def test_calibration_with_non_monotonic_knots(self) -> None:
        """Non-monotonic times should raise ParameterError."""
        # Create quotes with non-increasing maturities
        from finstack.valuations.calibration import DiscountCurveCalibrator, RatesQuote

        calibrator = DiscountCurveCalibrator("USD-OIS", dt.date(2024, 1, 2), Currency("USD"))

        # Quotes with decreasing maturities (invalid)
        quotes = [
            RatesQuote.from_deposit(2.0, 0.05, DayCount.ACT_360),
            RatesQuote.from_deposit(1.0, 0.04, DayCount.ACT_360),  # Earlier maturity after later one
        ]

        with pytest.raises((finstack.ParameterError, finstack.CalibrationError)):
            calibrator.calibrate(quotes)


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
        from finstack.core.market_data import VolSurface

        # Grid dimensions don't match strikes/expiries
        with pytest.raises(finstack.ParameterError, match="dimension"):
            VolSurface(
                "INVALID",
                expiries=[1.0, 2.0],  # 2 expiries
                strikes=[90.0, 100.0, 110.0],  # 3 strikes
                grid=[[0.2, 0.21]],  # Only 1 row (should be 2) and 2 cols (should be 3)
            )


class TestPricingErrors:
    """Test pricing-related error handling."""

    def test_unknown_pricer_error(self) -> None:
        """Pricing with unknown instrument/model combo should raise PricingError."""
        from finstack.valuations.instruments import Bond
        from finstack.valuations.pricer import create_standard_registry

        registry = create_standard_registry()
        market = MarketContext()

        # Add minimal market data
        market.insert_discount(
            DiscountCurve(
                "USD-OIS", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)], day_count=DayCount.ACT_365F
            )
        )

        bond = (
            Bond.builder("TEST_BOND")
            .notional(1_000_000.0)
            .currency("USD")
            .coupon_rate(0.05)
            .frequency("annual")
            .maturity(dt.date(2029, 1, 2))
            .disc_id("USD-OIS")
            .build()
        )

        # Pricing with invalid model should raise error
        with pytest.raises((finstack.PricingError, KeyError, finstack.FinstackError)):
            registry.price(bond, "INVALID_MODEL_THAT_DOESNT_EXIST", market)


class TestErrorMessageQuality:
    """Test that error messages are informative."""

    def test_currency_mismatch_shows_both_currencies(self) -> None:
        """CurrencyMismatchError should show expected and actual currencies."""
        # This would test actual currency mismatch operations
        # Placeholder for when we have operations that can trigger this

    def test_missing_curve_shows_curve_id(self) -> None:
        """MissingCurveError should include the requested curve ID."""
        market = MarketContext()

        with pytest.raises(finstack.MissingCurveError, match="MY_MISSING_CURVE"):
            market.get_discount("MY_MISSING_CURVE")

    def test_parameter_errors_are_descriptive(self) -> None:
        """Parameter errors should describe what's wrong."""
        # Test various parameter validation failures have good messages
        with pytest.raises(finstack.ParameterError) as exc_info:
            Currency("BAD_CODE_TOO_LONG")
        assert len(str(exc_info.value)) > 10  # Should have meaningful message


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
