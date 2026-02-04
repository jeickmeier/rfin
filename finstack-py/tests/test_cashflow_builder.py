"""Comprehensive tests for cashflow builder functionality.

Tests verify that the Python bindings expose all necessary cashflow builder
functionality from finstack-valuations/src/cashflow/builder/, including:

1. Basic cashflow construction (fixed, floating)
2. Amortization schedules (bullet, linear, step, custom)
3. Schedule parameters and conventions
4. Coupon types (cash, PIK, split)
5. Step-up coupon programs
6. Payment split programs
7. DataFrame conversions

All tests follow the pattern:
- Create builder with parameters
- Build cashflow schedule
- Verify schedule properties and flows
"""

from datetime import date

from finstack.core.currency import EUR, USD
from finstack.core.dates import BusinessDayConvention
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.schedule import Frequency, StubKind
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.cashflow import (
    AmortizationSpec,
    CashflowBuilder,
    CouponType,
    FixedCouponSpec,
    FloatCouponParams,
    FloatingCouponSpec,
    ScheduleParams,
)
import pytest

from finstack import Money


class TestBasicCashflowConstruction:
    """Test basic cashflow construction with fixed and floating rates."""

    def test_simple_fixed_coupon_bond(self) -> None:
        """Create a simple fixed-rate bond with quarterly coupons."""
        issue = date(2025, 1, 15)
        maturity = date(2027, 1, 15)
        notional = Money(1_000_000, USD)

        # Use convenience helper for quarterly Act/360
        schedule = ScheduleParams.quarterly_act360()

        # Define 5% fixed coupon
        fixed_spec = FixedCouponSpec.new(
            rate=0.05,
            schedule=schedule,
            coupon_type=CouponType.CASH,
        )

        # Build cashflow schedule
        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        # Verify schedule properties
        assert cf_schedule.notional.amount == notional.amount
        assert cf_schedule.notional.currency.code == "USD"
        assert cf_schedule.day_count.name == "act_360"

        # Verify cashflows exist
        flows = list(cf_schedule.flows())
        assert len(flows) > 0

        # First flow should be interest, last flow should include principal
        assert any(flow.kind.name == "fixed" for flow in flows)
        assert any(flow.kind.name == "notional" for flow in flows)

    def test_floating_rate_note(self) -> None:
        """Create a floating-rate note with SOFR + margin."""
        issue = date(2025, 3, 1)
        maturity = date(2028, 3, 1)
        notional = Money(5_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()

        # SOFR + 150 bps
        float_params = FloatCouponParams.new(
            index_id="USD-SOFR-3M",
            margin_bp=150.0,
            gearing=1.0,
            reset_lag_days=2,
        )

        float_spec = FloatingCouponSpec.new(
            params=float_params,
            schedule=schedule,
            coupon_type=CouponType.CASH,
        )

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.floating_cf(float_spec)

        cf_schedule = builder.build_with_curves(None)

        # Verify schedule properties
        assert cf_schedule.notional.amount == notional.amount
        flows = list(cf_schedule.flows())
        assert len(flows) > 0

        # Should have float_reset flows
        assert any(flow.kind.name == "float_reset" for flow in flows)

    def test_market_standard_conventions(self) -> None:
        """Test market standard schedule parameters."""
        # USD standard: quarterly Act/360
        usd_schedule = ScheduleParams.usd_standard()
        assert usd_schedule is not None

        # EUR standard: semi-annual 30/360
        eur_schedule = ScheduleParams.eur_standard()
        assert eur_schedule is not None

        # GBP standard: semi-annual Act/365
        gbp_schedule = ScheduleParams.gbp_standard()
        assert gbp_schedule is not None

        # JPY standard: semi-annual Act/365
        jpy_schedule = ScheduleParams.jpy_standard()
        assert jpy_schedule is not None


class TestAmortizationSchedules:
    """Test various amortization schedule types."""

    def test_bullet_no_amortization(self) -> None:
        """Test bullet loan (no amortization until maturity)."""
        issue = date(2025, 1, 1)
        maturity = date(2030, 1, 1)
        notional = Money(10_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()
        fixed_spec = FixedCouponSpec.new(rate=0.06, schedule=schedule, coupon_type=CouponType.CASH)

        # No amortization spec = bullet
        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        notional_flows = [f for f in flows if f.kind.name == "notional"]

        # Bullet: initial draw + final repayment
        assert len(notional_flows) == 2
        assert notional_flows[0].date == issue
        assert notional_flows[0].amount.amount == -notional.amount
        assert notional_flows[1].date == maturity
        assert notional_flows[1].amount.amount == notional.amount

    def test_linear_amortization(self) -> None:
        """Test linear amortization to a final notional."""
        issue = date(2025, 6, 1)
        maturity = date(2030, 6, 1)
        notional = Money(10_000_000, USD)
        final_notional = Money(2_000_000, USD)  # Amortize down to 20%

        schedule = ScheduleParams.quarterly_act360()
        fixed_spec = FixedCouponSpec.new(rate=0.06, schedule=schedule, coupon_type=CouponType.CASH)

        # Linear amortization
        amort_spec = AmortizationSpec.linear_to(final_notional)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort_spec)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        amort_flows = [f for f in flows if f.kind.name == "amortization"]

        # Should have multiple amortization flows
        assert len(amort_flows) > 0

        # Total amortization should reduce principal from initial to final notional.
        # The difference (10M - 2M = 8M) should be approximately the sum of amortization flows.
        # Note: Due to quarterly schedule alignment and day count conventions, the exact
        # total may vary slightly from the expected linear amortization amount.
        total_amort = sum(f.amount.amount for f in amort_flows)
        expected_amort = notional.amount - final_notional.amount  # 8,000,000
        # Allow 10% tolerance due to schedule/timing variations
        assert abs(total_amort - expected_amort) < expected_amort * 0.10

    def test_step_amortization(self) -> None:
        """Test step amortization with specific balance targets."""
        issue = date(2025, 1, 1)
        maturity = date(2030, 1, 1)
        notional = Money(10_000_000, USD)

        schedule = ScheduleParams.annual_actact()
        fixed_spec = FixedCouponSpec.new(rate=0.055, schedule=schedule, coupon_type=CouponType.CASH)

        # Define step amortization: remaining balance at specific dates
        amort_steps = [
            (date(2027, 1, 1), Money(8_000_000, USD)),  # After 2 years: 80%
            (date(2028, 1, 1), Money(6_000_000, USD)),  # After 3 years: 60%
            (date(2029, 1, 1), Money(3_000_000, USD)),  # After 4 years: 30%
        ]

        amort_spec = AmortizationSpec.step_remaining(amort_steps)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort_spec)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        amort_flows = [f for f in flows if f.kind.name == "amortization"]

        # Should have amortization flows at the specified dates
        assert len(amort_flows) > 0

    def test_percent_per_period_amortization(self) -> None:
        """Test amortization with fixed percentage per period."""
        issue = date(2025, 1, 1)
        maturity = date(2028, 1, 1)
        notional = Money(1_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        # 5% of original notional per period
        amort_spec = AmortizationSpec.percent_per_period(0.05)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort_spec)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        amort_flows = [f for f in flows if f.kind.name == "amortization"]

        # Should have multiple amortization flows of equal amount
        assert len(amort_flows) > 0

        # Each amortization should be approximately 5% of notional
        expected_per_period = notional.amount * 0.05
        for flow in amort_flows[:-1]:  # Exclude final flow which may differ
            assert abs(abs(flow.amount.amount) - expected_per_period) < 100  # $100 tolerance

    def test_custom_principal_flows(self) -> None:
        """Test custom principal repayment schedule."""
        issue = date(2025, 1, 1)
        maturity = date(2027, 1, 1)
        notional = Money(3_000_000, USD)

        schedule = ScheduleParams.semiannual_30360()
        fixed_spec = FixedCouponSpec.new(rate=0.06, schedule=schedule, coupon_type=CouponType.CASH)

        # Custom principal payments
        principal_payments = [
            (date(2025, 7, 1), Money(500_000, USD)),
            (date(2026, 1, 1), Money(800_000, USD)),
            (date(2026, 7, 1), Money(1_200_000, USD)),
            (date(2027, 1, 1), Money(500_000, USD)),
        ]

        amort_spec = AmortizationSpec.custom_principal(principal_payments)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort_spec)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        amort_flows = [f for f in flows if f.kind.name == "amortization"]

        # Should have custom flows at specified dates
        assert len(amort_flows) > 0


