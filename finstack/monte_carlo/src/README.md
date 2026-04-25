# Monte Carlo Pricing Framework

`finstack-monte-carlo` provides reusable Monte Carlo building blocks for
quantitative pricing and risk work: random streams, stochastic processes,
discretization schemes, payoffs, pricing engines, path capture, and result
types with explicit currencies.

This README is intentionally short and crate-specific. It describes the public
surface that exists today rather than a wishlist of future products.

## Start Here

- Use `finstack_monte_carlo::prelude::*` for the most common engine, process,
  payoff, and pricer types.
- Use `engine::McEngine` when you want a generic simulation loop.
- Use `pricer::european::EuropeanPricer` for a smaller GBM-only entry point.
- Read `traits.rs` first if you are implementing a new process, discretization,
  or payoff.

## Feature Flags

This crate currently declares no optional Cargo features. Rayon-backed parallel
simulation and the full model/pricer/payoff surface are compiled by default.

## Public Surface

### Public modules

- `engine`: `McEngine`, `McEngineConfig`, path-capture configuration
- `traits`: `RandomStream`, `StochasticProcess`, `Discretization`, `Payoff`, `PathState`
- `rng::philox`: deterministic pseudo-random generator with splittable streams
- `rng::sobol`, `rng::poisson`, and Brownian-bridge utilities
- `process`: GBM, Brownian, multi-GBM, multi-OU, Heston, CIR,
  Hull-White / Vasicek, Bates, Merton jump diffusion, Schwartz-Smith, LMM,
  rough Bergomi, rough Heston, Cheyette rough-rate, and process metadata helpers
- `discretization`: exact GBM and related exact schemes
- `payoff`: European, Asian, barrier, basket, lookback, digital, and forward payoffs
- `pricer`: European, path-dependent, Longstaff-Schwartz LSMC, and regression basis helpers
- `greeks`: pathwise, likelihood-ratio, and finite-difference estimators
- `variance_reduction`: antithetic, control variate, moment matching, and importance sampling
- `results`, `estimate`, `paths`, `time_grid`, `online_stats`

## Conventions

- Rates, dividend yields, and volatilities are quoted in decimals, not basis points.
- Times are year fractions.
- `McEngine::price` and `price_with_capture` expect a caller-supplied discount
  factor for the payoff horizon. Under a flat continuously compounded rate this
  is typically `exp(-rT)`.
- `Payoff::value` returns an undiscounted `Money` amount in the requested
  currency; the engine applies the discount factor outside the payoff.
- Path-capture summaries such as percentiles, minima, and maxima are derived
  from the captured paths, which may be a sample of the full simulation.

## Minimal Example

```rust,no_run
use finstack_core::currency::Currency;
use finstack_monte_carlo::prelude::*;

let engine = McEngine::builder()
    .num_paths(50_000)
    .seed(7)
    .uniform_grid(1.0, 252)
    .build()
    .expect("valid Monte Carlo configuration");

let rng = PhiloxRng::new(7);
let process = GbmProcess::with_params(0.03, 0.01, 0.20)
    .expect("valid GBM parameters");
let disc = ExactGbm::new();
let payoff = EuropeanCall::new(100.0, 1.0, 252);
let discount_factor = (-0.03_f64).exp();

let result = engine
    .price(
        &rng,
        &process,
        &disc,
        &[100.0],
        &payoff,
        Currency::USD,
        discount_factor,
    )
    .expect("pricing should succeed");

println!("price={} stderr={}", result.mean, result.stderr);
```

## Module Map

- `barriers/`: Brownian-bridge hit checks and continuity corrections
- `discretization/`: time-stepping schemes and exact transitions
- `greeks/`: pathwise, likelihood-ratio, and finite-difference estimators
- `paths.rs`: captured-path types, cashflow metadata, and process metadata
- `payoff/`: payoff definitions
- `pricer/`: higher-level pricing orchestrators
- `process/`: stochastic process definitions and correlation helpers
- `rng/`: pseudo-random and quasi-random generators
- `variance_reduction/`: antithetic, control variate, moment matching, importance sampling

## References

- Heston-model docs should link to
  [`docs/REFERENCES.md#heston-1993`](../../docs/REFERENCES.md#heston-1993).
- Discounting and basic option-pricing conventions should link to
  [`docs/REFERENCES.md#hull-options-futures`](../../docs/REFERENCES.md#hull-options-futures).
- Monte Carlo and numerical-scheme modules may also cite Glasserman (2003),
  Andersen (2008), Kloeden-Platen (1992), and related canonical texts in their
  own `# References` sections.
