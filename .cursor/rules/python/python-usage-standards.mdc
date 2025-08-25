---
globs: *.py
alwaysApply: false
---
# Python Usage Code Standards for rfin

## Overview

This document covers standards for Python code that uses the rfin library, including:
- Example scripts
- Test files
- Documentation examples
- User-facing Python code

## Import Conventions

### Standard Import Order
```python
# Standard library imports
import datetime
from typing import List, Optional, Tuple

# Third-party imports
import numpy as np
import pandas as pd
import pytest

# rfin imports - top-level classes
from rfin import Currency, Money, Date

# rfin imports - submodules
from rfin.dates import Calendar, DayCount, Frequency, generate_schedule
from rfin.cashflow import FixedRateLeg, CashFlow
```

### Import Aliases
```python
# Avoid aliases for rfin types to maintain clarity
# Good
from rfin import Currency, Money

# Avoid
from rfin import Currency as Ccy  # Don't do this
```

## Type Hints

### Always Use Type Hints
```python
from typing import List, Optional, Tuple
from rfin import Currency, Money, Date
from rfin.dates import DayCount, Calendar

def calculate_payment(
    principal: float,
    rate: float,
    currency: Currency,
    day_count: DayCount
) -> Money:
    """Calculate a payment amount."""
    return Money(principal * rate, currency)

def generate_payment_dates(
    start: Date,
    end: Date,
    frequency: Frequency,
    calendar: Optional[Calendar] = None
) -> List[Date]:
    """Generate payment dates with optional calendar adjustment."""
    dates = generate_schedule(start, end, frequency)
    if calendar:
        return [calendar.adjust(d, BusDayConvention.Following) for d in dates]
    return dates
```

## Error Handling

### Explicit Error Handling
```python
from rfin import Currency, Money

def safe_money_operation(amount1: Money, amount2: Money) -> Optional[Money]:
    """Safely add two money amounts, returning None on currency mismatch."""
    try:
        return amount1 + amount2
    except ValueError as e:
        if "Currency mismatch" in str(e):
            print(f"Warning: Cannot add {amount1.currency} and {amount2.currency}")
            return None
        raise  # Re-raise unexpected errors

# Or use explicit checking
def explicit_currency_check(amount1: Money, amount2: Money) -> Money:
    """Add two amounts with explicit currency validation."""
    if amount1.currency != amount2.currency:
        raise ValueError(
            f"Cannot add {amount1.currency.code} and {amount2.currency.code}"
        )
    return amount1 + amount2
```

## Testing Standards

### Test Structure
```python
import pytest
from datetime import date
from rfin import Currency, Money, Date
from rfin.dates import DayCount, Calendar, Frequency

class TestMoneyOperations:
    """Test suite for Money operations."""
    
    @pytest.fixture
    def usd(self) -> Currency:
        """USD currency fixture."""
        return Currency.usd()
    
    @pytest.fixture
    def eur(self) -> Currency:
        """EUR currency fixture."""
        return Currency.eur()
    
    def test_money_creation(self, usd: Currency):
        """Test creating Money instances."""
        money = Money(100.0, usd)
        assert money.amount == 100.0
        assert money.currency == usd
    
    def test_money_addition_same_currency(self, usd: Currency):
        """Test adding money with same currency."""
        money1 = Money(100.0, usd)
        money2 = Money(50.0, usd)
        result = money1 + money2
        assert result.amount == 150.0
        assert result.currency == usd
    
    def test_money_addition_different_currency_raises(
        self, 
        usd: Currency, 
        eur: Currency
    ):
        """Test that adding different currencies raises ValueError."""
        money1 = Money(100.0, usd)
        money2 = Money(50.0, eur)
        
        with pytest.raises(ValueError, match="Currency mismatch"):
            _ = money1 + money2
    
    @pytest.mark.parametrize("amount,expected_str", [
        (100.0, "100.00 USD"),
        (99.99, "99.99 USD"),
        (1000.50, "1000.50 USD"),
    ])
    def test_money_string_representation(
        self, 
        usd: Currency,
        amount: float,
        expected_str: str
    ):
        """Test string representation of Money."""
        money = Money(amount, usd)
        # Note: Actual format may differ, adjust as needed
        assert f"{money.amount:.2f} {money.currency}" == expected_str
```

