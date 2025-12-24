# Valuations Implementation TODO

This TODO list prioritizes unimplemented features and explicit limitations in
`finstack/valuations`, plus the expansion items in `EXPANSION_PLAN.md`.

## Prioritization Criteria
1. Hard errors or missing functionality in common pricing paths.
2. Risk/accuracy gaps that materially change reported results.
3. Coverage gaps blocking planned products or analytics.

## P0 - Blockers (pricing returns error or is unusable)
- [ ] MarketContext instrument registry + BondFuture pricing
  - [ ] Add instrument registry to `finstack/core/src/market_data/context.rs`
  - [ ] Define insert/get APIs for `Box<dyn Instrument>` (or `Arc`)
  - [ ] Add serialization strategy or document non-serializable behavior
  - [ ] Add unit tests for registry lifecycle + thread safety
  - [ ] Update `BondFuturePricer` to resolve CTD bond and compute NPV
  - [ ] Add end-to-end tests for bond future pricing (CTD selection, NPV)
- [ ] Seasoned compounded-in-arrears IRS valuation
  - [ ] Support valuation dates inside an accrual period
  - [ ] Use historical fixings up to `as_of`, project remainder off forward
  - [ ] Add tests for OIS swaps with partial accrual
- [ ] CDS tranche schedule in strict market mode
  - [ ] Support non-IMM schedules (or soften strict mode with a fallback path)
  - [ ] Add schedule generation tests for IMM vs non-IMM inputs

## P1 - Accuracy/Risk Gaps (results materially off)
- [ ] SIMM correlation matrix and aggregation
  - [ ] Implement risk-class correlations per ISDA SIMM
  - [ ] Add tests for aggregation vs reference examples
  - [ ] Document remaining gaps (vega/curvature treatment if not covered)
- [ ] Clearing house and internal model IM
  - [ ] Replace schedule fallback with CCP methodology stubs (LCH/CME/ICE)
  - [ ] Define interfaces for plugging in CCP VaR/SPAN inputs
  - [ ] Add unit tests to validate method selection and outputs
- [ ] VaR Taylor approximation
  - [ ] Implement delta/gamma-based approximation
  - [ ] Wire in required sensitivities via metrics registry
  - [ ] Add tests that compare full reval vs Taylor on simple instruments
- [ ] Basket constituent delta for instrument references
  - [ ] Add a bump/override mechanism for instrument-based constituents
  - [ ] Or expose per-constituent delta through instrument interface
  - [ ] Add tests for mixed MarketData + Instrument references

## P2 - Coverage Improvements (feature completeness)
- [ ] TermLoanSpec -> TermLoan conversion
  - [ ] Implement builder path from spec to runtime instrument
  - [ ] Add validation + tests for key spec combinations
- [ ] Term loan DDTL commitment fee windowing
  - [ ] Implement time-varying facility limits for commitment fee base
  - [ ] Add tests for step-downs and draw windows
- [ ] OID effective interest rate (EIR) amortization
  - [ ] Define `OidEirSpec` and schedule integration
  - [ ] Add accounting/reporting outputs for EIR amortization
- [ ] Barrier rebates in closed-form pricing
  - [ ] Implement rebate terms for knock-in/out formulas
  - [ ] Add unit tests vs known references
- [ ] American knock-in barrier tree support
  - [ ] Add path-dependent state tracking to tree nodes
  - [ ] Validate in/out parity vs European for consistency
- [ ] Sobol path capture in MC pricer
  - [ ] Support capture with Sobol RNG or enforce deterministic mapping
  - [ ] Add tests for path capture determinism
- [ ] Periodic clawback in PE fund waterfall
  - [ ] Implement periodic settlement logic
  - [ ] Add tests for periodic vs terminal clawback

## P3 - Expansion Plan (new instruments/metrics)
- [ ] Inflation cap/floor instrument + Black/Bachelier pricer
  - [ ] Add `InflationCapFloor` types and payoff logic
  - [ ] Add inflation vol surface support + calibration hooks
  - [ ] Add pricing + regression tests
- [ ] FX variance swap instrument + replication pricer
  - [ ] Implement variance swap payoff + realized variance
  - [ ] Add replication integration with FX vol surface
  - [ ] Add tests vs analytic/benchmark cases
- [ ] Commodity option instrument + European/American pricers
  - [ ] Implement Black-76 for European
  - [ ] Implement binomial or BAW for American
  - [ ] Add tests for futures- vs spot-based setups
- [ ] Real estate asset valuation
  - [ ] Implement DCF and direct capitalization methods
  - [ ] Add inputs for NOI schedules and appraisal overrides
  - [ ] Add tests for deterministic NOI cases
- [ ] Bond convexity metric
  - [ ] Implement closed-form convexity using existing cashflow engine
  - [ ] Add tests vs numerical second derivative
- [ ] G-spread metric
  - [ ] Add benchmark curve inputs to MetricContext
  - [ ] Implement maturity interpolation + spread calculation
  - [ ] Add tests for interpolated curve points
- [ ] Register core option greeks (gamma, vanna, volga)
  - [ ] Map FD calculators into metrics registry
  - [ ] Add tests for registration + output sanity
- [ ] Expected exposure (EE/PFE)
  - [ ] Define exposure time grid + MC path integration
  - [ ] Implement EE(t) aggregation + PFE percentiles
  - [ ] Add tests with deterministic scenarios

## Notes
- See `finstack/valuations/EXPANSION_PLAN.md` for detailed product specs.
- Items in P0/P1 should be prioritized before new instruments.
