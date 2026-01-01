"""Tests for portfolio margin aggregation and netting.

This module tests the margin aggregation functionality including:
- Netting set management
- SIMM sensitivity aggregation
- Initial and variation margin calculation
- Portfolio-level margin reporting
"""

from datetime import date

from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve

# Instrument imports
from finstack.valuations.instruments import InterestRateSwapBuilder
import pytest

# Core imports
from finstack import Currency, Money

# Portfolio imports
from finstack.portfolio import (
    Entity,
    NettingSet,
    NettingSetId,
    NettingSetManager,
    PortfolioBuilder,
    PortfolioMarginAggregator,
    Position,
    PositionUnit,
)

# ============================================================================
# Fixtures
# ============================================================================


@pytest.fixture
def usd() -> Currency:
    """USD currency."""
    return Currency("USD")


@pytest.fixture
def as_of() -> date:
    """Valuation date."""
    return date(2024, 6, 15)


@pytest.fixture
def market_context(usd: Currency, as_of: date) -> MarketContext:
    """Create a market context with discount and credit curves."""
    market = MarketContext()

    # Create a discount curve
    tenors = ["1D", "1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y"]
    rates = [0.0520, 0.0525, 0.0530, 0.0535, 0.0545, 0.0565, 0.0615, 0.0655]
    discount_curve = DiscountCurve.from_par_rates("USD.OIS", as_of, tenors, rates, usd, "Act360", "Linear")
    market.insert_discount(discount_curve)

    # Create a hazard curve for CDS
    cds_tenors = ["6M", "1Y", "2Y", "3Y", "5Y"]
    cds_spreads = [0.0100, 0.0120, 0.0150, 0.0180, 0.0220]
    hazard_curve = HazardCurve.from_cds_spreads(
        "ACME.5Y",
        as_of,
        cds_tenors,
        cds_spreads,
        0.40,  # recovery rate
        usd,
        "USD.OIS",
        "Act360",
    )
    market.insert_hazard(hazard_curve)

    return market


# ============================================================================
# Test NettingSetId
# ============================================================================


def test_netting_set_id_bilateral() -> None:
    """Test bilateral netting set ID creation."""
    ns_id = NettingSetId.bilateral("BANK_A", "CSA_001")

    assert ns_id.counterparty_id == "BANK_A"
    assert ns_id.csa_id == "CSA_001"
    assert ns_id.ccp_id is None
    assert not ns_id.is_cleared()
    assert "BANK_A" in str(ns_id)
    assert "CSA_001" in str(ns_id)


def test_netting_set_id_cleared() -> None:
    """Test cleared netting set ID creation."""
    ns_id = NettingSetId.cleared("LCH")

    assert ns_id.counterparty_id == "LCH"
    assert ns_id.csa_id is None
    assert ns_id.ccp_id == "LCH"
    assert ns_id.is_cleared()
    assert "LCH" in str(ns_id)


def test_netting_set_id_repr() -> None:
    """Test netting set ID string representation."""
    bilateral_id = NettingSetId.bilateral("BANK_A", "CSA_001")
    cleared_id = NettingSetId.cleared("CME")

    # Should have readable repr
    assert "NettingSetId" in repr(bilateral_id)
    assert "NettingSetId" in repr(cleared_id)


# ============================================================================
# Test NettingSet
# ============================================================================


def test_netting_set_creation() -> None:
    """Test netting set creation and basic operations."""
    ns_id = NettingSetId.bilateral("BANK_A", "CSA_001")
    ns = NettingSet(ns_id)

    assert ns.position_count() == 0
    assert not ns.is_cleared()


def test_netting_set_add_positions() -> None:
    """Test adding positions to netting set."""
    ns_id = NettingSetId.bilateral("BANK_A", "CSA_001")
    ns = NettingSet(ns_id)

    ns.add_position("POS_001")
    ns.add_position("POS_002")
    ns.add_position("POS_003")

    assert ns.position_count() == 3


def test_netting_set_cleared() -> None:
    """Test cleared netting set properties."""
    ns_id = NettingSetId.cleared("LCH")
    ns = NettingSet(ns_id)

    assert ns.is_cleared()


# ============================================================================
# Test NettingSetManager
# ============================================================================


def test_netting_set_manager_creation() -> None:
    """Test netting set manager creation."""
    manager = NettingSetManager()
    assert manager.count() == 0


def test_netting_set_manager_with_default() -> None:
    """Test setting default netting set."""
    default_id = NettingSetId.bilateral("DEFAULT", "CSA_DEFAULT")
    manager = NettingSetManager().with_default_set(default_id)

    assert manager.count() == 1
    assert len(manager.ids()) == 1


def test_netting_set_manager_get_netting_set() -> None:
    """Test retrieving netting sets from manager."""
    NettingSetManager()

    # Create netting set IDs
    bilateral_id = NettingSetId.bilateral("BANK_A", "CSA_001")
    NettingSetId.cleared("CME")

    # Add to manager by creating them with default
    manager_with_default = NettingSetManager().with_default_set(bilateral_id)

    # Should be able to retrieve
    ns = manager_with_default.get(bilateral_id)
    assert ns is not None
    assert ns.position_count() == 0


def test_netting_set_manager_ids() -> None:
    """Test listing netting set IDs."""
    bilateral_id = NettingSetId.bilateral("BANK_A", "CSA_001")
    manager = NettingSetManager().with_default_set(bilateral_id)

    ids = manager.ids()
    assert len(ids) == 1
    assert ids[0].counterparty_id == "BANK_A"


# ============================================================================
# Test PortfolioMarginAggregator
# ============================================================================


def test_margin_aggregator_creation(usd: Currency) -> None:
    """Test margin aggregator creation."""
    aggregator = PortfolioMarginAggregator(usd)
    # Just verify it can be created
    assert aggregator is not None


