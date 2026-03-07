"""Bulk pricing parity tests.

Tests that pricing many instruments in bulk produces identical results
across multiple invocations and verifies consistency with Rust implementation.
"""

from datetime import date

from finstack.core.market_data import MarketContext
from finstack.valuations.pricer import create_standard_registry
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    create_flat_market_context,
    create_test_bond,
    create_test_deposit,
    create_test_swap,
)


@pytest.mark.parity
@pytest.mark.bulk
class TestBulkBondPricingParity:
    """Test bulk bond pricing consistency."""

    @pytest.fixture
    def bond_portfolio(self) -> list:
        """Create portfolio of bonds with varying parameters."""
        bonds = []
        base_date = date(2024, 1, 1)
        for i in range(20):
            notional = 1_000_000.0 * (i + 1)
            coupon = 0.02 + (i * 0.005)  # 2% to 11.5%
            tenor = 1 + (i % 10)  # 1 to 10 years
            bonds.append(
                create_test_bond(
                    bond_id=f"BOND-{i:03d}",
                    notional=notional,
                    coupon_rate=coupon,
                    issue=base_date,
                    maturity=date(2024 + tenor, 1, 1),
                )
            )
        return bonds

    @pytest.fixture
    def market(self) -> MarketContext:
        """Create market context for bond pricing."""
        return create_flat_market_context(discount_rate=0.05)

    def test_bulk_pricing_deterministic(self, bond_portfolio: list, market: MarketContext) -> None:
        """Price portfolio twice, verify identical results."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        results1 = [registry.get_price(b, "discounting", market, as_of) for b in bond_portfolio]
        results2 = [registry.get_price(b, "discounting", market, as_of) for b in bond_portfolio]

        for i, (r1, r2) in enumerate(zip(results1, results2, strict=True)):
            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC, (
                f"Bond {i}: {r1.value.amount} != {r2.value.amount}"
            )
            assert r1.value.currency.code == r2.value.currency.code

    def test_bulk_pricing_order_independent(self, bond_portfolio: list, market: MarketContext) -> None:
        """Pricing order should not affect individual results."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        # Price in original order
        forward = [registry.get_price(b, "discounting", market, as_of) for b in bond_portfolio]

        # Price in reverse order
        backward = [registry.get_price(b, "discounting", market, as_of) for b in reversed(bond_portfolio)]
        backward.reverse()

        for i, (f, b) in enumerate(zip(forward, backward, strict=True)):
            assert abs(f.value.amount - b.value.amount) < TOLERANCE_DETERMINISTIC, (
                f"Bond {i}: forward={f.value.amount}, backward={b.value.amount}"
            )

    def test_bulk_pricing_with_varying_curves(self, bond_portfolio: list) -> None:
        """Same bonds priced against different curves produce consistent results."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        # Two different market scenarios
        market_low = create_flat_market_context(discount_rate=0.03)
        market_high = create_flat_market_context(discount_rate=0.07)

        results_low = [registry.get_price(b, "discounting", market_low, as_of) for b in bond_portfolio]
        results_high = [registry.get_price(b, "discounting", market_high, as_of) for b in bond_portfolio]

        # Higher rates should produce lower NPVs for positive coupon bonds
        for i, (low, high) in enumerate(zip(results_low, results_high, strict=True)):
            assert low.value.amount > high.value.amount, (
                f"Bond {i}: lower rate NPV ({low.value.amount}) should exceed higher rate NPV ({high.value.amount})"
            )

    def test_bulk_pricing_stability_multiple_runs(self, bond_portfolio: list, market: MarketContext) -> None:
        """Price portfolio 5 times, all results should match."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        all_results = []
        for _ in range(5):
            results = [registry.get_price(b, "discounting", market, as_of).value.amount for b in bond_portfolio]
            all_results.append(results)

        # Compare all runs to the first
        for run_idx in range(1, 5):
            for bond_idx in range(len(bond_portfolio)):
                assert abs(all_results[0][bond_idx] - all_results[run_idx][bond_idx]) < TOLERANCE_DETERMINISTIC, (
                    f"Run {run_idx}, Bond {bond_idx}: {all_results[0][bond_idx]} != {all_results[run_idx][bond_idx]}"
                )