### Test Naming Conventions
```python
# Test method names should be descriptive
def test_fixed_rate_leg_generates_correct_number_of_flows():
    """Test that a fixed rate leg generates the expected number of cash flows."""
    pass

def test_calendar_adjustment_following_moves_weekend_to_monday():
    """Test that Following convention moves Saturday/Sunday to Monday."""
    pass

# Use descriptive names for test data
def test_day_count_calculations():
    """Test day count convention calculations."""
    # Good: Descriptive variable names
    start_date = Date(2023, 1, 1)
    end_date = Date(2023, 7, 1)
    expected_act360_year_fraction = 0.5027777777777778
    
    # Calculate and assert
    dc = DayCount.act360()
    actual = dc.year_fraction(start_date, end_date)
    assert abs(actual - expected_act360_year_fraction) < 1e-10
```

## Documentation Examples

### Docstring Examples
```python
def create_swap_leg(
    notional: float,
    currency: Currency,
    fixed_rate: float,
    start_date: Date,
    maturity: Date,
    payment_frequency: Frequency = Frequency.SemiAnnual,
    day_count: DayCount = DayCount.thirty360()
) -> FixedRateLeg:
    """
    Create a fixed-rate swap leg.
    
    Args:
        notional: The notional principal amount
        currency: The currency of the swap leg
        fixed_rate: The fixed interest rate (e.g., 0.05 for 5%)
        start_date: The start date of the swap
        maturity: The maturity date of the swap
        payment_frequency: Payment frequency (default: semi-annual)
        day_count: Day count convention (default: 30/360)
    
    Returns:
        A FixedRateLeg object representing the swap leg
    
    Examples:
        >>> # Create a 5-year USD swap leg paying 3% semi-annually
        >>> leg = create_swap_leg(
        ...     notional=10_000_000,
        ...     currency=Currency.usd(),
        ...     fixed_rate=0.03,
        ...     start_date=Date(2023, 1, 1),
        ...     maturity=Date(2028, 1, 1),
        ...     payment_frequency=Frequency.SemiAnnual,
        ...     day_count=DayCount.thirty360()
        ... )
        >>> leg.num_flows
        10  # 5 years * 2 payments per year
        
        >>> # Create a quarterly EUR swap leg
        >>> eur_leg = create_swap_leg(
        ...     notional=5_000_000,
        ...     currency=Currency.eur(),
        ...     fixed_rate=0.025,
        ...     start_date=Date(2023, 1, 1),
        ...     maturity=Date(2025, 1, 1),
        ...     payment_frequency=Frequency.Quarterly
        ... )
        >>> eur_leg.num_flows
        8  # 2 years * 4 payments per year
    """
    return FixedRateLeg(
        notional_amount=notional,
        currency=currency,
        rate=fixed_rate,
        start_date=start_date,
        end_date=maturity,
        frequency=payment_frequency,
        day_count=day_count
    )
```

## Example Script Structure

