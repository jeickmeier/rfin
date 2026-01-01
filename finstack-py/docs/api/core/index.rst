Core API
========

The core module provides fundamental types for financial computations.

.. toctree::
   :maxdepth: 2

   currency
   money
   dates
   periods
   market_data
   math
   expressions

Overview
--------

The core module includes:

* **Currency and Money**: Type-safe currency operations
* **Dates and Time**: Business days, calendars, day count conventions
* **Market Data**: Curves (discount/forward/credit), FX, volatility surfaces
* **Math**: Interpolation, solvers, integration, distributions
* **Expressions**: Formula evaluation engine

Currency and Money
------------------

.. automodule:: finstack.core
   :members: Currency, Money
   :undoc-members:
   :show-inheritance:

Type-safe currency operations enforcing no implicit cross-currency arithmetic.

Example:

.. code-block:: python

   from finstack import Currency, Money

   usd = Currency.from_code("USD")
   amount = Money(100.0, usd)
   
   # Safe arithmetic
   total = amount.add(Money(50.0, usd))  # Works
   
   # Cross-currency requires explicit FX
   eur = Currency.from_code("EUR")
   eur_amount = Money(50.0, eur)
   # amount.add(eur_amount)  # Raises CurrencyMismatchError

Dates and Time
--------------

.. automodule:: finstack.dates
   :members: Date, Period, Tenor, DayCount, Calendar, BusDayConvention
   :undoc-members:
   :show-inheritance:

Financial date arithmetic with business day adjustments and day count conventions.

Example:

.. code-block:: python

   from finstack import Date, DayCount, Period

   start = Date(2024, 1, 15)
   end = Date(2024, 7, 15)
   
   # Year fraction
   dc = DayCount.act360()
   yf = dc.year_fraction(start, end)
   
   # Add period
   period = Period.from_string("6M")
   future = start.add_period(period)

Market Data
-----------

.. automodule:: finstack.market_data
   :members: DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, MarketContext, FxMatrix
   :undoc-members:
   :show-inheritance:

Term structures and market data containers.

Example:

.. code-block:: python

   from finstack import DiscountCurve, MarketContext, Tenor

   # Create curve
   curve = DiscountCurve.from_zero_rates(
       curve_id="USD.OIS",
       base_date=Date(2024, 1, 1),
       tenors=[Tenor.from_string(t) for t in ["1M", "3M", "1Y"]],
       rates=[0.045, 0.046, 0.048],
       currency=Currency.from_code("USD")
   )
   
   # Add to market context
   market = MarketContext()
   market.insert_discount(curve)

Math Utilities
--------------

.. automodule:: finstack.math
   :members: InterpStyle, Interpolator, Solver, Integration
   :undoc-members:
   :show-inheritance:

Numerical methods for interpolation, root-finding, and integration.

Expressions
-----------

.. automodule:: finstack.expressions
   :members: Expr, ExprContext, ExprEvaluator
   :undoc-members:
   :show-inheritance:

Formula evaluation engine with scalar and vectorized execution.

See Also
--------

* :doc:`../../tutorials/beginner/01_currencies_and_money`
* :doc:`../../tutorials/beginner/02_dates_and_calendars`
* :doc:`../../examples/cookbook/market_data`