@pytest.mark.parity
@pytest.mark.bulk
class TestBulkSwapPricingParity:
    """Test bulk swap pricing consistency."""

    @pytest.fixture
    def swap_portfolio(self) -> list:
        """Create portfolio of swaps with varying parameters."""
        swaps = []
        for i in range(10):
            notional = 10_000_000.0 * (i + 1)
            fixed_rate = 0.03 + (i * 0.01)  # 3% to 12%
            tenor = 1 + (i % 5)  # 1 to 5 years
            swaps.append(
                create_test_swap(
                    swap_id=f"IRS-{i:03d}",
                    notional=notional,
                    fixed_rate=fixed_rate,
                    maturity=date(2024 + tenor, 1, 1),
                )
            )
        return swaps

    @pytest.fixture
    def market(self) -> MarketContext:
        """Create market context for swap pricing."""
        return create_flat_market_context(discount_rate=0.05, forward_rate=0.05)

    def test_bulk_swap_pricing_deterministic(self, swap_portfolio: list, market: MarketContext) -> None:
        """Price swap portfolio twice, verify identical results."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        results1 = [registry.get_price(s, "discounting", market, as_of) for s in swap_portfolio]
        results2 = [registry.get_price(s, "discounting", market, as_of) for s in swap_portfolio]

        for i, (r1, r2) in enumerate(zip(results1, results2, strict=True)):
            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC, (
                f"Swap {i}: {r1.value.amount} != {r2.value.amount}"
            )

    def test_bulk_swap_with_forward_curve_bumps(self, swap_portfolio: list) -> None:
        """Verify swap NPVs respond consistently to forward curve changes."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        # Base market
        market_base = create_flat_market_context(discount_rate=0.05, forward_rate=0.05)
        # Bumped forward curve (higher floating rates)
        market_bumped = create_flat_market_context(discount_rate=0.05, forward_rate=0.06)

        results_base = [registry.get_price(s, "discounting", market_base, as_of) for s in swap_portfolio]
        results_bumped = [registry.get_price(s, "discounting", market_bumped, as_of) for s in swap_portfolio]

        # For a payer swap (pay fixed, receive floating), higher forward rates increase NPV
        # The effect depends on the fixed rate relative to the forward rate
        # Just verify results are different and consistent
        for i, (base, bumped) in enumerate(zip(results_base, results_bumped, strict=True)):
            # Results should be different (forward rate changed)
            assert abs(base.value.amount - bumped.value.amount) > 1.0, (
                f"Swap {i}: NPV should change when forward rate bumped"
            )


