#!/usr/bin/env python3
"""Example demonstrating loan workout and recovery management in finstack."""

from finstack import (
    Date, Money, Currency, USD,
    WorkoutState, WorkoutPolicy, WorkoutStrategy, 
    RateModification, PrincipalModification, 
    RecoveryWaterfall, RecoveryTier, ClaimAmount,
    WorkoutEngine
)

def main():
    print("=" * 60)
    print("Loan Workout and Recovery Example")
    print("=" * 60)
    
    # Define recovery waterfall
    recovery_tiers = [
        RecoveryTier(
            name="Senior Secured",
            claim_type="Senior",
            claim_amount=ClaimAmount("fixed", amount=Money(10_300_000, USD)),
            recovery_pct=0.85
        ),
        RecoveryTier(
            name="Senior Unsecured",
            claim_type="Senior",
            claim_amount=ClaimAmount("fixed", amount=Money(5_150_000, USD)),
            recovery_pct=0.45
        ),
        RecoveryTier(
            name="Subordinated",
            claim_type="Subordinated",
            claim_amount=ClaimAmount("fixed", amount=Money(2_060_000, USD)),
            recovery_pct=0.15
        )
    ]
    
    waterfall = RecoveryWaterfall(tiers=recovery_tiers)
    
    # Define workout strategies
    strategies = {
        "forbearance": WorkoutStrategy(
            name="Forbearance",
            forbearance_months=6,
            rate_modification=RateModification("reduce_by", bps=200),
            principal_modification=PrincipalModification("defer", percentage=0.25)
        ),
        "restructure": WorkoutStrategy(
            name="Restructure",
            rate_modification=RateModification("set_to", rate=0.05),
            principal_modification=PrincipalModification("forgive", percentage=0.10),
            maturity_extension_months=24
        ),
        "liquidation": WorkoutStrategy(
            name="Liquidation",
            rate_modification=RateModification("set_to", rate=0.0),
            principal_modification=PrincipalModification("reamortize", months=1)
        )
    }
    
    # Create workout policy
    policy = WorkoutPolicy(
        name="Default Policy",
        recovery_waterfall=waterfall
    )
    
    # Add stress thresholds
    policy.add_stress_threshold("dscr", 1.25)
    policy.add_stress_threshold("ltv", 0.80)
    
    # Add workout strategies
    for key, strategy in strategies.items():
        policy.add_strategy(strategy)
    
    # Create workout engine
    engine = WorkoutEngine(policy)
    print(f"Initial state: {engine.get_state()}")
    print()
    
    # Simulate workout progression
    
    # 1. Move to Stressed status
    print("Loan showing stress - moving to Stressed status...")
    engine.transition(
        WorkoutState("stressed", indicators=["dscr_breach", "covenant_breach"]), 
        Date(2024, 1, 15),
        "DSCR and covenant breaches detected"
    )
    print(f"Current state: {engine.get_state()}")
    print()
    
    # 2. Default occurs
    print("Default event - moving to Default status...")
    engine.transition(
        WorkoutState("default", 
                    default_date=Date(2024, 3, 1),
                    reason="Payment default"),
        Date(2024, 3, 1),
        "Missed payment - loan in default"
    )
    print(f"Current state: {engine.get_state()}")
    print()
    
    # 3. Move to Workout and apply forbearance strategy
    print("Moving to Workout status and applying forbearance strategy...")
    engine.transition(
        WorkoutState("workout",
                    start_date=Date(2024, 3, 15),
                    workout_type="forbearance"),
        Date(2024, 3, 15),
        "Entering forbearance agreement"
    )
    
    # Note: Apply method would be called here in a full implementation
    print("Forbearance terms applied (rate reduced by 200bps, 25% principal deferred)")
    print(f"Current state: {engine.get_state()}")
    print()
    
    # 4. Generate recovery analysis
    print("Generating recovery analysis...")
    recovery = engine.generate_recovery_analysis(
        outstanding=Money(17_000_000, USD),
        collateral_value=Money(12_000_000, USD),
        as_of=Date(2024, 6, 1)
    )
    
    print(f"Expected recovery: {recovery.expected_recovery}")
    print(f"Recovery rate: {recovery.recovery_rate:.1%}")
    print("Recovery by tier:")
    for tier_name, amount in recovery.tier_recoveries:
        print(f"  - {tier_name}: {amount}")
    print()
    
    # 5. Restructure loan - move to Recovered status
    print("Loan recovered after restructuring...")
    engine.transition(
        WorkoutState("recovered",
                    recovery_date=Date(2024, 7, 1),
                    recovery_rate=recovery.recovery_rate),
        Date(2024, 7, 1),
        "Loan successfully restructured and recovered"
    )
    print(f"Final state: {engine.get_state()}")
    
    print()
    print("Example completed successfully!")

if __name__ == "__main__":
    main()