### Complete Example Script
```python
#!/usr/bin/env python3
"""
Example: Creating and analyzing a fixed-rate bond.

This example demonstrates how to:
1. Create a fixed-rate bond cash flow schedule
2. Calculate accrued interest
3. Compute present value
4. Handle different day count conventions
"""

from typing import List
import pandas as pd

from rfin import Currency, Money, Date
from rfin.dates import (
    Calendar, 
    DayCount, 
    Frequency, 
    BusDayConvention,
    generate_schedule
)
from rfin.cashflow import FixedRateLeg


def create_bond_cashflows(
    face_value: float,
    coupon_rate: float,
    issue_date: Date,
    maturity_date: Date,
    currency: Currency = Currency.usd(),
    frequency: Frequency = Frequency.SemiAnnual,
    day_count: DayCount = DayCount.thirty360(),
    calendar: Calendar = Calendar.from_id("usny")
) -> FixedRateLeg:
    """Create cash flows for a fixed-rate bond."""
    return FixedRateLeg(
        notional_amount=face_value,
        currency=currency,
        rate=coupon_rate,
        start_date=issue_date,
        end_date=maturity_date,
        frequency=frequency,
        day_count=day_count
    )


def analyze_bond(leg: FixedRateLeg, valuation_date: Date) -> pd.DataFrame:
    """Analyze bond cash flows and return summary DataFrame."""
    flows = leg.flows()
    
    data = []
    for i, flow in enumerate(flows, 1):
        data.append({
            "Payment #": i,
            "Date": flow.date,
            "Amount": flow.amount,
            "Currency": flow.currency.code,
            "Type": flow.kind
        })
    
    df = pd.DataFrame(data)
    
    # Add summary statistics
    print(f"\nBond Analysis as of {valuation_date}")
    print(f"Total cash flows: {len(flows)}")
    print(f"Accrued interest: {leg.accrued(valuation_date)}")
    print(f"Present value: {leg.npv()}")
    
    return df


def main():
    """Main example execution."""
    # Create a 5-year corporate bond
    bond = create_bond_cashflows(
        face_value=1_000_000,      # $1M face value
        coupon_rate=0.045,         # 4.5% coupon
        issue_date=Date(2023, 1, 15),
        maturity_date=Date(2028, 1, 15),
        currency=Currency.usd(),
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360()
    )
    
    # Analyze the bond
    valuation_date = Date(2023, 3, 1)
    df = analyze_bond(bond, valuation_date)
    
    # Display results
    print("\nCash Flow Schedule:")
    print(df.to_string(index=False))
    
    # Compare day count conventions
    print("\nDay Count Convention Comparison:")
    conventions = [
        ("30/360", DayCount.thirty360()),
        ("ACT/360", DayCount.act360()),
        ("ACT/365F", DayCount.act365f()),
        ("ACT/ACT", DayCount.actact())
    ]
    
    start = Date(2023, 1, 1)
    end = Date(2023, 7, 1)
    
    for name, dc in conventions:
        yf = dc.year_fraction(start, end)
        print(f"{name:10} Year fraction: {yf:.6f}")


if __name__ == "__main__":
    main()
```

## Best Practices

### Use Context Managers Where Appropriate
```python
# Future: When file I/O or resources are added
class MarketDataContext:
    """Context manager for market data operations."""
    
    def __enter__(self):
        # Setup market data connections
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        # Cleanup resources
        pass

# Usage
with MarketDataContext() as market:
    curve = market.get_discount_curve("USD-OIS")
    df = curve.discount_factor(2.0)
```

### Prefer Explicit Construction
```python
# Good: Explicit construction
usd = Currency.usd()
money = Money(100.0, usd)

# Also good: Direct construction when clear
payment = Money(100.0, Currency.usd())

# Avoid: Ambiguous construction
create_payment(100.0, "USD")  # What does "USD" mean here?
```

### Handle Financial Precision
```python
from decimal import Decimal, ROUND_HALF_UP

def round_money(amount: Money, decimals: int = 2) -> Money:
    """Round money to specified decimal places using banker's rounding."""
    # Convert to Decimal for precise rounding
    decimal_amount = Decimal(str(amount.amount))
    rounded = decimal_amount.quantize(
        Decimal(10) ** -decimals,
        rounding=ROUND_HALF_UP
    )
    return Money(float(rounded), amount.currency)

# Example usage
original = Money(100.12579, Currency.usd())
rounded = round_money(original, 2)  # 100.13
```

## Performance Considerations

### Batch Operations
```python
# Good: Batch operations when possible
def calculate_portfolio_value(positions: List[Tuple[float, Currency]]) -> dict:
    """Calculate total value by currency."""
    totals = {}
    
    for amount, currency in positions:
        if currency not in totals:
            totals[currency] = Money(0.0, currency)
        totals[currency] = totals[currency] + Money(amount, currency)
    
    return totals

# Avoid: Repeated single operations in tight loops
# Bad example - creates many intermediate objects
total = Money(0.0, Currency.usd())
for i in range(10000):
    total = total + Money(0.01, Currency.usd())
```

### Cache Reusable Objects
```python
from functools import lru_cache

@lru_cache(maxsize=128)
def get_day_count(convention_name: str) -> DayCount:
    """Get cached day count convention by name."""
    conventions = {
        "30/360": DayCount.thirty360(),
        "ACT/360": DayCount.act360(),
        "ACT/365F": DayCount.act365f(),
        "ACT/ACT": DayCount.actact(),
        "30E/360": DayCount.thirty_e_360()
    }
    return conventions[convention_name]

# Usage - convention objects are cached
dc1 = get_day_count("30/360")
dc2 = get_day_count("30/360")  # Returns cached instance
``` # Python Usage Code Standards for rfin

