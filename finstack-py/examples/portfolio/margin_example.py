"""Portfolio Margin and Netting Example.

This example demonstrates:
1. Creating netting sets for bilateral and cleared trades
2. Organizing positions into netting sets
3. Calculating initial and variation margin (SIMM)
4. Aggregating margin requirements at portfolio level
5. Splitting margin between cleared and bilateral

Note: This example shows the API structure. Full margin calculation requires:
- Instruments implementing the Marginable trait
- Proper netting set assignment to instruments
- Complete market data for SIMM sensitivities
"""

from datetime import date

# Core imports
from finstack.core import Currency, Money, Date, MarketContext

# Portfolio imports
from finstack.portfolio import (
    Portfolio,
    PortfolioBuilder,
    Entity,
    Position,
    PositionUnit,
    NettingSetId,
    NettingSet,
    NettingSetManager,
    PortfolioMarginAggregator,
)

# Instrument imports (for demonstration)
from finstack.valuations.instruments.builders import (
    InterestRateSwapBuilder,
    CreditDefaultSwapBuilder,
)

# Market data imports
from finstack.valuations.market_data import DiscountCurve, HazardCurve


def create_market_context():
    """Create a market context with discount and credit curves."""
    usd = Currency.from_code("USD")
    as_of = Date(2024, 6, 15)
    market = MarketContext()
    
    # Create discount curve for OIS
    tenors = ["1D", "1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y", "30Y"]
    rates = [0.0520, 0.0525, 0.0530, 0.0535, 0.0545, 0.0565, 0.0615, 0.0655, 0.0695]
    discount_curve = DiscountCurve.from_par_rates(
        "USD.OIS",
        as_of,
        tenors,
        rates,
        usd,
        "Act360",
        "Linear"
    )
    market.insert_discount(discount_curve)
    
    # Create hazard curve for credit
    cds_tenors = ["6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y"]
    cds_spreads = [0.0100, 0.0120, 0.0150, 0.0180, 0.0220, 0.0250, 0.0280]
    hazard_curve = HazardCurve.from_cds_spreads(
        "ACME.5Y",
        as_of,
        cds_tenors,
        cds_spreads,
        0.40,  # recovery rate
        usd,
        "USD.OIS",
        "Act360"
    )
    market.insert_hazard(hazard_curve)
    
    return market, as_of, usd


# ============================================================================
# Example 1: Creating Netting Sets
# ============================================================================

def example1_netting_sets():
    """Example 1: Create and manage netting sets."""
    print("=" * 80)
    print("Example 1: Creating Netting Sets")
    print("=" * 80)
    
    # Create bilateral netting set (OTC with CSA)
    bilateral_id = NettingSetId.bilateral("JPMORGAN", "CSA_2024_001")
    bilateral_ns = NettingSet(bilateral_id)
    
    # Create cleared netting set (CCP)
    cleared_id = NettingSetId.cleared("LCH")
    cleared_ns = NettingSet(cleared_id)
    
    print(f"\nBilateral Netting Set:")
    print(f"  ID: {bilateral_id}")
    print(f"  Counterparty: {bilateral_id.counterparty_id}")
    print(f"  CSA: {bilateral_id.csa_id}")
    print(f"  Is Cleared: {bilateral_id.is_cleared()}")
    
    print(f"\nCleared Netting Set:")
    print(f"  ID: {cleared_id}")
    print(f"  CCP: {cleared_id.ccp_id}")
    print(f"  Is Cleared: {cleared_id.is_cleared()}")
    
    # Add positions to netting sets
    bilateral_ns.add_position("IRS_001")
    bilateral_ns.add_position("IRS_002")
    bilateral_ns.add_position("CDS_001")
    
    cleared_ns.add_position("IRS_003")
    cleared_ns.add_position("IRS_004")
    cleared_ns.add_position("IRS_005")
    
    print(f"\nBilateral positions: {bilateral_ns.position_count()}")
    print(f"Cleared positions: {cleared_ns.position_count()}")


