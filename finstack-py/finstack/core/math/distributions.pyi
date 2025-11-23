"""Statistical distribution bindings.

Provides binomial probability calculations, logarithmic helpers, and
Beta sampling utilities for financial modeling.
"""

def binomial_probability(trials: int, successes: int, probability: float) -> float: ...

"""Calculate binomial probability.

Parameters
----------
trials : int
    Number of trials.
successes : int
    Number of successes.
probability : float
    Success probability per trial.

Returns
-------
float
    Binomial probability.

Raises
------
ValueError
    If parameters are invalid.
"""

def log_binomial_coefficient(trials: int, successes: int) -> float: ...

"""Calculate log of binomial coefficient.

Parameters
----------
trials : int
    Number of trials.
successes : int
    Number of successes.

Returns
-------
float
    Log of binomial coefficient.

Raises
------
ValueError
    If parameters are invalid.
"""

def log_factorial(value: int) -> float: ...

"""Calculate log of factorial.

Parameters
----------
value : int
    Value to compute factorial of.

Returns
-------
float
    Log of factorial.

Raises
------
ValueError
    If value is negative.
"""

def sample_beta(alpha: float, beta: float, seed: int | None = ...) -> float: ...

"""Sample from a Beta(alpha, beta) distribution.

Parameters
----------
alpha : float
    First shape parameter (> 0).
beta : float
    Second shape parameter (> 0).
seed : int, optional
    Optional RNG seed for deterministic sampling.

Returns
-------
float
    Sample in [0.0, 1.0].
"""