## Overview

This document covers standards for Python code that uses the rfin library, including:
- Example scripts
- Test files
- Documentation examples
- User-facing Python code

## Import Conventions

### Standard Import Order
```python
# Standard library imports
import datetime
from typing import List, Optional, Tuple

# Third-party imports
import numpy as np
import pandas as pd
import pytest

# rfin imports - top-level classes
from rfin import Currency, Money, Date

# rfin imports - submodules
from rfin.dates import Calendar, DayCount, Frequency, generate_schedule
from rfin.cashflow import FixedRateLeg, CashFlow
```

### Import Aliases
```python
# Avoid aliases for rfin types to maintain clarity
# Good
from rfin import Currency, Money

# Avoid
from rfin import Currency as Ccy  # Don't do this
```

## Type Hints

### Always Use Type Hints
```python
from typing import List, Optional, Tuple
from rfin import Currency, Money, Date
from rfin.dates import DayCount, Calendar

def calculate_payment(
    principal: float,
    rate: float,
    currency: Currency,
    day_count: DayCount
) -> Money:
    """Calculate a payment amount."""
    return Money(principal * rate, currency)

def generate_payment_dates(
    start: Date,
    end: Date,
    frequency: Frequency,
    calendar: Optional[Calendar] = None
) -> List[Date]:
    """Generate payment dates with optional calendar adjustment."""
    dates = generate_schedule(start, end, frequency)
    if calendar:
        return [calendar.adjust(d, BusDayConvention.Following) for d in dates]
    return dates
```

## Error Handling

### Explicit Error Handling
```python
from rfin import Currency, Money

def safe_money_operation(amount1: Money, amount2: Money) -> Optional[Money]:
    """Safely add two money amounts, returning None on currency mismatch."""
    try:
        return amount1 + amount2
    except ValueError as e:
        if "Currency mismatch" in str(e):
            print(f"Warning: Cannot add {amount1.currency} and {amount2.currency}")
            return None
        raise  # Re-raise unexpected errors

# Or use explicit checking
def explicit_currency_check(amount1: Money, amount2: Money) -> Money:
    """Add two amounts with explicit currency validation."""
    if amount1.currency != amount2.currency:
        raise ValueError(
            f"Cannot add {amount1.currency.code} and {amount2.currency.code}"
        )
    return amount1 + amount2
```

## Testing Standards

### Test Structure
```python
import pytest
from datetime import date
from rfin import Currency, Money, Date
from rfin.dates import DayCount, Calendar, Frequency

class TestMoneyOperations:
    """Test suite for Money operations."""
    
    @pytest.fixture
    def usd(self) -> Currency:
        """USD currency fixture."""
        return Currency.usd()
    
    @pytest.fixture
    def eur(self) -> Currency:
        """EUR currency fixture."""
        return Currency.eur()
    
    def test_money_creation(self, usd: Currency):
        """Test creating Money instances."""
        money = Money(100.0, usd)
        assert money.amount == 100.0
        assert money.currency == usd
    
    def test_money_addition_same_currency(self, usd: Currency):
        """Test adding money with same currency."""
        money1 = Money(100.0, usd)
        money2 = Money(50.0, usd)
        result = money1 + money2
        assert result.amount == 150.0
        assert result.currency == usd
    
    def test_money_addition_different_currency_raises(
        self, 
        usd: Currency, 
        eur: Currency
    ):
        """Test that adding different currencies raises ValueError."""
        money1 = Money(100.0, usd)
        money2 = Money(50.0, eur)
        
        with pytest.raises(ValueError, match="Currency mismatch"):
            _ = money1 + money2
    
    @pytest.mark.parametrize("amount,expected_str", [
        (100.0, "100.00 USD"),
        (99.99, "99.99 USD"),
        (1000.50, "1000.50 USD"),
    ])
    def test_money_string_representation(
        self, 
        usd: Currency,
        amount: float,
        expected_str: str
    ):
        """Test string representation of Money."""
        money = Money(amount, usd)
        # Note: Actual format may differ, adjust as needed
        assert f"{money.amount:.2f} {money.currency}" == expected_str
```

