"""Property tests for currency safety invariants.

These tests verify that currency operations maintain safety guarantees:
- Same-currency operations never raise
- Cross-currency operations raise appropriately
- Currency preservation through operations
- Commutativity and associativity properties
"""

from collections.abc import Callable
from typing import Any

from finstack.core.currency import Currency
from finstack.core.money import Money
from hypothesis import assume, given, strategies as st
import pytest

# Hypothesis strategies for generating test data
major_currencies = st.sampled_from(["USD", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD"])
amounts = st.floats(min_value=-1e9, max_value=1e9, allow_nan=False, allow_infinity=False)
positive_amounts = st.floats(min_value=0.01, max_value=1e9, allow_nan=False, allow_infinity=False)
non_zero_amounts = st.floats(min_value=-1e9, max_value=1e9, allow_nan=False, allow_infinity=False).filter(
    lambda x: abs(x) > 1e-10
)


@st.composite
def money_strategy(draw: Callable[[Any], Any], currency_code: str | None = None) -> Money:
    """Generate Money instances with valid amounts."""
    if currency_code is None:
        currency_code = draw(major_currencies)
    amount = draw(amounts)
    currency = Currency(currency_code)
    return Money(amount, currency)


@st.composite
def same_currency_pair(draw: Callable[[Any], Any]) -> tuple[Money, Money]:
    """Generate two Money instances with the same currency."""
    currency_code = draw(major_currencies)
    amount1 = draw(amounts)
    amount2 = draw(amounts)
    currency = Currency(currency_code)
    return Money(amount1, currency), Money(amount2, currency)


@st.composite
def different_currency_pair(draw: Callable[[Any], Any]) -> tuple[Money, Money]:
    """Generate two Money instances with different currencies."""
    currency1_code = draw(major_currencies)
    currency2_code = draw(major_currencies.filter(lambda c: c != currency1_code))
    amount1 = draw(amounts)
    amount2 = draw(amounts)
    return Money(amount1, Currency(currency1_code)), Money(amount2, Currency(currency2_code))


class TestCurrencySafetyProperties:
    """Property tests for currency safety."""

    @given(same_currency_pair())
    def test_same_currency_addition_never_raises(self, money_pair: tuple[Money, Money]) -> None:
        """Adding Money with same currency never raises an exception."""
        m1, m2 = money_pair
        try:
            result = m1 + m2
            # Should succeed
            assert result.currency.code == m1.currency.code
            # Check numerical correctness with relative tolerance
            expected = m1.amount + m2.amount
            if abs(expected) > 1e-6:
                assert abs(result.amount - expected) / abs(expected) < 1e-6
            else:
                assert abs(result.amount - expected) < 1e-6
        except Exception as e:  # noqa: BLE001
            pytest.fail(f"Same-currency addition raised: {e}")

    @given(same_currency_pair())
    def test_same_currency_subtraction_never_raises(self, money_pair: tuple[Money, Money]) -> None:
        """Subtracting Money with same currency never raises an exception."""
        m1, m2 = money_pair
        try:
            result = m1 - m2
            # Should succeed
            assert result.currency.code == m1.currency.code
            # Check numerical correctness with relative tolerance
            expected = m1.amount - m2.amount
            if abs(expected) > 1e-6:
                assert abs(result.amount - expected) / abs(expected) < 1e-6
            else:
                assert abs(result.amount - expected) < 1e-6
        except Exception as e:  # noqa: BLE001
            pytest.fail(f"Same-currency subtraction raised: {e}")

    @given(different_currency_pair())
    def test_cross_currency_addition_raises(self, money_pair: tuple[Money, Money]) -> None:
        """Adding Money with different currencies raises an exception."""
        m1, m2 = money_pair
        with pytest.raises(Exception, match=r"[Cc]urrency|[Mm]ismatch"):
            _ = m1 + m2

    @given(different_currency_pair())
    def test_cross_currency_subtraction_raises(self, money_pair: tuple[Money, Money]) -> None:
        """Subtracting Money with different currencies raises an exception."""
        m1, m2 = money_pair
        with pytest.raises(Exception, match=r"[Cc]urrency|[Mm]ismatch"):
            _ = m1 - m2

    @given(money_strategy(), st.floats(min_value=-1e6, max_value=1e6, allow_nan=False, allow_infinity=False))
    def test_scalar_multiplication_preserves_currency(self, money: Money, scalar: float) -> None:
        """Multiplying Money by scalar preserves currency."""
        result = money * scalar
        assert result.currency.code == money.currency.code
        assert abs(result.amount - (money.amount * scalar)) < 1e-6 * abs(money.amount * scalar + 1)

    @given(money_strategy(), non_zero_amounts)
    def test_scalar_division_preserves_currency(self, money: Money, divisor: float) -> None:
        """Dividing Money by non-zero scalar preserves currency."""
        assume(abs(divisor) > 1e-10)  # Avoid division by zero
        result = money / divisor
        assert result.currency.code == money.currency.code
        expected = money.amount / divisor
        # Use relative tolerance for division
        if abs(expected) > 1e-10:
            assert abs(result.amount - expected) / abs(expected) < 1e-6
        else:
            assert abs(result.amount - expected) < 1e-10

    @given(same_currency_pair())
    def test_addition_commutative(self, money_pair: tuple[Money, Money]) -> None:
        """Addition of Money is commutative: a + b = b + a."""
        m1, m2 = money_pair
        result1 = m1 + m2
        result2 = m2 + m1
        assert result1.currency.code == result2.currency.code
        assert abs(result1.amount - result2.amount) < 1e-10

    @given(st.lists(money_strategy(currency_code="USD"), min_size=3, max_size=5))
    def test_addition_associative(self, money_list: list[Money]) -> None:
        """Addition of Money is associative: (a + b) + c = a + (b + c)."""
        if len(money_list) < 3:
            return

        a, b, c = money_list[0], money_list[1], money_list[2]

        # First compute (a + b) + c
        result1 = (a + b) + c

        # Then compute a + (b + c)
        result2 = a + (b + c)

        assert result1.currency.code == result2.currency.code
        # Internal Decimal arithmetic is exact, but .amount converts to f64.
        # At magnitude ~1e9 the f64 ULP is ~1e-7, so use relative tolerance.
        diff = abs(result1.amount - result2.amount)
        scale = max(abs(result1.amount), 1.0)
        assert diff / scale < 1e-12, f"Associativity: diff={diff}, scale={scale}"

    @given(money_strategy())
    def test_zero_addition_identity(self, money: Money) -> None:
        """Adding zero preserves the value (additive identity)."""
        zero = Money(0.0, money.currency)
        result = money + zero
        assert result.currency.code == money.currency.code
        assert abs(result.amount - money.amount) < 1e-10

    @given(money_strategy())
    def test_subtraction_self_gives_zero(self, money: Money) -> None:
        """Subtracting a value from itself gives zero."""
        result = money - money
        assert result.currency.code == money.currency.code
        assert abs(result.amount) < 1e-10

    @given(money_strategy())
    def test_multiplication_by_one_identity(self, money: Money) -> None:
        """Multiplying by 1 preserves the value (multiplicative identity)."""
        result = money * 1.0
        assert result.currency.code == money.currency.code
        assert abs(result.amount - money.amount) < 1e-10

    @given(money_strategy())
    def test_multiplication_by_zero_gives_zero(self, money: Money) -> None:
        """Multiplying by 0 gives zero amount."""
        result = money * 0.0
        assert result.currency.code == money.currency.code
        assert abs(result.amount) < 1e-10

    @given(money_strategy())
    def test_negation_via_multiplication(self, money: Money) -> None:
        """Negating Money (via multiplication by -1) preserves currency."""
        result = money * -1.0
        assert result.currency.code == money.currency.code
        assert abs(result.amount + money.amount) < 1e-6

    @given(money_strategy())
    def test_double_negation_via_multiplication(self, money: Money) -> None:
        """Negating twice (via multiplication) returns to original value."""
        result = (money * -1.0) * -1.0
        assert result.currency.code == money.currency.code
        # Use relative tolerance for large values
        if abs(money.amount) > 1e-6:
            assert abs(result.amount - money.amount) / abs(money.amount) < 1e-6
        else:
            assert abs(result.amount - money.amount) < 1e-6

    @given(same_currency_pair(), st.floats(min_value=-1e6, max_value=1e6, allow_nan=False, allow_infinity=False))
    def test_distributive_property(self, money_pair: tuple[Money, Money], scalar: float) -> None:
        """Scalar multiplication distributes over addition: k*(a + b) = k*a + k*b."""
        m1, m2 = money_pair

        # Compute k * (a + b)
        sum_first = m1 + m2
        result1 = sum_first * scalar

        # Compute k*a + k*b
        scaled1 = m1 * scalar
        scaled2 = m2 * scalar
        result2 = scaled1 + scaled2

        assert result1.currency.code == result2.currency.code
        # Use relative tolerance for floating-point comparisons
        expected = (m1.amount + m2.amount) * scalar
        if abs(expected) > 1e-10:
            assert abs(result1.amount - result2.amount) / abs(expected) < 1e-6
        else:
            assert abs(result1.amount - result2.amount) < 1e-9


class TestCurrencyImmutability:
    """Property tests for currency immutability."""

    @given(money_strategy())
    def test_operations_dont_mutate_original(self, money: Money) -> None:
        """Operations on Money don't mutate the original instance."""
        original_amount = money.amount
        original_currency_code = money.currency.code

        # Perform various operations
        _ = money + Money(10.0, money.currency)
        _ = money - Money(5.0, money.currency)
        _ = money * 2.0
        _ = -money

        # Check original is unchanged
        assert money.amount == original_amount
        assert money.currency.code == original_currency_code

    @given(st.lists(money_strategy(currency_code="USD"), min_size=2, max_size=10))
    def test_chain_operations_preserve_currency(self, money_list: list[Money]) -> None:
        """Chaining operations preserves currency throughout."""
        if len(money_list) < 2:
            return

        result = money_list[0]
        expected_currency = result.currency.code

        for money in money_list[1:]:
            result = result + money
            assert result.currency.code == expected_currency


class TestCurrencyComparison:
    """Property tests for currency comparison."""

    @given(same_currency_pair())
    def test_comparison_same_currency(self, money_pair: tuple[Money, Money]) -> None:
        """Comparing Money with same currency works correctly."""
        m1, m2 = money_pair

        # Comparison should work without raising
        if m1.amount < m2.amount:
            assert m1 < m2
            assert m1 <= m2
            assert not (m1 > m2)
            assert not (m1 >= m2)
        elif m1.amount > m2.amount:
            assert m1 > m2
            assert m1 >= m2
            assert not (m1 < m2)
            assert not (m1 <= m2)
        else:  # Equal amounts
            assert m1 <= m2
            assert m1 >= m2
            assert not (m1 < m2)
            assert not (m1 > m2)

    @given(money_strategy())
    def test_equality_reflexive(self, money: Money) -> None:
        """Money is equal to itself."""
        # Test reflexive property: a == a
        assert money == money  # noqa: PLR0124

    @given(same_currency_pair())
    def test_equality_symmetric(self, money_pair: tuple[Money, Money]) -> None:
        """If a == b, then b == a."""
        m1, m2 = money_pair
        if m1.amount == m2.amount:
            assert (m1 == m2) == (m2 == m1)
