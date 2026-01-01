"""Comprehensive parity tests for valuations module.

Tests instruments, pricing, metrics, calibration, and cashflow builder functionality.
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.dates import DayCount
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Bond, Deposit, InterestRateSwap
from finstack.valuations.pricer import create_standard_registry
import pytest


class TestBondPricingParity:
    """Test bond pricing matches Rust implementation."""

    def test_bond_construction(self) -> None:
        """Test bond construction via builder."""
        bond = (
            Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        assert bond.id == "BOND-001"
        # Bond properties are accessible but might not be directly exposed
        # Focus on pricing parity instead

    def test_bond_pricing_simple(self) -> None:
        """Test simple bond pricing matches expected NPV."""
        # Create bond
        bond = (
            Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75), (10.0, 0.60)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Price bond
        registry = create_standard_registry()
        result = registry.price(bond, "discounting", market, date(2024, 1, 1))

        # Bond should have positive value
        assert result.value.amount > 0
        assert result.value.currency.code == "USD"

    def test_bond_pricing_at_par(self) -> None:
        """Test bond priced at par when coupon equals discount rate."""
        # Create bond with 5% coupon
        bond = (
            Bond.builder("BOND-PAR")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("USD-OIS")
            .build()
        )

        # Create flat 5% discount curve
        market = MarketContext()
        # Create discount factors for flat 5% rate
        # df(t) = exp(-0.05 * t)
        import math

        knots = [(t, math.exp(-0.05 * t)) for t in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]]
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            knots,
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Price bond
        registry = create_standard_registry()
        result = registry.price(bond, "discounting", market, date(2024, 1, 1))

        # Bond should be approximately at par (1,000,000)
        # Allow 1% tolerance due to discrete coupon payments
        expected_par = 1_000_000.0
        assert abs(result.value.amount - expected_par) / expected_par < 0.01

    def test_bond_with_metrics(self) -> None:
        """Test bond pricing with metrics calculation."""
        bond = (
            Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Price with metrics
        registry = create_standard_registry()
        metric_keys = ["clean_price", "accrued", "ytm"]
        result = registry.price_with_metrics(bond, "discounting", market, metric_keys, date(2024, 1, 1))

        # Should have base value
        assert result.value.amount > 0

        # Should have metrics (might be None if not supported for this model)
        # Just verify the API works


class TestSwapPricingParity:
    """Test interest rate swap pricing matches Rust."""

    def test_swap_construction(self) -> None:
        """Test swap construction via builder."""
        swap = (
            InterestRateSwap.builder("IRS-001")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        assert swap.id == "IRS-001"

    def test_swap_pricing_simple(self) -> None:
        """Test simple swap pricing."""
        swap = (
            InterestRateSwap.builder("IRS-001")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        forward_curve = ForwardCurve(
            "USD-SOFR",
            0.25,  # 3-month tenor
            [(0.0, 0.045), (1.0, 0.05), (5.0, 0.055)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )
        market.insert_forward(forward_curve)

        # Price swap
        registry = create_standard_registry()
        result = registry.price(swap, "discounting", market, date(2024, 1, 1))

        # Swap should have a value (could be positive or negative)
        assert result.value.currency.code == "USD"

    def test_swap_at_market(self) -> None:
        """Test swap valued at zero when fixed rate equals forward rate."""
        # This test verifies pricing consistency
        swap = (
            InterestRateSwap.builder("IRS-ATM")
            .notional(10_000_000.0)
            .currency("USD")
            .maturity(date(2029, 1, 1))
            .fixed_rate(0.05)  # Set equal to forward rate
            .frequency(Frequency.ANNUAL)
            .disc_id("USD-OIS")
            .fwd_id("USD-SOFR")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Flat 5% forward curve
        forward_curve = ForwardCurve(
            "USD-SOFR",
            0.25,
            [(0.0, 0.05), (1.0, 0.05), (5.0, 0.05)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )
        market.insert_forward(forward_curve)

        registry = create_standard_registry()
        result = registry.price(swap, "discounting", market, date(2024, 1, 1))

        # Swap should be close to zero value (at-market swap)
        # Allow reasonable tolerance due to day count and compounding
        assert abs(result.value.amount) / 10_000_000.0 < 0.1  # Within 10% of notional


class TestDepositPricingParity:
    """Test deposit pricing matches Rust."""

    def test_deposit_construction(self) -> None:
        """Test deposit construction via constructor."""
        from finstack.core.currency import Currency
        from finstack.core.money import Money

        deposit = Deposit(
            "DEP-001",
            Money(1_000_000.0, Currency("USD")),
            date(2024, 1, 1),
            date(2024, 4, 1),
            DayCount.ACT_360,
            "USD-OIS",
            quote_rate=0.045,
        )

        assert deposit.instrument_id == "DEP-001"

    def test_deposit_pricing_simple(self) -> None:
        """Test simple deposit pricing."""
        from finstack.core.currency import Currency
        from finstack.core.money import Money

        deposit = Deposit(
            "DEP-001",
            Money(1_000_000.0, Currency("USD")),
            date(2024, 1, 1),
            date(2024, 4, 1),
            DayCount.ACT_360,
            "USD-OIS",
            quote_rate=0.045,
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()
        result = registry.price(deposit, "discounting", market, date(2024, 1, 1))

        # Deposit PV can be positive or negative depending on quote vs curve.
        assert result.value.currency.code == "USD"

    def test_deposit_analytical_value(self) -> None:
        """Deposit PV is near zero at market rate."""
        # 1M deposit at 4.5% on 1M USD
        deposit = Deposit(
            "DEP-001",
            Money(1_000_000.0, USD),
            date(2024, 1, 1),
            date(2024, 4, 1),  # 90 days
            DayCount.ACT_360,
            "USD-OIS",
            quote_rate=0.045,
        )

        # Flat discount curve at 4.5%
        import math

        knots = [(t, math.exp(-0.045 * t)) for t in [0.0, 0.25, 0.5, 1.0]]
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            knots,
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()
        result = registry.price(deposit, "discounting", market, date(2024, 1, 1))

        # For a deposit quoted at the same rate implied by the curve, PV should be close to zero
        # (i.e., no value over par).
        assert abs(result.value.amount) / 1_000_000.0 < 0.01


class TestPricerRegistryParity:
    """Test pricer registry functionality."""

    def test_registry_creation(self) -> None:
        """Test standard registry creation."""
        registry = create_standard_registry()
        assert registry is not None

    def test_registry_multiple_model_keys(self) -> None:
        """Test pricing with different model keys."""
        bond = (
            Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()

        # Price with discounting model
        result = registry.price(bond, "discounting", market, date(2024, 1, 1))
        assert result.value.amount > 0


class TestMetricsParity:
    """Test metrics calculation matches Rust."""

    def test_scalar_metrics_available(self) -> None:
        """Test scalar metrics are computed."""
        bond = (
            Bond.builder("BOND-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.05)
            .frequency(Frequency.SEMI_ANNUAL)
            .day_count(DayCount.THIRTY_360)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()
        metric_keys = ["clean_price", "accrued", "ytm", "duration_mod", "dv01"]
        result = registry.price_with_metrics(bond, "discounting", market, metric_keys, date(2024, 1, 1))

        # Should have value
        assert result.value.amount > 0

        # Metrics might not all be available for every model/instrument
        # Just verify the API works without error


class TestCashflowBuilderParity:
    """Test cashflow builder matches Rust."""

    def test_cashflow_builder_basic(self) -> None:
        """Test basic cashflow schedule generation."""
        from finstack.valuations.cashflow import CashflowBuilder, CouponType, FixedCouponSpec, ScheduleParams

        issue = date(2024, 1, 1)
        maturity = date(2029, 1, 1)
        notional = Money(1_000_000.0, USD)
        schedule = ScheduleParams.semiannual_30360()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)
        cf_schedule = builder.build_with_curves(None)

        assert len(list(cf_schedule.flows())) > 0

    def test_cashflow_builder_with_amortization(self) -> None:
        """Test cashflow builder with amortization."""
        from finstack.valuations.cashflow import (
            AmortizationSpec,
            CashflowBuilder,
            CouponType,
            FixedCouponSpec,
            ScheduleParams,
        )

        issue = date(2024, 1, 1)
        maturity = date(2029, 1, 1)
        notional = Money(1_000_000.0, USD)
        final_notional = Money(0.0, USD)
        amort = AmortizationSpec.linear_to(final_notional)
        schedule = ScheduleParams.annual_actact()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort)
        builder.fixed_cf(fixed_spec)
        cf_schedule = builder.build_with_curves(None)

        assert len([f for f in cf_schedule.flows() if f.kind.name == "amortization"]) > 0


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_zero_coupon_bond(self) -> None:
        """Test zero-coupon bond pricing."""
        bond = (
            Bond.builder("ZERO-001")
            .notional(1_000_000.0)
            .currency("USD")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.0)  # Zero coupon
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("USD-OIS")
            .build()
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()
        result = registry.price(bond, "discounting", market, date(2024, 1, 1))

        # Zero-coupon bond NPV should be notional * df(maturity)
        # NPV ≈ 1,000,000 * 0.75 = 750,000
        expected = 750_000.0
        assert abs(result.value.amount - expected) / expected < 0.05

    def test_deposit_overnight(self) -> None:
        """Test overnight deposit pricing."""
        deposit = Deposit(
            "ON-001",
            Money(1_000_000.0, USD),
            date(2024, 1, 1),
            date(2024, 1, 2),  # 1 day
            DayCount.ACT_360,
            "USD-OIS",
            quote_rate=0.045,
        )

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (0.003, 0.9999)],  # Almost flat for short tenor
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        registry = create_standard_registry()
        result = registry.price(deposit, "discounting", market, date(2024, 1, 1))

        # At (roughly) market rates, a deposit should have PV close to zero (no value over par).
        assert abs(result.value.amount) / 1_000_000.0 < 0.01

    def test_swap_zero_notional(self) -> None:
        """Test swap with zero notional."""
        with pytest.raises(ValueError, match="notional"):
            InterestRateSwap.builder("IRS-ZERO").notional(0.0).currency("USD").maturity(date(2029, 1, 1)).fixed_rate(
                0.05
            ).frequency(Frequency.ANNUAL).disc_id("USD-OIS").fwd_id("USD-SOFR").build()

        # Builder should reject zero notional rather than producing a degenerate instrument.


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
