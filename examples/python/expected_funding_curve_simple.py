#!/usr/bin/env python3
"""
Simple example demonstrating expected funding curves concept.

This shows the structure and API for adding expected future draws
to DDTL and RCF instruments for pricing.
"""


def demo_expected_funding_concept():
    """Demonstrate the concept of expected funding curves."""
    print("=" * 60)
    print("EXPECTED FUNDING CURVES - CONCEPT DEMONSTRATION")
    print("=" * 60)

    print(
        """
Expected funding curves enable more accurate pricing by incorporating:

1. DELAYED-DRAW TERM LOAN (DDTL) Example:
   - $50M commitment, expires 2026-12-31
   - Already drawn: $5M
   - Expected future draws:
     * 2025-06-01: $15M (expansion project) @ 100% probability  
     * 2025-12-01: $10M (working capital) @ 90% probability
     * 2026-06-01: $8M (equipment) @ 60% probability
   
   Expected value impact:
   - Draw cashflows: -$15M - ($10M × 0.9) - ($8M × 0.6) = -$29.8M
   - Interest income: +$X (based on rates and remaining terms)
   - Commitment fees: reduced as draws occur

2. REVOLVING CREDIT FACILITY (RCF) Example:
   - $100M commitment, revolving facility
   - Current drawn: $20M
   - Expected activity:
     * 2025-03-01: Draw $25M (seasonal needs)
     * 2025-06-01: Repay $15M (cash flow improvement)  
     * 2025-09-01: Draw $30M (inventory buildup)
     * 2025-12-01: Repay $20M (year-end deleveraging)
   
   Expected utilization over time:
   - Mar: ($20M + $25M) / $100M = 45%
   - Jun: ($45M - $15M) / $100M = 30%  
   - Sep: ($30M + $30M) / $100M = 60%
   - Dec: ($60M - $20M) / $100M = 40%

3. Pricing Benefits:
   - More accurate NPV calculations
   - Better commitment fee projections
   - Risk-adjusted valuations using probabilities
   - Scenario analysis capabilities
   
4. API Structure (pseudo-code):
   ```python
   # Basic DDTL with expected draws
   ddtl = DelayedDrawTermLoan("DDTL-001", commitment, expiry, maturity)
   ddtl.with_expected_draws([
       DrawEvent(date="2025-06-01", amount=15_000_000),
       DrawEvent(date="2025-12-01", amount=10_000_000)
   ])
   
   # With probabilities
   curve = ExpectedFundingCurve(
       expected_draws=draws,
       draw_probabilities=[1.0, 0.9, 0.6]
   )
   ddtl.with_expected_funding_curve(curve)
   
   # RCF with expected activity
   rcf = RevolvingCreditFacility("RCF-001", commitment, start, maturity)
   rcf.with_expected_events([
       DrawEvent(date="2025-03-01", amount=25_000_000),   # draw
       DrawEvent(date="2025-06-01", amount=-15_000_000),  # repay
   ])
   ```

This functionality is now implemented in both Rust and Python bindings!
    """
    )


if __name__ == "__main__":
    demo_expected_funding_concept()
