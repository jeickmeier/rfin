# flake8: noqa: PYI021
def binomial_probability(trials, successes, probability):
    """
    Compute the probability mass at a target count for a Binomial distribution.

    Computes the probability ``P(X = successes)`` where ``X ~ Binomial(trials, probability)``.

    Args:
        trials (int): Total number of Bernoulli trials (``n``).
        successes (int): Target number of successes (``k``).
        probability (float): Probability of success per trial (``p``), in the range [0, 1].

    Returns:
        float: Probability mass at ``successes``.

    Examples:
        >>> from finstack.core.math.distributions import binomial_probability
        >>> binomial_probability(10, 3, 0.5)
        0.1171875
    """

def log_binomial_coefficient(trials, successes):
    """
    Natural logarithm of the binomial coefficient.

    Computes ``ln(C(trials, successes)) = ln(n! / (k!(n-k)!))``.

    Args:
        trials (int): Total number of items (``n``).
        successes (int): Number of items chosen (``k``).

    Returns:
        float: Natural logarithm of the binomial coefficient.

    Examples:
        >>> from finstack.core.math.distributions import log_binomial_coefficient
        >>> round(log_binomial_coefficient(5, 2), 6)
        2.397895
    """

def log_factorial(value):
    """
    Natural logarithm of a factorial.

    Computes ``ln(value!)`` using exact arithmetic for small values and a
    stable approximation (e.g., Stirling-like) when needed.

    Args:
        value (int): Non-negative integer ``n`` whose factorial is evaluated.

    Returns:
        float: ``ln(n!)``.

    Raises:
        ValueError: If ``value`` is negative.

    Examples:
        >>> from finstack.core.math.distributions import log_factorial
        >>> round(log_factorial(5), 6)
        4.787492
    """
