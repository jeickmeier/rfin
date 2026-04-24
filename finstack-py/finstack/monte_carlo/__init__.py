"""Monte Carlo GBM convenience bindings: engine, pricers, analytical helpers.

Bindings for the GBM-oriented convenience subset of the
``finstack-monte-carlo`` Rust crate. Advanced Rust process, discretization,
RNG, payoff, and Greeks types are not surfaced as standalone Python types yet;
their parameters are passed directly as numeric arguments to the exposed
pricer constructors and methods.
"""

from __future__ import annotations

import sys

from finstack.finstack import monte_carlo as _mc

MonteCarloResult = _mc.MonteCarloResult
Estimate = _mc.Estimate

TimeGrid = _mc.TimeGrid

McEngine = _mc.McEngine

EuropeanPricer = _mc.EuropeanPricer
PathDependentPricer = _mc.PathDependentPricer
LsmcPricer = _mc.LsmcPricer

black_scholes_call = _mc.black_scholes_call
black_scholes_put = _mc.black_scholes_put

price_european_call = _mc.price_european_call
price_european_put = _mc.price_european_put

_key = "finstack.monte_carlo"
if _key not in sys.modules:
    sys.modules[_key] = sys.modules[__name__]

__all__: list[str] = [
    "Estimate",
    "EuropeanPricer",
    "LsmcPricer",
    "McEngine",
    "MonteCarloResult",
    "PathDependentPricer",
    "TimeGrid",
    "black_scholes_call",
    "black_scholes_put",
    "price_european_call",
    "price_european_put",
]
