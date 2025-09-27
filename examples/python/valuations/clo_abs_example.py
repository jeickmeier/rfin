"""
CLO/ABS Structured Credit Example

This example demonstrates how to create and value Collateralized Loan Obligations (CLOs)
and Asset-Backed Securities (ABS) using the finstack library.

Key features demonstrated:
- Creating asset pools from existing loans
- Defining tranche structures with attachment/detachment points
- Setting up coverage tests and waterfall logic
- Calculating present values and risk metrics
"""

import finstack
from finstack import Money, Currency, Date
from datetime import datetime, timedelta
import sys

def create_sample_loan_pool():
    """Create a sample pool of loan assets for CLO structuring."""
    
    # Base date for the structure
    base_date = Date.from_calendar_date(2025, 1, 15)
    
    # Create some sample loans with different characteristics
    loans = []
    
    # Technology sector loans
    for i in range(5):
        loan = finstack.Loan.fixed_rate(
            id=f"TECH_LOAN_{i+1}",
            amount=Money.new(50_000_000.0, Currency.USD),
            fixed_rate=0.08 + (i * 0.005),  # 8% to 10%
            issue_date=base_date,
            maturity_date=base_date.add_years(5)
        ).with_borrower(f"TechCorp_{i+1}")
        loans.append(loan)
    
    # Healthcare sector loans  
    for i in range(3):
        loan = finstack.Loan.floating_sofr(
            id=f"HEALTH_LOAN_{i+1}",
            amount=Money.new(75_000_000.0, Currency.USD),
            spread_bp=500.0 + (i * 50),  # SOFR + 500-600 bps
            issue_date=base_date,
            maturity_date=base_date.add_years(6)
        ).with_borrower(f"HealthCorp_{i+1}")
        loans.append(loan)
    
    # Energy sector loans
    for i in range(2):
        loan = finstack.Loan.fixed_rate(
            id=f"ENERGY_LOAN_{i+1}",
            amount=Money.new(100_000_000.0, Currency.USD),
            fixed_rate=0.12 + (i * 0.01),  # 12% to 13%
            issue_date=base_date,
            maturity_date=base_date.add_years(4)
        ).with_borrower(f"EnergyCorp_{i+1}")
        loans.append(loan)
    
    return loans

def create_clo_structure():
    """Create a sample CLO structure with multiple tranches."""
    
    # Create asset pool
    loans = create_sample_loan_pool()
    pool = finstack.AssetPool.new("CLO_POOL_2025_1", finstack.DealType.CLO, Currency.USD)
    
    # Add loans to pool with industry classifications
    for i, loan in enumerate(loans):
        if "TECH" in loan.id:
            pool.add_loan(loan, "Technology")
        elif "HEALTH" in loan.id:
            pool.add_loan(loan, "Healthcare")
        elif "ENERGY" in loan.id:
            pool.add_loan(loan, "Energy")
    
    # Update pool statistics
    pool.update_stats(Date.from_calendar_date(2025, 1, 15))
    
    # Create CLO with standard tranche structure
    legal_maturity = Date.from_calendar_date(2030, 1, 15)
    
    clo = finstack.Clo.builder("CLO_2025_1") \
        .pool(pool) \
        .add_equity_tranche(0.0, 10.0, Money.new(87_500_000.0, Currency.USD), 0.15) \
        .add_senior_tranche(10.0, 100.0, Money.new(787_500_000.0, Currency.USD), 150.0) \
        .legal_maturity(legal_maturity) \
        .disc_id("USD-OIS") \
        .manager("CLO_MANAGER_LLC") \
        .build()
    
    return clo

