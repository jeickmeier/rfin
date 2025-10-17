# flake8: noqa: PYI021
def create_standard_registry():
    """
    Create a registry populated with all standard finstack pricers.

    Returns:
        PricerRegistry: Registry with all built-in pricers loaded.

    Examples:
        >>> registry = create_standard_registry()
        >>> registry.price(bond, "discounting", market)
        <ValuationResult ...>
    """

def validate_discount_curve(curve):
    ...

def validate_forward_curve(curve):
    ...

def validate_hazard_curve(curve):
    ...

def validate_inflation_curve(curve):
    ...

def validate_vol_surface(surface):
    ...
