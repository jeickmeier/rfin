"""Comprehensive parity tests for portfolio module.

Tests entities, positions, valuation, aggregation, and optimization.
"""

from datetime import date

from finstack.core.currency import EUR, USD
from finstack.core.dates import DayCount
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import DiscountCurve, FxMatrix, MarketContext
from finstack.valuations.instruments import Bond
import pytest

from finstack.portfolio import (
    Entity,
    PortfolioBuilder,
    Position,
    PositionUnit,
    value_portfolio,
)


class TestEntityParity:
    """Test entity operations match Rust."""

    def test_entity_construction(self) -> None:
        """Test entity construction."""
        entity = Entity("FUND-001").with_name("Test Fund").with_tags({"type": "hedge_fund", "strategy": "long_short"})

        assert entity.id == "FUND-001"
        assert entity.name == "Test Fund"
        assert entity.tags["type"] == "hedge_fund"

    def test_entity_minimal_construction(self) -> None:
        """Test entity with minimal fields."""
        entity = Entity("FUND-002").with_name("Minimal Fund")

        assert entity.id == "FUND-002"
        assert entity.name == "Minimal Fund"


class TestPositionParity:
    """Test position operations match Rust."""

    def test_position_construction(self) -> None:
        """Test position construction."""
        # Create simple bond
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

        position = Position(
            "POS-001",
            "FUND-001",
            bond.instrument_id,
            bond,
            1.0,
            PositionUnit.UNITS,
        )

        assert position.position_id == "POS-001"
        assert position.entity_id == "FUND-001"
        assert position.quantity == 1.0

    def test_position_with_tags(self) -> None:
        """Test position with tags."""
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

        position = (
            Position(
                "POS-001",
                "FUND-001",
                bond.instrument_id,
                bond,
                1.0,
                PositionUnit.UNITS,
            )
            .with_tag("rating", "AAA")
            .with_tag("sector", "government")
        )

        assert position.tags["rating"] == "AAA"
        assert position.tags["sector"] == "government"

    def test_position_negative_quantity(self) -> None:
        """Test position with negative quantity (short position)."""
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

        position = Position(
            "POS-SHORT",
            "FUND-001",
            bond.instrument_id,
            bond,
            -0.5,  # Short position
            PositionUnit.UNITS,
        )

        assert position.quantity == -0.5


