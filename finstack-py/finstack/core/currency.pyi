# flake8: noqa: PYI021
def get(code):
    """
    Look up a currency by ISO code.

    Parameters
    ----------
    code : str
        Three-letter ISO currency code (case-insensitive).

    Returns
    -------
    Currency
        Currency instance matching ``code``.
    """