def run_coverage_tests(clo):
    """Demonstrate coverage test calculations."""
    
    print("\n=== Coverage Test Analysis ===")
    
    # Calculate current pool statistics
    print(f"Pool Total Balance: ${clo.pool.total_balance().amount():,.0f}")
    print(f"Pool WAC: {clo.pool.weighted_avg_coupon()*100:.2f}%")
    print(f"Pool WAL: {clo.pool.weighted_avg_life(Date.from_calendar_date(2025, 1, 15)):.2f} years")
    print(f"Diversity Score: {clo.pool.diversity_score():.1f}")
    
    # Check concentration limits
    concentration_result = clo.pool.check_concentration_limits()
    if concentration_result.has_violations():
        print(f"\nConcentration Violations: {len(concentration_result.violations)}")
        for violation in concentration_result.violations:
            print(f"  {violation.violation_type}: {violation.current_level:.1f}% vs {violation.limit:.1f}% limit")
    else:
        print("\nAll concentration limits satisfied")
    
    # Calculate coverage ratios for each tranche
    print("\nCoverage Ratios:")
    for tranche in clo.tranches.tranches:
        # Calculate OC ratio (simplified)
        pool_value = clo.pool.performing_balance().amount()
        senior_balance = clo.tranches.senior_balance(tranche.id.as_str()).amount()
        
        if tranche.seniority != finstack.TrancheSeniority.Equity:
            oc_ratio = pool_value / (senior_balance + tranche.current_balance.amount())
            print(f"  {tranche.id.as_str()} OC Ratio: {oc_ratio:.2f}x")
            
            # Check against typical triggers
            if oc_ratio < 1.15:
                print(f"    WARNING: Below typical 115% OC trigger")

def demonstrate_loss_scenarios(clo):
    """Demonstrate how losses flow through the tranche structure."""
    
    print("\n=== Loss Scenario Analysis ===")
    
    pool_balance = clo.pool.total_balance()
    
    # Test different loss scenarios
    loss_scenarios = [0.0, 5.0, 10.0, 15.0, 20.0]  # Loss percentages
    
    print(f"{'Loss %':<8} {'Equity Loss':<15} {'Senior Loss':<15} {'Equity Remaining':<18}")
    print("-" * 65)
    
    for loss_pct in loss_scenarios:
        equity_tranche = next((t for t in clo.tranches.tranches if t.seniority == finstack.TrancheSeniority.Equity), None)
        senior_tranche = next((t for t in clo.tranches.tranches if t.seniority == finstack.TrancheSeniority.Senior), None)
        
        if equity_tranche and senior_tranche:
            equity_loss = equity_tranche.loss_allocation(loss_pct, pool_balance)
            senior_loss = senior_tranche.loss_allocation(loss_pct, pool_balance)
            equity_remaining = equity_tranche.current_balance_after_losses(loss_pct, pool_balance)
            
            print(f"{loss_pct:<8.1f} ${equity_loss.amount():<14,.0f} ${senior_loss.amount():<14,.0f} ${equity_remaining.amount():<17,.0f}")

def main():
    """Main example execution."""
    
    print("CLO/ABS Structured Credit Example")
    print("=" * 50)
    
    try:
        # Create CLO structure
        print("\nCreating CLO structure...")
        clo = create_clo_structure()
        
        print(f"Created CLO: {clo.id.as_str()}")
        print(f"Deal Type: {clo.deal_type}")
        print(f"Number of Pool Assets: {len(clo.pool.assets)}")
        print(f"Number of Tranches: {len(clo.tranches.tranches)}")
        
        # Display tranche information
        print("\nTranche Structure:")
        print(f"{'Tranche':<12} {'Attachment':<12} {'Detachment':<12} {'Balance':<15} {'Coupon':<8}")
        print("-" * 70)
        
        for tranche in clo.tranches.tranches:
            balance_str = f"${tranche.current_balance.amount():,.0f}"
            print(f"{tranche.id.as_str():<12} {tranche.attachment_point:<12.1f} {tranche.detachment_point:<12.1f} {balance_str:<15} {tranche.coupon.current_rate(Date.from_calendar_date(2025, 1, 15))*100:<8.1f}%")
        
        # Run coverage tests
        run_coverage_tests(clo)
        
        # Demonstrate loss scenarios
        demonstrate_loss_scenarios(clo)
        
        # Calculate basic valuation (would need market context in real implementation)
        print(f"\nCLO Legal Maturity: {clo.legal_maturity}")
        print(f"Expected Life: {clo.expected_life(Date.from_calendar_date(2025, 1, 15)):.2f} years")
        
        print("\n✓ CLO/ABS example completed successfully")
        
    except Exception as e:
        print(f"Error: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