class TestPortfolioBuilderParity:
    """Test portfolio builder matches Rust."""

    def test_builder_basic(self) -> None:
        """Test basic portfolio construction."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        # Add entity
        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

        portfolio = builder.build()

        assert portfolio.base_ccy.code == "USD"
        assert portfolio.as_of == date(2024, 1, 1)
        assert len(portfolio.entities) == 1

    def test_builder_with_positions(self) -> None:
        """Test portfolio with positions."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        # Add entity
        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

        # Add position
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

        position = Position(
            "POS-001",
            "FUND-001",
            bond.instrument_id,
            bond,
            1.0,
            PositionUnit.UNITS,
        )
        builder.position(position)

        portfolio = builder.build()

        assert len(portfolio.positions) == 1

    def test_builder_validation(self) -> None:
        """Test portfolio builder validation."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        # Add position without corresponding entity
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

        position = Position(
            "POS-001",
            "FUND-MISSING",  # Entity doesn't exist
            bond.instrument_id,
            bond,
            1.0,
            PositionUnit.UNITS,
        )
        builder.position(position)

        # Build should fail validation
        with pytest.raises(Exception, match=r"[Vv]alid|error"):
            builder.build()


class TestPortfolioValuationParity:
    """Test portfolio valuation matches Rust."""

    def test_value_portfolio_simple(self) -> None:
        """Test simple portfolio valuation."""
        # Create portfolio
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

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

        position = Position(
            "POS-001",
            "FUND-001",
            bond.instrument_id,
            bond,
            1.0,
            PositionUnit.UNITS,
        )
        builder.position(position)

        portfolio = builder.build()

        # Create market context
        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        # Value portfolio
        valuation = value_portfolio(portfolio, market)

        assert valuation is not None
        assert len(valuation.positions) == 1
        assert valuation.total.currency.code == "USD"

    def test_value_portfolio_multiple_positions(self) -> None:
        """Test valuation with multiple positions."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

        # Add two bond positions
        for i in range(2):
            bond = (
                Bond.builder(f"BOND-00{i + 1}")
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

            position = Position(
                f"POS-00{i + 1}",
                "FUND-001",
                bond.instrument_id,
                bond,
                1.0,
                PositionUnit.UNITS,
            )
            builder.position(position)

        portfolio = builder.build()

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        valuation = value_portfolio(portfolio, market)

        assert len(valuation.positions) == 2

    def test_value_portfolio_cross_currency(self) -> None:
        """Test portfolio with cross-currency positions."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

        # Add USD bond
        usd_bond = (
            Bond.builder("BOND-USD")
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

        builder.position(
            Position(
                "POS-USD",
                "FUND-001",
                usd_bond.instrument_id,
                usd_bond,
                1.0,
                PositionUnit.UNITS,
            )
        )

        # Add EUR bond
        eur_bond = (
            Bond.builder("BOND-EUR")
            .notional(1_000_000.0)
            .currency("EUR")
            .issue(date(2024, 1, 1))
            .maturity(date(2029, 1, 1))
            .coupon_rate(0.04)
            .frequency(Frequency.ANNUAL)
            .day_count(DayCount.ACT_365F)
            .disc_id("EUR-OIS")
            .build()
        )

        builder.position(
            Position(
                "POS-EUR",
                "FUND-001",
                eur_bond.instrument_id,
                eur_bond,
                1.0,
                PositionUnit.UNITS,
            )
        )

        portfolio = builder.build()

        # Create market context with FX
        market = MarketContext()

        usd_discount = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(usd_discount)

        eur_discount = DiscountCurve(
            "EUR-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.96), (5.0, 0.78)],
            day_count="act_365f",
        )
        market.insert_discount(eur_discount)

        # Add FX rate
        fx = FxMatrix()
        fx.set_quote(EUR, USD, 1.10)
        market.set_fx(fx)

        # Value portfolio
        valuation = value_portfolio(portfolio, market)

        # Should have two positions
        assert len(valuation.positions) == 2
        # Total should be in USD (base currency)
        assert valuation.total.currency.code == "USD"


class TestPortfolioAggregationParity:
    """Test portfolio aggregation matches Rust."""

    def test_aggregate_by_entity(self) -> None:
        """Test aggregation by entity."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        # Add two entities
        entity1 = Entity("FUND-001").with_name("Fund 1")
        entity2 = Entity("FUND-002").with_name("Fund 2")
        builder.entity(entity1)
        builder.entity(entity2)

        # Add positions to each entity
        for fund_id in ["FUND-001", "FUND-002"]:
            bond = (
                Bond.builder(f"BOND-{fund_id}")
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

            position = Position(
                f"POS-{fund_id}",
                fund_id,
                bond.instrument_id,
                bond,
                1.0,
                PositionUnit.UNITS,
            )
            builder.position(position)

        portfolio = builder.build()

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        valuation = value_portfolio(portfolio, market)

        # Should have entity-level aggregates
        assert len(valuation.entities) == 2

    def test_aggregate_by_attribute(self) -> None:
        """Test aggregation by attribute (tags)."""
        from finstack.portfolio import aggregate_by_attribute

        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Test Fund")
        builder.entity(entity)

        # Add positions with different ratings
        for rating in ["AAA", "AA"]:
            bond = (
                Bond.builder(f"BOND-{rating}")
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

            position = Position(
                f"POS-{rating}",
                "FUND-001",
                bond.instrument_id,
                bond,
                1.0,
                PositionUnit.UNITS,
            ).with_tag("rating", rating)
            builder.position(position)

        portfolio = builder.build()

        market = MarketContext()
        discount_curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        market.insert_discount(discount_curve)

        valuation = value_portfolio(portfolio, market)

        # Aggregate by rating
        aggregated = aggregate_by_attribute(valuation, portfolio.positions, "rating", USD)

        assert len(aggregated) == 2
        assert "AAA" in aggregated
        assert "AA" in aggregated


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_empty_portfolio(self) -> None:
        """Test empty portfolio valuation."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Empty Fund")
        builder.entity(entity)

        portfolio = builder.build()

        market = MarketContext()
        valuation = value_portfolio(portfolio, market)

        # Should succeed with zero positions
        assert len(valuation.positions) == 0

    def test_position_zero_quantity(self) -> None:
        """Test position with zero quantity."""
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

        position = Position(
            "POS-ZERO",
            "FUND-001",
            bond.instrument_id,
            bond,
            0.0,
            PositionUnit.UNITS,
        )

        assert position.quantity == 0.0

    def test_portfolio_single_entity_multiple_positions(self) -> None:
        """Test portfolio with one entity holding multiple positions."""
        builder = PortfolioBuilder("TEST_PORTFOLIO")
        builder.base_ccy("USD")
        builder.as_of(date(2024, 1, 1))

        entity = Entity("FUND-001").with_name("Diversified Fund")
        builder.entity(entity)

        # Add 10 positions
        for i in range(10):
            bond = (
                Bond.builder(f"BOND-{i:03d}")
                .notional(100_000.0)
                .currency("USD")
                .issue(date(2024, 1, 1))
                .maturity(date(2029, 1, 1))
                .coupon_rate(0.05)
                .frequency(Frequency.SEMI_ANNUAL)
                .day_count(DayCount.THIRTY_360)
                .disc_id("USD-OIS")
                .build()
            )

            position = Position(
                f"POS-{i:03d}",
                "FUND-001",
                bond.instrument_id,
                bond,
                1.0,
                PositionUnit.UNITS,
            )
            builder.position(position)

        portfolio = builder.build()

        assert len(portfolio.positions) == 10


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