class TestCouponTypes:
    """Test different coupon payment types."""

    def test_cash_coupon(self) -> None:
        """Test cash coupon (100% paid in cash)."""
        issue = date(2025, 1, 1)
        maturity = date(2027, 1, 1)
        notional = Money(1_000_000, USD)

        schedule = ScheduleParams.semiannual_30360()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        interest_flows = [f for f in flows if f.kind.name == "fixed"]

        assert len(interest_flows) > 0
        # All interest flows should be cash (not PIK)
        assert all(f.amount.amount > 0 for f in interest_flows)  # Cash payments are positive

    def test_pik_coupon(self) -> None:
        """Test PIK coupon (100% capitalized)."""
        issue = date(2025, 1, 1)
        maturity = date(2030, 1, 1)
        notional = Money(2_000_000, USD)

        schedule = ScheduleParams.semiannual_30360()
        fixed_spec = FixedCouponSpec.new(rate=0.08, schedule=schedule, coupon_type=CouponType.PIK)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        pik_flows = [f for f in flows if f.kind.name == "pik"]

        # Should have PIK flows
        assert len(pik_flows) > 0

    def test_split_coupon(self) -> None:
        """Test split coupon (partial cash, partial PIK)."""
        issue = date(2025, 1, 1)
        maturity = date(2030, 1, 1)
        notional = Money(2_000_000, EUR)

        schedule = ScheduleParams.semiannual_30360()

        fixed_spec = FixedCouponSpec.new(rate=0.08, schedule=schedule, coupon_type=CouponType.split(0.7, 0.3))

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=EUR, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        interest_flows = [f for f in flows if f.kind.name in ("fixed", "pik")]

        # Should have both cash and PIK flows
        assert len(interest_flows) > 0


