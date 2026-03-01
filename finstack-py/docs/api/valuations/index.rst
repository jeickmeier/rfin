Valuations API
==============

The valuations module provides instrument pricing, risk metrics, and calibration.

.. toctree::
   :maxdepth: 2

   instruments
   pricer_registry
   calibration
   metrics
   cashflows

Overview
--------

The valuations module includes:

* **40+ Instruments**: Fixed income, equity, FX, credit, commodities, structured products
* **Pricing Models**: Analytical, Monte Carlo, trees
* **Risk Metrics**: DV01, CS01, Greeks (Delta/Gamma/Vega/Theta/Rho)
* **Calibration**: Bootstrap curves from market quotes
* **Cashflow Builders**: Schedule-based cashflow generation

Instruments
-----------

Fixed Income
~~~~~~~~~~~~

.. automodule:: finstack.instruments
   :members: Bond, InterestRateSwap, Deposit, BondFuture, CrossCurrencySwap, InflationCapFloor
   :undoc-members:
   :show-inheritance:

Example:

.. code-block:: python

   from finstack import Bond, Money, Date

   bond = Bond.fixed_semiannual(
       id="US912828A",
       notional=Money.from_code(1_000_000, "USD"),
       rate=0.05,
       issue_date=Date(2024, 1, 15),
       maturity_date=Date(2029, 1, 15),
       discount_curve_id="USD.OIS"
   )

Equity and FX
~~~~~~~~~~~~~

.. automodule:: finstack.instruments
   :members: EquityOption, EquityForward, EquityIndexFuture, FxForward, FxSwap, NonDeliverableForward
   :undoc-members:
   :show-inheritance:

Example:

.. code-block:: python

   from finstack import EquityOption, OptionType

   option = EquityOption(
       id="SPY_CALL_500",
       underlying="SPY",
       option_type=OptionType.Call,
       strike=500.0,
       expiry=Date(2024, 12, 20),
       notional=Money.from_code(100_000, "USD")
   )

Credit
~~~~~~

.. automodule:: finstack.instruments
   :members: CreditDefaultSwap, CdsIndex, TotalReturnSwap
   :undoc-members:
   :show-inheritance:

Structured Products
~~~~~~~~~~~~~~~~~~~

.. automodule:: finstack.instruments
   :members: CallableBond, CollateralizedLoanObligation, MortgageBackedSecurity, RevolvingCreditFacility
   :undoc-members:
   :show-inheritance:

Alternative Assets
~~~~~~~~~~~~~~~~~~

.. automodule:: finstack.instruments
   :members: RealEstateAsset, PrivateMarketsFund, CommodityOption
   :undoc-members:
   :show-inheritance:

Pricer Registry
---------------

.. automodule:: finstack.pricer
   :members: PricerRegistry, ModelKey, ValuationResult
   :undoc-members:
   :show-inheritance:

The pricer registry provides type-safe dispatch from (InstrumentType, ModelKey) to pricers.

Example:

.. code-block:: python

   from finstack.valuations.common import ModelKey
   from finstack.valuations.pricer import create_standard_registry

   registry = create_standard_registry()

   # Analytical pricing
   result_analytical = registry.price_barrier_option(
       option, ModelKey.BarrierBSContinuous(), market, []
   )

   # Monte Carlo pricing
   result_mc = registry.price_barrier_option(
       option, ModelKey.MonteCarloGBM(), market, []
   )

Available Models:

* **Analytical**: BarrierBSContinuous, AsianGeometricBS, AsianTurnbullWakeman, LookbackBSContinuous, QuantoBS
* **Monte Carlo**: MonteCarloGBM, MonteCarloHeston, MonteCarloHullWhite1F
* **Advanced**: HestonFourier, Normal

Calibration
-----------

.. automodule:: finstack.calibration
   :members: execute_calibration, CalibrationConfig, RatesQuote, CreditQuote, VolQuote, InflationQuote
   :undoc-members:
   :show-inheritance:

Bootstrap market curves from quotes using the plan-driven API.

Example:

.. code-block:: python

   from finstack import execute_calibration, RatesQuote

   quotes = [
       RatesQuote.deposit_rate(tenor="3M", rate=0.045),
       RatesQuote.swap_rate(tenor="1Y", rate=0.048),
       RatesQuote.swap_rate(tenor="5Y", rate=0.050),
   ]

   plan = {
       "steps": [{
           "kind": "discount",
           "curve_id": "USD.OIS",
           "base_date": "2024-01-15",
           "quotes": [q.to_json() for q in quotes]
       }]
   }

   result = execute_calibration(plan, {})
   curve = result.curves["USD.OIS"]

Metrics
-------

.. automodule:: finstack.metrics
   :members: MetricId, MetricRegistry
   :undoc-members:
   :show-inheritance:

Risk metrics and analytics.

Available Metrics:

* **Price**: clean_price, dirty_price, fair_value
* **Accruals**: accrued, accrued_interest
* **Yields**: ytm, current_yield, yield_to_call
* **Duration**: duration_macaulay, duration_mod, convexity
* **Risk**: dv01, cs01, theta, delta, gamma, vega, rho
* **Spreads**: z_spread, oas, asw_spread

Example:

.. code-block:: python

   result = registry.price_with_metrics(
       bond,
       "discounting",
       market,
       ["clean_price", "accrued_interest", "ytm", "dv01"],
   )

   print(f"Clean Price: {result.measures['clean_price']:.4f}")
   print(f"DV01: {result.measures['dv01']:.2f}")

Cashflows
---------

.. automodule:: finstack.cashflows
   :members: CashflowBuilder, CashFlowSchedule, AmortizationType, CouponType
   :undoc-members:
   :show-inheritance:

Schedule-based cashflow generation with amortization and coupon types.

Example:

.. code-block:: python

   from finstack import CashflowBuilder, AmortizationType

   builder = CashflowBuilder()
   builder.notional(1_000_000.0)
   builder.currency(Currency.from_code("USD"))
   builder.rate(0.05)
   builder.start_date(Date(2024, 1, 15))
   builder.end_date(Date(2029, 1, 15))
   builder.frequency(Tenor.SemiAnnual())
   builder.amortization(AmortizationType.Bullet())

   schedule = builder.build()
   flows = schedule.flows()

See Also
--------

* :doc:`../../tutorials/intermediate/01_exotic_options`
* :doc:`../../tutorials/intermediate/03_curve_calibration`
* :doc:`../../examples/phase1_instruments/index`
