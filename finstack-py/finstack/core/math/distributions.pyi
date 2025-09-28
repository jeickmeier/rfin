# flake8: noqa: PYI021
def binomial_probability(trials: int, successes: int, probability: float) -> float:
    """Compute the probability mass at ``successes`` for a Binomial(trials, probability) distribution."""

def log_binomial_coefficient(trials: int, successes: int) -> float:
    """Natural logarithm of the binomial coefficient C(trials, successes)."""

def log_factorial(value: int) -> float:
    """Natural logarithm of ``value!`` calculated with a stable approximation."""

__all__ = [
    "binomial_probability",
    "log_binomial_coefficient",
    "log_factorial",
]
