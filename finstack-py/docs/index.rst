finstack-py: Financial Computation Library
==========================================

.. toctree::
   :maxdepth: 2
   :caption: Getting Started
   :hidden:

   tutorials/installation
   tutorials/quickstart
   tutorials/core_concepts

.. toctree::
   :maxdepth: 2
   :caption: Tutorials
   :hidden:

   tutorials/beginner/index
   tutorials/intermediate/index
   tutorials/advanced/index

.. toctree::
   :maxdepth: 3
   :caption: API Reference
   :hidden:

   api/core/index
   api/valuations/index
   api/scenarios/index
   api/statements/index
   api/portfolio/index

.. toctree::
   :maxdepth: 1
   :caption: Examples
   :hidden:

   examples/cookbook/index
   examples/phase1_instruments/index
   examples/workflows/index

.. toctree::
   :maxdepth: 1
   :caption: Reference
   :hidden:

   reference/glossary
   reference/contributing
   reference/changelog

Overview
========

**finstack-py** is a comprehensive financial computation library providing:

* **Deterministic Pricing**: Accounting-grade precision with Decimal numerics
* **Currency Safety**: Explicit cross-currency handling with FX policies
* **40+ Instruments**: Fixed income, equity, FX, credit, commodities, structured products
* **Advanced Analytics**: Risk metrics (DV01/CS01/Greeks), calibration, scenarios, optimization
* **Performance**: Rust-powered computation with Python ergonomics

Quick Example
=============

.. code-block:: python

   from finstack import (
       Money, Date, Currency,
       Bond, MarketContext, DiscountCurve,
       PricerRegistry
   )

   # Create a 5-year bond
   notional = Money.from_code(1_000_000, "USD")
   issue = Date(2024, 1, 15)
   maturity = Date(2029, 1, 15)

   bond = Bond.fixed_semiannual(
       id="US912828A",
       notional=notional,
       rate=0.05,
       issue_date=issue,
       maturity_date=maturity,
       discount_curve_id="USD.OIS"
   )

   # Create market data
   curve = DiscountCurve(...)
   market = MarketContext()
   market.insert(curve)

   # Price the bond
   from finstack.valuations.pricer import standard_registry

   registry = standard_registry()
   result = registry.price_with_metrics(
       bond,
       "discounting",
       market,
       ["clean_price", "accrued_interest", "ytm", "dv01"],
   )

   print(f"PV: {result.value.amount:,.2f}")
   print(f"Clean Price: {result.measures['clean_price']:.4f}")
   print(f"DV01: {result.measures['dv01']:.2f}")

Key Features
============

Core Capabilities
-----------------

* **Currency & Money**: Type-safe currency operations with 180+ ISO currencies
* **Date & Time**: Business day conventions, calendars, day count conventions
* **Market Data**: Term structures (discount/forward/credit/inflation), FX, vol surfaces
* **Expression Engine**: Formula evaluation with vectorized execution

Instrument Coverage
-------------------

Fixed Income
~~~~~~~~~~~~

* Bonds (fixed/floating/zero-coupon/structured)
* Interest rate swaps (vanilla/basis/OIS/amortizing)
* Deposits and term loans
* Bond futures and options
* Cross-currency swaps
* Inflation-linked products

Equity & FX
~~~~~~~~~~~

* Equity options (vanilla/barrier/Asian/lookback/quanto)
* Equity forwards and futures
* FX forwards and swaps
* Non-deliverable forwards (NDF)
* Variance swaps

Credit
~~~~~~

* Credit default swaps (CDS)
* CDS indices and tranches
* Total return swaps (TRS)
* Convertible bonds

Structured Products
~~~~~~~~~~~~~~~~~~~

* Callable bonds
* Collateralized loan obligations (CLO)
* Mortgage-backed securities (MBS)
* Revolving credit facilities

Alternative Assets
~~~~~~~~~~~~~~~~~~

* Real estate (DCF and direct cap)
* Private equity funds
* Commodity options

Analytics & Valuation
---------------------

* **Pricing Models**: Analytical (Black-Scholes variants), Monte Carlo, trees
* **Risk Metrics**: DV01, CS01, Greeks (Delta/Gamma/Vega/Theta/Rho)
* **Calibration**: Bootstrap curves from market quotes
* **Scenarios**: Stress testing and what-if analysis
* **Portfolio**: Aggregation, netting, margin, optimization

Integration
-----------

* **DataFrames**: Polars and Pandas integration for data I/O
* **Serialization**: JSON-based configuration and results
* **Performance**: Rust core with GIL release for parallelism

User Personas
=============

This library is designed for:

* **Quantitative Analysts**: Need robust, tested pricing and risk analytics
* **Credit/Equity Analysts**: Portfolio management and covenant modeling
* **Risk Managers**: Stress testing and scenario analysis
* **Portfolio Managers**: Optimization and position tracking
* **Data Engineers**: Integration with data pipelines

Indices and Tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