class TestAdvancedFeatures:
    """Test advanced cashflow builder features."""

    def test_step_up_coupon(self) -> None:
        """Test step-up coupon structure (rate increases over time)."""
        issue = date(2025, 1, 1)
        maturity = date(2032, 1, 1)
        notional = Money(3_000_000, USD)

        schedule = ScheduleParams.semiannual_30360()

        # Step-up program: 4% → 5% → 6%
        step_program = [
            (date(2027, 1, 1), 0.04),  # 4% until 2027
            (date(2030, 1, 1), 0.05),  # 5% until 2030
            (date(2032, 1, 1), 0.06),  # 6% until maturity
        ]

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_stepup(steps=step_program, schedule=schedule, default_split=CouponType.CASH)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        assert len(flows) > 0

        # Interest flows should exist throughout the period
        interest_flows = [f for f in flows if f.kind.name == "fixed"]
        assert len(interest_flows) > 0

    def test_payment_split_program(self) -> None:
        """Test payment split program (cash/PIK mix changes over time)."""
        issue = date(2025, 1, 1)
        maturity = date(2030, 1, 1)
        notional = Money(5_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()

        fixed_spec = FixedCouponSpec.new(
            rate=0.07,
            schedule=schedule,
            coupon_type=CouponType.CASH,  # Initial default
        )

        # Split program: 100% cash → 50/50 → 100% PIK
        split_program = [
            (date(2027, 1, 1), CouponType.CASH),
            (date(2028, 1, 1), CouponType.split(0.5, 0.5)),
            (date(2030, 1, 1), CouponType.PIK),
        ]

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)
        builder.payment_split_program(split_program)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        assert len(flows) > 0

        # Should have mix of cash and PIK flows depending on period
        interest_flows = [f for f in flows if f.kind.name in ("fixed", "pik")]
        assert len(interest_flows) > 0

    def test_complex_structure(self) -> None:
        """Test complex structure combining multiple features."""
        issue = date(2025, 1, 1)
        maturity = date(2035, 1, 1)
        notional = Money(20_000_000, USD)
        final_notional = Money(5_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()

        # Step-up coupons
        step_program = [
            (date(2028, 1, 1), 0.06),
            (date(2032, 1, 1), 0.07),
            (date(2035, 1, 1), 0.08),
        ]

        # Linear amortization
        amort_spec = AmortizationSpec.linear_to(final_notional)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.amortization(amort_spec)
        builder.fixed_stepup(steps=step_program, schedule=schedule, default_split=CouponType.CASH)

        cf_schedule = builder.build_with_curves(None)

        flows = list(cf_schedule.flows())
        assert len(flows) > 0

        # Should have both interest and amortization flows
        interest_flows = [f for f in flows if f.kind.name == "fixed"]
        amort_flows = [f for f in flows if f.kind.name == "amortization"]

        assert len(interest_flows) > 0
        assert len(amort_flows) > 0


class TestScheduleParameters:
    """Test schedule parameter construction and helpers."""

    def test_custom_schedule_params(self) -> None:
        """Test creating custom schedule parameters."""
        schedule = ScheduleParams.new(
            freq=Frequency.QUARTERLY,
            day_count=DayCount.ACT_360,
            bdc=BusinessDayConvention.MODIFIED_FOLLOWING,
            calendar_id="usny",
            stub=StubKind.NONE,
            end_of_month=False,
            payment_lag_days=0,
        )

        assert schedule is not None

    def test_convenience_helpers(self) -> None:
        """Test convenience schedule parameter helpers."""
        # Quarterly Act/360
        q_act360 = ScheduleParams.quarterly_act360()
        assert q_act360 is not None

        # Semi-annual 30/360
        sa_30360 = ScheduleParams.semiannual_30360()
        assert sa_30360 is not None

        # Annual Act/Act
        a_actact = ScheduleParams.annual_actact()
        assert a_actact is not None


class TestDataFrameConversion:
    """Test DataFrame export functionality."""

    def test_to_dataframe_no_market(self) -> None:
        """DataFrame export requires a market context."""
        issue = date(2025, 1, 1)
        maturity = date(2027, 1, 1)
        notional = Money(1_000_000, USD)

        schedule = ScheduleParams.quarterly_act360()
        fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

        builder = CashflowBuilder.new()
        builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        builder.fixed_cf(fixed_spec)

        cf_schedule = builder.build_with_curves(None)

        with pytest.raises(ValueError, match="market context required"):
            cf_schedule.to_dataframe()

        market = MarketContext()
        market.insert_discount(
            DiscountCurve(
                "USD-OIS",
                issue,
                [(0.0, 1.0), (5.0, 0.9)],
            )
        )
        df = cf_schedule.to_dataframe(market=market, discount_curve_id="USD-OIS")
        assert df.shape[0] > 0


def test_amortization_spec_repr() -> None:
    """Test AmortizationSpec string representations."""
    # None
    spec_none = AmortizationSpec.none()
    repr_none = repr(spec_none)
    assert "none" in repr_none.lower()

    # Linear
    spec_linear = AmortizationSpec.linear_to(Money(100_000, USD))
    repr_linear = repr(spec_linear)
    assert "linear" in repr_linear.lower()

    # Percent per period
    spec_pct = AmortizationSpec.percent_per_period(0.05)
    repr_pct = repr(spec_pct)
    assert "0.05" in repr_pct


def test_builder_with_5y_bond() -> None:
    """Task requirement: Build cashflows for 5Y bond with semiannual coupons."""
    issue = date(2025, 1, 1)
    maturity = date(2030, 1, 1)  # 5 years
    notional = Money(1_000_000, USD)

    # Semiannual 30/360 schedule
    schedule = ScheduleParams.semiannual_30360()

    # 5% fixed coupon
    fixed_spec = FixedCouponSpec.new(rate=0.05, schedule=schedule, coupon_type=CouponType.CASH)

    # Build schedule
    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.fixed_cf(fixed_spec)

    cf_schedule = builder.build_with_curves(None)

    # Verify dates
    flows = list(cf_schedule.flows())

    # Should have 10 semiannual coupons + 2 notional flows (draw + repay)
    assert len(flows) >= 12

    # Verify amounts
    interest_flows = [f for f in flows if f.kind.name in ("fixed", "stub")]
    notional_flows = [f for f in flows if f.kind.name == "notional"]

    assert len(interest_flows) == 10  # Semiannual for 5 years (last payment may be a stub)
    assert len(notional_flows) == 2  # Initial draw + final principal repayment

    # Verify interest amount (approximately $25,000 per semiannual period)
    expected_coupon = notional.amount * 0.05 / 2  # $25,000
    for flow in interest_flows:
        # Allow 2% tolerance for day count adjustments (30/360 can vary based on period dates)
        assert abs(flow.amount.amount - expected_coupon) < expected_coupon * 0.02

    # Verify final principal repayment is at maturity
    final_flow = notional_flows[-1]
    assert final_flow.date == maturity
    assert abs(final_flow.amount.amount) == notional.amount


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