# ============================================================================
# Example 2: Using Netting Set Manager
# ============================================================================

def example2_netting_set_manager():
    """Example 2: Use NettingSetManager to organize positions."""
    print("\n" + "=" * 80)
    print("Example 2: Netting Set Manager")
    print("=" * 80)
    
    # Create manager
    manager = NettingSetManager()
    
    # Set default netting set for positions without explicit assignment
    default_id = NettingSetId.bilateral("HOUSE_ACCOUNT", "CSA_DEFAULT")
    manager = manager.with_default_set(default_id)
    
    print(f"\nNetting Set Manager:")
    print(f"  Total netting sets: {manager.count()}")
    print(f"  Netting set IDs:")
    
    for ns_id in manager.ids():
        print(f"    - {ns_id}")
        ns = manager.get(ns_id)
        if ns:
            print(f"      Positions: {ns.position_count()}")
            print(f"      Cleared: {ns.is_cleared()}")


# ============================================================================
# Example 3: Portfolio Margin Aggregator
# ============================================================================

def example3_margin_aggregator():
    """Example 3: Create and use portfolio margin aggregator."""
    print("\n" + "=" * 80)
    print("Example 3: Portfolio Margin Aggregator")
    print("=" * 80)
    
    market, as_of, usd = create_market_context()
    
    # Create portfolio
    builder = PortfolioBuilder()
    builder.base_ccy(usd)
    builder.as_of(as_of)
    
    # Add entities
    builder.entity(Entity("JPMORGAN", "JPMorgan Chase"))
    builder.entity(Entity("LCH", "LCH Clearnet"))
    
    # Build portfolio
    portfolio = builder.build()
    
    # Create margin aggregator from portfolio
    aggregator = PortfolioMarginAggregator.from_portfolio(portfolio)
    
    print(f"\nMargin Aggregator Created:")
    print(f"  Base Currency: {usd.code}")
    print(f"  Portfolio Entities: {len(portfolio.entities)}")
    print(f"  Portfolio Positions: {len(portfolio.positions)}")
    
    print("\nNote: To calculate margin, you need:")
    print("  1. Positions with marginable instruments (IRS, CDS, etc.)")
    print("  2. Instruments with netting set assignments")
    print("  3. Complete market data for SIMM sensitivities")
    print("\nExample calculation would look like:")
    print("  result = aggregator.calculate(portfolio, market_context, as_of)")
    print("  print(f'Total IM: {result.total_initial_margin}')")
    print("  print(f'Total VM: {result.total_variation_margin}')")
    print("  for ns_id, ns_margin in result.by_netting_set.items():")
    print("      print(f'  {ns_id}: IM={ns_margin.initial_margin}')")


# ============================================================================
# Example 4: Margin Workflow (Conceptual)
# ============================================================================

def example4_margin_workflow():
    """Example 4: Complete margin workflow (conceptual)."""
    print("\n" + "=" * 80)
    print("Example 4: Margin Calculation Workflow (Conceptual)")
    print("=" * 80)
    
    print("""
Typical Margin Calculation Workflow:

1. **Setup Portfolio**
   - Create entities (counterparties, CCPs)
   - Create positions with marginable instruments
   - Assign positions to entities

2. **Create Netting Sets**
   - Define bilateral netting sets (counterparty + CSA)
   - Define cleared netting sets (CCP)
   - Assign positions to appropriate netting sets

3. **Build Market Context**
   - Discount curves (for discounting cashflows)
   - Credit curves (for CDS pricing)
   - Volatility surfaces (for options)
   - FX rates (for cross-currency positions)

4. **Calculate Margin**
   - Create PortfolioMarginAggregator
   - Call aggregator.calculate(portfolio, market, as_of)
   - Returns PortfolioMarginResult with:
     * Initial Margin (IM) by netting set
     * Variation Margin (VM) by netting set
     * SIMM sensitivities and breakdown
     * Cleared vs bilateral split

5. **Analyze Results**
   - Review IM and VM by netting set
   - Check IM breakdown by risk class
   - Identify concentration risks
   - Report margin utilization

Example Result Structure:
```
PortfolioMarginResult(
    as_of=2024-06-15,
    base_currency=USD,
    total_initial_margin=Money(15,234,567.89, USD),
    total_variation_margin=Money(1,234,567.89, USD),
    by_netting_set={
        'bilateral:JPMORGAN:CSA_2024_001': NettingSetMargin(
            initial_margin=Money(8,500,000.00, USD),
            variation_margin=Money(750,000.00, USD),
            position_count=15,
            im_methodology='SIMM',
            im_breakdown={
                'InterestRate': Money(6,200,000.00, USD),
                'CreditQualifying': Money(1,800,000.00, USD),
                'Equity': Money(500,000.00, USD),
            }
        ),
        'cleared:LCH': NettingSetMargin(
            initial_margin=Money(6,734,567.89, USD),
            variation_margin=Money(484,567.89, USD),
            position_count=22,
            im_methodology='ClearingHouse'
        )
    },
    total_positions=37,
    positions_without_margin=3
)
```

Cleared vs Bilateral Split:
- Cleared Margin: $7,219,135.78
- Bilateral Margin: $9,250,000.00
    """)