### Test Naming Conventions
```python
# Test method names should be descriptive
def test_fixed_rate_leg_generates_correct_number_of_flows():
    """Test that a fixed rate leg generates the expected number of cash flows."""
    pass

def test_calendar_adjustment_following_moves_weekend_to_monday():
    """Test that Following convention moves Saturday/Sunday to Monday."""
    pass

# Use descriptive names for test data
def test_day_count_calculations():
    """Test day count convention calculations."""
    # Good: Descriptive variable names
    start_date = Date(2023, 1, 1)
    end_date = Date(2023, 7, 1)
    expected_act360_year_fraction = 0.5027777777777778
    
    # Calculate and assert
    dc = DayCount.act360()
    actual = dc.year_fraction(start_date, end_date)
    assert abs(actual - expected_act360_year_fraction) < 1e-10
```

## Documentation Examples

### Docstring Examples
```python
def create_swap_leg(
    notional: float,
    currency: Currency,
    fixed_rate: float,
    start_date: Date,
    maturity: Date,
    payment_frequency: Frequency = Frequency.SemiAnnual,
    day_count: DayCount = DayCount.thirty360()
) -> FixedRateLeg:
    """
    Create a fixed-rate swap leg.
    
    Args:
        notional: The notional principal amount
        currency: The currency of the swap leg
        fixed_rate: The fixed interest rate (e.g., 0.05 for 5%)
        start_date: The start date of the swap
        maturity: The maturity date of the swap
        payment_frequency: Payment frequency (default: semi-annual)
        day_count: Day count convention (default: 30/360)
    
    Returns:
        A FixedRateLeg object representing the swap leg
    
    Examples:
        >>> # Create a 5-year USD swap leg paying 3% semi-annually
        >>> leg = create_swap_leg(
        ...     notional=10_000_000,
        ...     currency=Currency.usd(),
        ...     fixed_rate=0.03,
        ...     start_date=Date(2023, 1, 1),
        ...     maturity=Date(2028, 1, 1),
        ...     payment_frequency=Frequency.SemiAnnual,
        ...     day_count=DayCount.thirty360()
        ... )
        >>> leg.num_flows
        10  # 5 years * 2 payments per year
        
        >>> # Create a quarterly EUR swap leg
        >>> eur_leg = create_swap_leg(
        ...     notional=5_000_000,
        ...     currency=Currency.eur(),
        ...     fixed_rate=0.025,
        ...     start_date=Date(2023, 1, 1),
        ...     maturity=Date(2025, 1, 1),
        ...     payment_frequency=Frequency.Quarterly
        ... )
        >>> eur_leg.num_flows
        8  # 2 years * 4 payments per year
    """
    return FixedRateLeg(
        notional_amount=notional,
        currency=currency,
        rate=fixed_rate,
        start_date=start_date,
        end_date=maturity,
        frequency=payment_frequency,
        day_count=day_count
    )
```

## Example Script Structure

