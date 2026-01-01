Quickstart Guide
================

This guide will walk you through the basics of using finstack-py for financial computations.

Basic Concepts
--------------

Currency and Money
~~~~~~~~~~~~~~~~~~

finstack enforces currency safety - you cannot accidentally add USD and EUR without explicit conversion.

.. code-block:: python

   from finstack import Currency, Money

   # Create currencies
   usd = Currency.from_code("USD")
   eur = Currency.from_code("EUR")

   # Create money amounts
   amount1 = Money.from_code(1000.0, "USD")
   amount2 = Money.from_code(500.0, "USD")

   # Safe arithmetic (same currency)
   total = amount1.add(amount2)
   print(f"Total: {total.amount} {total.currency.code}")  # 1500.0 USD

   # This would raise an error (currency mismatch)
   # bad = Money.from_code(100.0, "USD").add(Money.from_code(100.0, "EUR"))

Dates and Day Counts
~~~~~~~~~~~~~~~~~~~~~

Financial date arithmetic with business day conventions and day count conventions.

.. code-block:: python

   from finstack import Date, DayCount, Period

   # Create dates
   start = Date(2024, 1, 15)
   end = Date(2024, 7, 15)

   # Calculate year fraction using Act/360
   dc = DayCount.act360()
   yf = dc.year_fraction(start, end)
   print(f"Year fraction: {yf:.6f}")

   # Work with periods
   period = Period.from_string("6M")
   maturity = start.add_period(period)
   print(f"Maturity: {maturity}")

Pricing a Simple Bond
----------------------

Let's price a 5-year fixed-rate bond.

Step 1: Create the Instrument
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack import Bond, Money, Date

   notional = Money.from_code(1_000_000, "USD")
   issue = Date(2024, 1, 15)
   maturity = Date(2029, 1, 15)

   bond = Bond.fixed_semiannual(
       id="US912828A",
       notional=notional,
       rate=0.05,  # 5% coupon
       issue_date=issue,
       maturity_date=maturity,
       discount_curve_id="USD.OIS"
   )

   print(f"Bond: {bond.id}")
   print(f"Coupon: {bond.coupon_rate * 100}%")

Step 2: Create Market Data
~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack import DiscountCurve, MarketContext, Tenor

   # Create a discount curve
   as_of = Date(2024, 1, 15)
   tenors = [Tenor.from_string(t) for t in ["1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y"]]
   rates = [0.045, 0.046, 0.047, 0.048, 0.049, 0.050, 0.051]

   curve = DiscountCurve.from_zero_rates(
       curve_id="USD.OIS",
       base_date=as_of,
       tenors=tenors,
       rates=rates,
       currency=Currency.from_code("USD")
   )

   # Create market context
   market = MarketContext()
   market.insert_discount(curve)

Step 3: Price the Bond
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack.valuations.pricer import create_standard_registry

   registry = create_standard_registry()

   # Price with metrics
   result = registry.price_with_metrics(
       bond,
       "discounting",
       market,
       ["clean_price", "accrued_interest", "ytm", "dv01"],
   )

   print(f"\nBond Valuation:")
   print(f"Present Value: {result.value.amount:,.2f}")
   print(f"Clean Price:   {result.measures['clean_price']:.4f}")
   print(f"Accrued:       {result.measures['accrued_interest']:,.2f}")
   print(f"Yield to Mat:  {result.measures['ytm'] * 100:.4f}%")
   print(f"DV01:          {result.measures['dv01']:,.2f}")

Running Scenarios
-----------------

Stress test the bond under different rate scenarios.

.. code-block:: python

   from datetime import date
   from finstack.scenarios import ExecutionContext, ScenarioEngine
   from finstack.scenarios.dsl import from_dsl
   from finstack.statements.types import FinancialModelSpec

   # Create a rates shock scenario
   scenario = from_dsl("""
       # Shock discount curve by +50bp
       shift USD.OIS +50bp
   """)

   # Apply scenario
   ctx = ExecutionContext(market, FinancialModelSpec("empty", []), date(2024, 1, 15))
   engine = ScenarioEngine()
   report = engine.apply(scenario, ctx)
   shocked_market = ctx.market

   # Re-price under stress
   stressed_result = registry.price_with_metrics(
       bond,
       "discounting",
       shocked_market,
       ["clean_price", "dv01"],
   )

   # Compare results
   base_pv = result.value.amount
   stressed_pv = stressed_result.value.amount
   pnl = stressed_pv - base_pv

   print(f"\nScenario Analysis:")
   print(f"Base PV:      {base_pv:,.2f}")
   print(f"Stressed PV:  {stressed_pv:,.2f}")
   print(f"P&L:          {pnl:,.2f}")

Building a Portfolio
--------------------

Aggregate multiple positions.

.. code-block:: python

   from finstack import (
       Portfolio, PortfolioBuilder, Entity,
       Deposit, InterestRateSwap
   )

   # Create entities
   fund = Entity(id="FUND001", name="Alpha Fund")

   # Build portfolio
   builder = PortfolioBuilder()
   builder.entity(fund)
   builder.base_ccy(Currency.from_code("USD"))
   builder.as_of(as_of)

   # Add bond position
   builder.position_from_instrument(
       position_id="POS_BOND",
       instrument=bond,
       quantity=10.0,  # 10 bonds
       entity_id="FUND001"
   )

   # Add deposit position
   deposit = Deposit(...)
   builder.position_from_instrument(
       position_id="POS_DEPOSIT",
       instrument=deposit,
       quantity=1.0,
       entity_id="FUND001"
   )

   portfolio = builder.build()

   # Value the portfolio
   from finstack import value_portfolio

   valuation = value_portfolio(portfolio, market, None)

   print(f"\nPortfolio Valuation:")
   print(f"Total Value: {valuation.total_value.amount:,.2f}")
   for entity_id, value in valuation.by_entity.items():
       print(f"  {entity_id}: {value.amount:,.2f}")

Next Steps
----------

* Learn about :doc:`core_concepts` like determinism and currency safety
* Explore :doc:`beginner/index` tutorials for instrument-by-instrument guides
* Check the :doc:`../examples/cookbook/index` for common workflows
* Review the full :doc:`../api/index` API reference

Common Patterns
---------------

Working with DataFrames
~~~~~~~~~~~~~~~~~~~~~~~~

Export results to Polars or Pandas for analysis:

.. code-block:: python

   # Export to Polars
   df_polars = valuation.to_polars()
   df_polars.write_csv("portfolio.csv")

   # Or to Pandas
   df_pandas = valuation.to_pandas()
   df_pandas.to_excel("portfolio.xlsx")

Batch Pricing
~~~~~~~~~~~~~

Price multiple instruments efficiently:

.. code-block:: python

   instruments = [bond1, bond2, bond3]
   results = [
       registry.price(bond, "discounting", market)
       for bond in instruments
   ]

Configuration
~~~~~~~~~~~~~

Set global precision and rounding:

.. code-block:: python

   from finstack import FinstackConfig, RoundingMode

   config = FinstackConfig()
   config.set_rounding_mode(RoundingMode.HalfEven)
   config.set_decimal_places(4)