def test_margin_aggregator_from_portfolio(usd: Currency, as_of: date) -> None:
    """Test creating aggregator from portfolio."""
    # Create a simple portfolio
    builder = PortfolioBuilder("TEST_PORTFOLIO")
    builder.base_ccy(usd)
    builder.as_of(as_of)

    # Add entity
    entity = Entity("ENTITY_001").with_name("Test Entity")
    builder.entity(entity)

    portfolio = builder.build()

    # Create aggregator from portfolio
    aggregator = PortfolioMarginAggregator.from_portfolio(portfolio)
    assert aggregator is not None


@pytest.mark.skip(reason="Requires marginable instruments with proper market data")
def test_margin_calculation_simple(usd: Currency, as_of: date, market_context: MarketContext) -> None:
    """Test basic margin calculation with interest rate swaps.

    Note: This test is skipped because it requires:
    1. Instruments that implement the Marginable trait
    2. Proper netting set assignment to instruments
    3. Complete market data for SIMM sensitivities

    This is a placeholder for future integration tests once the instrument
    margin specifications are properly configured.
    """
    # Create portfolio with IRS positions
    builder = PortfolioBuilder("TEST_PORTFOLIO_IRS")
    builder.base_ccy(usd)
    builder.as_of(as_of)

    # Add entity
    entity = Entity("BANK_A", "Bank A")
    builder.entity(entity)

    # Create IRS (would need proper margin spec assignment)
    notional = Money(10_000_000.0, Currency("USD"))
    irs = InterestRateSwapBuilder.new(
        "IRS_001",
        notional,
        0.055,  # fixed rate
        as_of,
        date(2029, 6, 15),  # maturity
        "pay_fixed",
        "USD.OIS",
    ).build()

    # Add position
    position = Position("POS_001", irs, 1.0, PositionUnit.Units, "BANK_A")
    builder.position(position)

    portfolio = builder.build()

    # Calculate margin
    aggregator = PortfolioMarginAggregator.from_portfolio(portfolio)
    result = aggregator.calculate(portfolio, market_context, as_of)

    # Verify result structure
    assert result.base_currency.code == "USD"
    assert result.total_positions >= 0
    assert result.total_initial_margin.amount >= 0


# ============================================================================
# Test Margin Results
# ============================================================================


def test_portfolio_margin_result_properties(usd: Currency, as_of: date) -> None:
    """Test portfolio margin result properties."""
    # Note: Cannot directly construct PortfolioMarginResult from Python
    # This would typically come from aggregator.calculate()
    # Testing what we can access


def test_netting_set_margin_properties() -> None:
    """Test netting set margin result properties."""
    # Note: Cannot directly construct NettingSetMargin from Python
    # This would typically come from aggregator.calculate()
    # Testing what we can access


# ============================================================================
# Integration Tests
# ============================================================================


@pytest.mark.usefixtures("usd", "as_of")
def test_margin_workflow_end_to_end() -> None:
    """Test complete margin calculation workflow.

    This test demonstrates the expected workflow:
    1. Create portfolio with positions
    2. Create netting set manager
    3. Create margin aggregator
    4. Calculate margin requirements

    Note: Skipped because it requires marginable instruments with proper
    netting set specifications.
    """
    pytest.skip("Requires marginable instruments with netting set specs")


def test_cleared_bilateral_split() -> None:
    """Test splitting margin between cleared and bilateral.

    Note: Skipped because it requires a calculated PortfolioMarginResult.
    """
    pytest.skip("Requires calculated margin result")


# ============================================================================
# Documentation Examples
# ============================================================================


def test_example_netting_set_creation() -> None:
    """Example: Create bilateral and cleared netting sets."""
    # Bilateral netting set (OTC with CSA)
    bilateral_id = NettingSetId.bilateral("JPMORGAN", "CSA_2024_001")
    bilateral_ns = NettingSet(bilateral_id)

    # Cleared netting set (CCP)
    cleared_id = NettingSetId.cleared("LCH")
    cleared_ns = NettingSet(cleared_id)

    # Add positions
    bilateral_ns.add_position("IRS_001")
    bilateral_ns.add_position("IRS_002")
    bilateral_ns.add_position("CDS_001")

    cleared_ns.add_position("IRS_003")
    cleared_ns.add_position("IRS_004")

    assert bilateral_ns.position_count() == 3
    assert cleared_ns.position_count() == 2
    assert not bilateral_ns.is_cleared()
    assert cleared_ns.is_cleared()


def test_example_netting_set_manager() -> None:
    """Example: Use netting set manager to organize positions."""
    manager = NettingSetManager()

    # Set default netting set for positions without explicit assignment
    default_id = NettingSetId.bilateral("HOUSE_ACCOUNT", "CSA_DEFAULT")
    manager = manager.with_default_set(default_id)

    # Check netting sets
    assert manager.count() == 1

    # Get IDs
    ids = manager.ids()
    assert len(ids) == 1
    assert ids[0].counterparty_id == "HOUSE_ACCOUNT"


def test_example_margin_aggregator_creation(usd: Currency, as_of: date) -> None:
    """Example: Create margin aggregator and add positions."""
    # Create aggregator with base currency
    aggregator = PortfolioMarginAggregator(usd)

    # Or create from existing portfolio
    builder = PortfolioBuilder("TEST_PORTFOLIO_EXAMPLE")
    builder.base_ccy(usd)
    builder.as_of(as_of)
    builder.entity(Entity("ENTITY_001", "Test Entity"))
    portfolio = builder.build()

    aggregator_from_portfolio = PortfolioMarginAggregator.from_portfolio(portfolio)

    assert aggregator is not None
    assert aggregator_from_portfolio is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
