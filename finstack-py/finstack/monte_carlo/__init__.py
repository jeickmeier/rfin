"""Monte Carlo simulation: engines, processes, payoffs, variance reduction.

Bindings for the ``finstack-monte-carlo`` Rust crate.
"""

from __future__ import annotations

import sys

from finstack.finstack import monte_carlo as _mc

# Results
MonteCarloResult = _mc.MonteCarloResult
Estimate = _mc.Estimate

# Time grid
TimeGrid = _mc.TimeGrid

# Engine
McEngine = _mc.McEngine

# Processes
GbmProcess = _mc.GbmProcess
MultiGbmProcess = _mc.MultiGbmProcess
BrownianProcess = _mc.BrownianProcess
HestonProcess = _mc.HestonProcess
CirProcess = _mc.CirProcess
MertonJumpProcess = _mc.MertonJumpProcess
BatesProcess = _mc.BatesProcess
SchwartzSmithProcess = _mc.SchwartzSmithProcess

# Payoffs
EuropeanCall = _mc.EuropeanCall
EuropeanPut = _mc.EuropeanPut
DigitalCall = _mc.DigitalCall
DigitalPut = _mc.DigitalPut
ForwardLong = _mc.ForwardLong
ForwardShort = _mc.ForwardShort
AsianCall = _mc.AsianCall
AsianPut = _mc.AsianPut
BarrierOption = _mc.BarrierOption
BasketCall = _mc.BasketCall
BasketPut = _mc.BasketPut
AmericanPut = _mc.AmericanPut
AmericanCall = _mc.AmericanCall

# Pricers
EuropeanPricer = _mc.EuropeanPricer
PathDependentPricer = _mc.PathDependentPricer
LsmcPricer = _mc.LsmcPricer

# Analytical
black_scholes_call = _mc.black_scholes_call
black_scholes_put = _mc.black_scholes_put

# Convenience
price_european_call = _mc.price_european_call
price_european_put = _mc.price_european_put

_key = "finstack.monte_carlo"
if _key not in sys.modules:
    sys.modules[_key] = sys.modules[__name__]

__all__: list[str] = [
    "AmericanCall",
    "AmericanPut",
    "AsianCall",
    "AsianPut",
    "BarrierOption",
    "BasketCall",
    "BasketPut",
    "BatesProcess",
    "BrownianProcess",
    "CirProcess",
    "DigitalCall",
    "DigitalPut",
    "Estimate",
    "EuropeanCall",
    "EuropeanPricer",
    "EuropeanPut",
    "ForwardLong",
    "ForwardShort",
    "GbmProcess",
    "HestonProcess",
    "LsmcPricer",
    "McEngine",
    "MertonJumpProcess",
    "MonteCarloResult",
    "MultiGbmProcess",
    "PathDependentPricer",
    "SchwartzSmithProcess",
    "TimeGrid",
    "black_scholes_call",
    "black_scholes_put",
    "price_european_call",
    "price_european_put",
]
