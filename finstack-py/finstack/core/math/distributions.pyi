"""Statistical distribution bindings.

Provides binomial probability calculations and related
logarithmic functions for financial modeling.
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
