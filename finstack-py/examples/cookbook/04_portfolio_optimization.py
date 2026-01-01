"""
Title: Constrained Portfolio Optimization
Persona: Portfolio Manager
Complexity: Intermediate
Runtime: ~2 seconds

Description:
Demonstrates portfolio optimization with rating constraints.

Key Concepts:
- Optimization problem setup
- Objective functions (maximize yield)
- Constraints (rating limits, concentration)
- Trade universe definition

Prerequisites:
- Portfolio construction basics
- Understanding of optimization
"""

from finstack import (
    PortfolioOptimizationProblem,
    Objective,
    Constraint,
    TradeUniverse,
    CandidatePosition,
)


def main():
    print("COOKBOOK EXAMPLE 04: Portfolio Optimization")
    print("="*60)
    
    # Define candidate positions
    candidates = [
        CandidatePosition(
            id="BOND.AAA.5Y",
            instrument_type="bond",
            tags={"rating": "AAA", "sector": "Financial"},
            expected_yield=0.040
        ),
        CandidatePosition(
            id="BOND.AA.5Y",
            instrument_type="bond",
            tags={"rating": "AA", "sector": "Industrial"},
            expected_yield=0.045
        ),
        CandidatePosition(
            id="BOND.BBB.5Y",
            instrument_type="bond",
            tags={"rating": "BBB", "sector": "Technology"},
            expected_yield=0.050
        ),
        CandidatePosition(
            id="BOND.BB.5Y",
            instrument_type="bond",
            tags={"rating": "BB", "sector": "Energy"},
            expected_yield=0.070
        ),
        CandidatePosition(
            id="BOND.CCC.3Y",
            instrument_type="bond",
            tags={"rating": "CCC", "sector": "Energy"},
            expected_yield=0.100
        ),
    ]
    
    universe = TradeUniverse(candidates)
    
    # Create optimization problem
    problem = PortfolioOptimizationProblem(universe)
    
    # Objective: maximize yield
    problem.add_objective(Objective.maximize_metric("expected_yield"))
    
    # Constraints
    problem.add_constraint(Constraint.budget(100_000_000))  # $100M total
    problem.add_constraint(Constraint.weight_bounds(0.0, 0.25))  # Max 25% per position
    problem.add_constraint(Constraint.tag_exposure_limit("rating", "CCC", 0.10))  # Max 10% CCC
    problem.add_constraint(Constraint.tag_exposure_limit("rating", "BB", 0.20))  # Max 20% BB
    problem.add_constraint(Constraint.tag_exposure_minimum("rating", "AAA", 0.20))  # Min 20% AAA
    problem.add_constraint(Constraint.tag_exposure_limit("sector", "Energy", 0.30))  # Max 30% Energy
    
    # Solve
    result = problem.solve()
    
    print(f"\nOptimization Status: {result.status}")
    print(f"Objective Value (Yield): {result.objective_value:.4f}\n")
    
    print("Optimal Allocation:")
    print("-"*70)
    print(f"{'Position':<18} {'Rating':<8} {'Sector':<12} {'Weight':<12} {'Amount':<18}")
    print("-"*70)
    
    for trade in result.trades:
        cand = next(c for c in candidates if c.id == trade.position_id)
        rating = cand.tags.get("rating", "N/A")
        sector = cand.tags.get("sector", "N/A")
        weight_pct = trade.target_weight * 100
        amount = trade.notional
        
        print(f"{trade.position_id:<18} {rating:<8} {sector:<12} "
              f"{weight_pct:>10.2f}% ${amount:>16,.2f}")
    
    print("-"*70)
    print(f"{'TOTAL':<18} {'':<8} {'':<12} {100.0:>10.2f}% ${100_000_000:>16,.2f}")
    print()
    
    print("="*60)
    print("Key Takeaway: Optimization balances yield vs credit quality")


if __name__ == "__main__":
    main()