@pytest.mark.parity
@pytest.mark.bulk
class TestBulkDepositPricingParity:
    """Test bulk deposit pricing consistency."""

    @pytest.fixture
    def deposit_portfolio(self) -> list:
        """Create portfolio of deposits with varying parameters."""
        deposits = []
        base_date = date(2024, 1, 1)
        for i in range(15):
            notional = 1_000_000.0 * (i + 1)
            tenor_days = 30 * (i + 1)  # 30 to 450 days
            rate = 0.02 + (i * 0.005)  # 2% to 9%
            end_date = date(2024, 1, 1 + tenor_days % 28)
            # Adjust for month overflow
            month_offset = tenor_days // 30
            year_offset = month_offset // 12
            month = 1 + (month_offset % 12)
            if month > 12:
                month = month - 12
                year_offset += 1
            end_date = date(2024 + year_offset, month, min(28, 1 + (tenor_days % 28)))

            deposits.append(
                create_test_deposit(
                    deposit_id=f"DEP-{i:03d}",
                    notional=notional,
                    start=base_date,
                    end=end_date,
                    quote_rate=rate,
                )
            )
        return deposits

    @pytest.fixture
    def market(self) -> MarketContext:
        """Create market context for deposit pricing."""
        return create_flat_market_context(discount_rate=0.05)

    def test_bulk_deposit_pricing_deterministic(self, deposit_portfolio: list, market: MarketContext) -> None:
        """Price deposit portfolio twice, verify identical results."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        results1 = [registry.get_price(d, "discounting", market, as_of) for d in deposit_portfolio]
        results2 = [registry.get_price(d, "discounting", market, as_of) for d in deposit_portfolio]

        for i, (r1, r2) in enumerate(zip(results1, results2, strict=True)):
            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC, (
                f"Deposit {i}: {r1.value.amount} != {r2.value.amount}"
            )


@pytest.mark.parity
@pytest.mark.bulk
class TestMixedPortfolioParity:
    """Test pricing mixed portfolios (bonds, swaps, deposits)."""

    @pytest.fixture
    def mixed_portfolio(self) -> list:
        """Create a mixed portfolio of different instrument types."""
        instruments = []
        base_date = date(2024, 1, 1)

        # Add bonds
        instruments.extend([
            create_test_bond(
                bond_id=f"BOND-{i:03d}",
                notional=1_000_000.0 * (i + 1),
                coupon_rate=0.03 + (i * 0.01),
                issue=base_date,
                maturity=date(2024 + i + 1, 1, 1),
            )
            for i in range(5)
        ])

        # Add swaps
        instruments.extend([
            create_test_swap(
                swap_id=f"IRS-{i:03d}",
                notional=10_000_000.0 * (i + 1),
                fixed_rate=0.04 + (i * 0.01),
                maturity=date(2024 + i + 2, 1, 1),
            )
            for i in range(5)
        ])

        # Add deposits
        instruments.extend([
            create_test_deposit(
                deposit_id=f"DEP-{i:03d}",
                notional=500_000.0 * (i + 1),
                start=base_date,
                end=date(2024, 3 + i, 1),
                quote_rate=0.02 + (i * 0.005),
            )
            for i in range(5)
        ])

        return instruments

    @pytest.fixture
    def market(self) -> MarketContext:
        """Create complete market context."""
        return create_flat_market_context(discount_rate=0.05, forward_rate=0.05)

    def test_mixed_portfolio_pricing_stable(self, mixed_portfolio: list, market: MarketContext) -> None:
        """Price mixed portfolio multiple times, verify stability."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        # Price 3 times
        all_results: list[dict] = []
        for _ in range(3):
            results = {}
            for inst in mixed_portfolio:
                result = registry.get_price(inst, "discounting", market, as_of)
                results[inst.instrument_id] = result.value.amount
            all_results.append(results)

        # All runs should match
        for inst_id in all_results[0]:
            for run_idx in range(1, len(all_results)):
                assert abs(all_results[0][inst_id] - all_results[run_idx][inst_id]) < TOLERANCE_DETERMINISTIC, (
                    f"Instrument {inst_id}, run {run_idx}: {all_results[0][inst_id]} != {all_results[run_idx][inst_id]}"
                )

    def test_portfolio_individual_vs_batch_order(self, mixed_portfolio: list, market: MarketContext) -> None:
        """Verify individual pricing matches regardless of portfolio order."""
        registry = create_standard_registry()
        as_of = date(2024, 1, 1)

        # Price each instrument
        results_original = {
            inst.instrument_id: registry.get_price(inst, "discounting", market, as_of).value.amount
            for inst in mixed_portfolio
        }

        # Shuffle and price again
        import random

        shuffled = mixed_portfolio.copy()
        random.seed(42)
        random.shuffle(shuffled)

        results_shuffled = {
            inst.instrument_id: registry.get_price(inst, "discounting", market, as_of).value.amount for inst in shuffled
        }

        # Results should match by instrument ID
        for inst_id, original_value in results_original.items():
            shuffled_value = results_shuffled[inst_id]
            assert abs(original_value - shuffled_value) < TOLERANCE_DETERMINISTIC, (
                f"Instrument {inst_id}: original={original_value}, shuffled={shuffled_value}"
            )