# ============================================================================
# Example 5: CSA Terms
# ============================================================================

def example5_csa_terms():
    """Example 5: Common CSA terms affecting margin."""
    print("\n" + "=" * 80)
    print("Example 5: Standard CSA Terms")
    print("=" * 80)
    
    print("""
Common Credit Support Annex (CSA) Terms:

1. **Threshold**
   - Minimum exposure before collateral is posted
   - Example: $10 million threshold means no collateral until
     exposure exceeds $10M

2. **Minimum Transfer Amount (MTA)**
   - Minimum amount of collateral movement
   - Example: $500k MTA prevents small daily transfers

3. **Independent Amount (IA)**
   - Upfront collateral regardless of exposure
   - Similar to initial margin

4. **Eligible Collateral**
   - Cash (USD, EUR, etc.)
   - Government securities
   - Investment grade bonds
   - Haircuts applied to non-cash collateral

5. **Valuation Agent**
   - Party responsible for daily MTM calculations
   - Usually the dealer in dealer-client CSAs

6. **Dispute Resolution**
   - Process for resolving valuation disputes
   - Typically requires recalculation by agreed third party

7. **Rehypothecation Rights**
   - Whether posted collateral can be reused
   - Important for liquidity management

Standard Bilateral CSA Example:
```
NettingSetId.bilateral(
    counterparty_id="JPMORGAN",
    csa_id="CSA_2024_001"
)

# CSA Terms (stored in margin spec):
- Threshold: $10,000,000
- MTA: $500,000
- IA: $0
- Eligible Collateral: Cash USD, US Treasuries (98% haircut)
- Valuation Agent: Dealer
- Frequency: Daily
```

Cleared Margin Example:
```
NettingSetId.cleared(ccp_id="LCH")

# Clearing Terms:
- Initial Margin: Calculated by CCP (e.g., SPAN, SIMM)
- Variation Margin: Daily settlement
- No Threshold
- No MTA
- Cash only
```
    """)


# ============================================================================
# Main
# ============================================================================

def main():
    """Run all examples."""
    example1_netting_sets()
    example2_netting_set_manager()
    example3_margin_aggregator()
    example4_margin_workflow()
    example5_csa_terms()
    
    print("\n" + "=" * 80)
    print("Summary")
    print("=" * 80)
    print("""
The finstack margin module provides:
✓ Netting set management (bilateral and cleared)
✓ Position organization by netting set
✓ Portfolio-level margin aggregation
✓ SIMM sensitivity calculation
✓ Initial and variation margin reporting
✓ Risk breakdown by asset class

For production use, ensure:
1. Instruments implement Marginable trait
2. Proper netting set assignments
3. Complete market data (curves, vols, FX)
4. CSA terms configured correctly
5. Regular margin reconciliation with counterparties
    """)


if __name__ == "__main__":
    main()
