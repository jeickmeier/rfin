1. Currencies and Money
=======================

Learn the fundamentals of currency-safe financial computations.

Learning Objectives
-------------------

By the end of this tutorial, you will:

* Create Currency and Money objects
* Perform safe arithmetic operations
* Understand currency mismatch errors
* Format money for display

What You'll Build
-----------------

A simple currency calculator that enforces type safety.

Creating Currencies
-------------------

finstack supports 180+ ISO 4217 currencies:

.. code-block:: python

   from finstack import Currency

   # Create from ISO code
   usd = Currency.from_code("USD")
   eur = Currency.from_code("EUR")
   gbp = Currency.from_code("GBP")
   jpy = Currency.from_code("JPY")

   # Access currency properties
   print(f"Code: {usd.code}")              # USD
   print(f"Numeric: {usd.numeric_code}")   # 840
   print(f"Decimals: {usd.decimals}")      # 2

   # Check for equality
   usd2 = Currency.from_code("USD")
   assert usd.equals(usd2)

**Invalid Currency Codes**:

.. code-block:: python

   try:
       invalid = Currency.from_code("XXX")
   except ValueError as e:
       print(f"Error: {e}")  # Unknown currency code: XXX

Creating Money
--------------

Money combines an amount with a currency:

.. code-block:: python

   from finstack import Money

   # Create from amount and currency
   amount1 = Money(1000.50, usd)

   # Or use the convenience method
   amount2 = Money.from_code(1000.50, "USD")

   # Access properties
   print(f"Amount: {amount1.amount}")         # 1000.5
   print(f"Currency: {amount1.currency.code}") # USD

Arithmetic Operations
---------------------

Addition and Subtraction
~~~~~~~~~~~~~~~~~~~~~~~~

Money arithmetic **requires matching currencies**:

.. code-block:: python

   usd1 = Money.from_code(1000.0, "USD")
   usd2 = Money.from_code(500.0, "USD")

   # ✅ Same currency - works
   total = usd1.add(usd2)
   print(f"Total: {total.amount} {total.currency.code}")  # 1500.0 USD

   difference = usd1.subtract(usd2)
   print(f"Difference: {difference.amount}")  # 500.0

   # ❌ Different currencies - raises error
   eur_amount = Money.from_code(100.0, "EUR")
   try:
       bad = usd1.add(eur_amount)
   except Exception as e:
       print(f"Error: {e}")  # Currency mismatch: USD vs EUR

Multiplication and Division
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Multiply/divide by scalars (not other Money):

.. code-block:: python

   base = Money.from_code(100.0, "USD")

   # Multiply by scalar
   doubled = base.multiply(2.0)
   print(f"Doubled: {doubled.amount}")  # 200.0

   # Divide by scalar
   halved = base.divide(2.0)
   print(f"Halved: {halved.amount}")  # 50.0

   # ❌ Cannot multiply two Money objects
   # bad = base.multiply(doubled)  # Type error

Negation
~~~~~~~~

.. code-block:: python

   amount = Money.from_code(100.0, "USD")
   negative = amount.negate()
   print(f"Negative: {negative.amount}")  # -100.0

Comparison
----------

Compare amounts (same currency only):

.. code-block:: python

   m1 = Money.from_code(100.0, "USD")
   m2 = Money.from_code(200.0, "USD")
   m3 = Money.from_code(100.0, "USD")

   # Equality
   assert not m1.equals(m2)
   assert m1.equals(m3)

   # Note: Comparison operators (<, >, <=, >=) may not be exposed yet
   # Use amount property for now:
   assert m1.amount < m2.amount

Formatting
----------

Display money with proper formatting:

.. code-block:: python

   amount = Money.from_code(1234567.89, "USD")

   # Basic string representation
   print(str(amount))  # Implementation-dependent

   # Manual formatting
   print(f"{amount.currency.code} {amount.amount:,.2f}")
   # USD 1,234,567.89

   # Different currencies have different decimal places
   jpy_amount = Money.from_code(1234567, "JPY")
   print(f"{jpy_amount.currency.code} {jpy_amount.amount:,.0f}")
   # JPY 1,234,567 (no decimals)

Working with Zero
-----------------

Create zero amounts:

.. code-block:: python

   zero = Money.from_code(0.0, "USD")

   # Check for zero
   if zero.amount == 0.0:
       print("Amount is zero")

   # Zero + anything = anything
   result = zero.add(Money.from_code(100.0, "USD"))
   assert result.amount == 100.0

Immutability
------------

Money objects are **immutable** - operations return new objects:

.. code-block:: python

   original = Money.from_code(100.0, "USD")
   doubled = original.multiply(2.0)

   # original is unchanged
   assert original.amount == 100.0
   assert doubled.amount == 200.0

   # No mutation methods exist
   # original.amount = 200.0  # AttributeError

Practice Exercise
-----------------

**Task**: Build a simple expense tracker.

.. code-block:: python

   from finstack import Money, Currency

   usd = Currency.from_code("USD")

   # Track expenses
   groceries = Money.from_code(150.75, "USD")
   gas = Money.from_code(45.50, "USD")
   utilities = Money.from_code(120.00, "USD")

   # Calculate total
   total = groceries.add(gas).add(utilities)
   print(f"Total expenses: ${total.amount:.2f}")

   # Calculate average
   average = total.divide(3.0)
   print(f"Average: ${average.amount:.2f}")

   # Check if over budget
   budget = Money.from_code(300.0, "USD")
   over_budget = total.subtract(budget)
   if over_budget.amount > 0:
       print(f"Over budget by: ${over_budget.amount:.2f}")
   else:
       print("Under budget!")

Key Takeaways
-------------

* **Currency safety**: Cannot mix currencies without explicit conversion
* **Immutability**: Operations return new Money objects
* **Type safety**: Only valid operations are allowed (no Money * Money)
* **Precision**: Decimal arithmetic (no floating-point errors)

Common Pitfalls
---------------

1. **Mixing currencies**: Always check currency before arithmetic
2. **Mutating expectations**: Money is immutable
3. **Comparison**: Use ``.equals()`` for currency-aware comparison
4. **Formatting**: Respect currency decimal places

Next Steps
----------

* Learn about :doc:`02_dates_and_calendars` for time-based calculations
* Explore :doc:`../core_concepts` for currency conversion with FX
* Review the :doc:`../../api/core/money` API reference

Next: :doc:`02_dates_and_calendars`