### Complete Example Script
```python
#!/usr/bin/env python3
"""
Example: Creating and analyzing a fixed-rate bond.

This example demonstrates how to:
1. Create a fixed-rate bond cash flow schedule
2. Calculate accrued interest
3. Compute present value
4. Handle different day count conventions
"""

from typing import List
import pandas as pd

from rfin import Currency, Money, Date
from rfin.dates import (
    Calendar, 
    DayCount, 
    Frequency, 
    BusDayConvention,
    generate_schedule
)
from rfin.cashflow import FixedRateLeg


def create_bond_cashflows(
    face_value: float,
    coupon_rate: float,
    issue_date: Date,
    maturity_date: Date,
    currency: Currency = Currency.usd(),
    frequency: Frequency = Frequency.SemiAnnual,
    day_count: DayCount = DayCount.thirty360(),
    calendar: Calendar = Calendar.from_id("usny")
) -> FixedRateLeg:
    """Create cash flows for a fixed-rate bond."""
    return FixedRateLeg(
        notional_amount=face_value,
        currency=currency,
        rate=coupon_rate,
        start_date=issue_date,
        end_date=maturity_date,
        frequency=frequency,
        day_count=day_count
    )


def analyze_bond(leg: FixedRateLeg, valuation_date: Date) -> pd.DataFrame:
    """Analyze bond cash flows and return summary DataFrame."""
    flows = leg.flows()
    
    data = []
    for i, flow in enumerate(flows, 1):
        data.append({
            "Payment #": i,
            "Date": flow.date,
            "Amount": flow.amount,
            "Currency": flow.currency.code,
            "Type": flow.kind
        })
    
    df = pd.DataFrame(data)
    
    # Add summary statistics
    print(f"\nBond Analysis as of {valuation_date}")
    print(f"Total cash flows: {len(flows)}")
    print(f"Accrued interest: {leg.accrued(valuation_date)}")
    print(f"Present value: {leg.npv()}")
    
    return df


def main():
    """Main example execution."""
    # Create a 5-year corporate bond
    bond = create_bond_cashflows(
        face_value=1_000_000,      # $1M face value
        coupon_rate=0.045,         # 4.5% coupon
        issue_date=Date(2023, 1, 15),
        maturity_date=Date(2028, 1, 15),
        currency=Currency.usd(),
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360()
    )
    
    # Analyze the bond
    valuation_date = Date(2023, 3, 1)
    df = analyze_bond(bond, valuation_date)
    
    # Display results
    print("\nCash Flow Schedule:")
    print(df.to_string(index=False))
    
    # Compare day count conventions
    print("\nDay Count Convention Comparison:")
    conventions = [
        ("30/360", DayCount.thirty360()),
        ("ACT/360", DayCount.act360()),
        ("ACT/365F", DayCount.act365f()),
        ("ACT/ACT", DayCount.actact())
    ]
    
    start = Date(2023, 1, 1)
    end = Date(2023, 7, 1)
    
    for name, dc in conventions:
        yf = dc.year_fraction(start, end)
        print(f"{name:10} Year fraction: {yf:.6f}")


if __name__ == "__main__":
    main()
```

## Best Practices

### Use Context Managers Where Appropriate
```python
# Future: When file I/O or resources are added
class MarketDataContext:
    """Context manager for market data operations."""
    
    def __enter__(self):
        # Setup market data connections
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        # Cleanup resources
        pass

# Usage
with MarketDataContext() as market:
    curve = market.get_discount_curve("USD-OIS")
    df = curve.discount_factor(2.0)
```

### Prefer Explicit Construction
```python
# Good: Explicit construction
usd = Currency.usd()
money = Money(100.0, usd)

# Also good: Direct construction when clear
payment = Money(100.0, Currency.usd())

# Avoid: Ambiguous construction
create_payment(100.0, "USD")  # What does "USD" mean here?
```

### Handle Financial Precision
```python
from decimal import Decimal, ROUND_HALF_UP

def round_money(amount: Money, decimals: int = 2) -> Money:
    """Round money to specified decimal places using banker's rounding."""
    # Convert to Decimal for precise rounding
    decimal_amount = Decimal(str(amount.amount))
    rounded = decimal_amount.quantize(
        Decimal(10) ** -decimals,
        rounding=ROUND_HALF_UP
    )
    return Money(float(rounded), amount.currency)

# Example usage
original = Money(100.12579, Currency.usd())
rounded = round_money(original, 2)  # 100.13
```

## Performance Considerations

### Batch Operations
```python
# Good: Batch operations when possible
def calculate_portfolio_value(positions: List[Tuple[float, Currency]]) -> dict:
    """Calculate total value by currency."""
    totals = {}
    
    for amount, currency in positions:
        if currency not in totals:
            totals[currency] = Money(0.0, currency)
        totals[currency] = totals[currency] + Money(amount, currency)
    
    return totals

# Avoid: Repeated single operations in tight loops
# Bad example - creates many intermediate objects
total = Money(0.0, Currency.usd())
for i in range(10000):
    total = total + Money(0.01, Currency.usd())
```

### Cache Reusable Objects
```python
from functools import lru_cache

@lru_cache(maxsize=128)
def get_day_count(convention_name: str) -> DayCount:
    """Get cached day count convention by name."""
    conventions = {
        "30/360": DayCount.thirty360(),
        "ACT/360": DayCount.act360(),
        "ACT/365F": DayCount.act365f(),
        "ACT/ACT": DayCount.actact(),
        "30E/360": DayCount.thirty_e_360()
    }
    return conventions[convention_name]

# Usage - convention objects are cached
dc1 = get_day_count("30/360")
dc2 = get_day_count("30/360")  # Returns cached instance
``` 