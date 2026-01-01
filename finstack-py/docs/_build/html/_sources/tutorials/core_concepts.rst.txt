Core Concepts
=============

This page explains the fundamental design principles of finstack-py.

Determinism
-----------

finstack guarantees **bit-for-bit reproducible results**:

* Same inputs → same outputs (every time, any platform)
* Serial and parallel execution produce identical results
* Explicit random seeds for Monte Carlo simulations

**Why it matters**: Regulatory compliance, debugging, production consistency.

.. code-block:: python

   from finstack import PricerRegistry

   # Same instrument, market, config → always same result
   result1 = registry.price_bond(bond, "discounting", market)
   result2 = registry.price_bond(bond, "discounting", market)

   assert result1.present_value.amount == result2.present_value.amount

Currency Safety
---------------

Cross-currency arithmetic requires **explicit conversion**:

.. code-block:: python

   from finstack import Money, FxMatrix

   usd_amount = Money.from_code(100.0, "USD")
   eur_amount = Money.from_code(100.0, "EUR")

   # ❌ This raises an error
   # total = usd_amount.add(eur_amount)

   # ✅ Explicit FX conversion
   fx_matrix = FxMatrix()
   fx_matrix.set_rate("USD", "EUR", 0.92)

   eur_converted = fx_matrix.convert(usd_amount, Currency.from_code("EUR"))
   total = eur_converted.add(eur_amount)

**FX Policy Stamping**: All cross-currency operations record the conversion policy in results metadata.

.. code-block:: python

   from finstack import value_portfolio

   # Portfolio with mixed currencies
   valuation = value_portfolio(portfolio, market, None)

   # Check applied FX policy
   print(valuation.metadata.fx_policy)  # e.g., "TriangulationViaBase(USD)"

Decimal Precision
-----------------

finstack uses **Decimal arithmetic** by default (not IEEE 754 floats):

* No cumulative rounding errors
* Accounting-grade accuracy
* Explicit rounding policies

.. code-block:: python

   from finstack import Money

   # Decimal-based arithmetic
   amount = Money.from_code(0.1, "USD")
   total = amount
   for _ in range(10):
       total = total.add(amount)

   # Exactly 1.1 (no floating-point drift)
   assert total.amount == 1.1

**Rounding Context**: All operations stamp their rounding mode and scale in results.

.. code-block:: python

   from finstack import FinstackConfig, RoundingMode

   config = FinstackConfig()
   config.set_rounding_mode(RoundingMode.HalfEven)
   config.set_decimal_places(4)

   # Results include rounding metadata
   result = registry.price_bond_with_config(bond, "discounting", market, config)
   print(result.metadata.rounding_context)

Stable Wire Formats
-------------------

All public types support **JSON serialization** with stable field names:

* Configuration files version-controlled
* Golden test baselines never break
* Pipeline integration without surprises

.. code-block:: python

   from finstack import Bond

   bond = Bond.fixed_semiannual(...)

   # Serialize to JSON
   json_str = bond.to_json()

   # Deserialize from JSON
   bond2 = Bond.from_json(json_str)

   # Same bond
   assert bond.id == bond2.id

**Versioning**: Breaking changes in JSON schema are major version bumps.

No Implicit Operations
----------------------

finstack requires **explicit decisions** for:

* FX conversion (no auto-conversion)
* Rate compounding (annual/semiannual/continuous explicit)
* Day count conventions (no defaults)
* Business day adjustments (calendar + rule required)

.. code-block:: python

   from finstack import DayCount, BusDayConvention, Calendar

   # ✅ Explicit day count
   dc = DayCount.act360()
   yf = dc.year_fraction(start, end)

   # ✅ Explicit calendar + convention
   calendar = Calendar.from_id("nyse")
   adjusted = calendar.adjust(date, BusDayConvention.ModifiedFollowing)

**Why**: Prevents silent errors from incorrect assumptions.

Pricing Models Registry
------------------------

The **PricerRegistry** maps (InstrumentType, ModelKey) → pricer:

* Type-safe dispatch (no runtime model string matching)
* Compile-time exhaustiveness checks
* Extensible for custom pricers

.. code-block:: python

   from finstack import PricerRegistry, ModelKey

   registry = PricerRegistry.create_standard()

   # Analytical pricing (default)
   result_analytical = registry.price_barrier_option(
       option, ModelKey.BarrierBSContinuous(), market, []
   )

   # Monte Carlo pricing
   result_mc = registry.price_barrier_option(
       option, ModelKey.MonteCarloGBM(), market, []
   )

   # Both use the same Instrument, different pricers

**Available Models**:

* Analytical: BarrierBSContinuous, AsianGeometricBS, LookbackBSContinuous
* Monte Carlo: MonteCarloGBM, MonteCarloHeston, MonteCarloHullWhite1F
* Trees: BinomialTree, TrinomialTree
* Advanced: HestonFourier, SABRImpliedVol

Bucketed Risk Metrics
---------------------

Risk metrics can be **scalar** or **bucketed**:

.. code-block:: python

   # Scalar DV01 (total sensitivity)
   dv01_scalar = result.metric("dv01")  # e.g., 450.23

   # Bucketed DV01 (per tenor)
   # Note: Currently requires manual bumping (see Task 1.5 notes)
   dv01_bucketed = compute_bucketed_dv01(bond, market, tenors)
   # { "1Y": 50.0, "2Y": 120.0, "5Y": 280.23 }

**Future**: Native bucketed metrics via `metric_bucketed()` (blocked on Rust core changes).

Performance
-----------

finstack balances **correctness and speed**:

* Rust core (10-100x faster than pure Python)
* GIL release for parallelism
* Vectorized operations where possible
* Zero-copy data transfer for large arrays

.. code-block:: python

   # Batch pricing releases GIL
   instruments = [bond1, bond2, ..., bond1000]
   results = [
       registry.price_bond(bond, "discounting", market)
       for bond in instruments
   ]
   # Parallelizes across CPU cores

**Benchmarks**: See :doc:`../reference/benchmarks` for performance data.

Error Handling
--------------

finstack uses **granular error types**:

.. code-block:: python

   from finstack import CurrencyMismatchError, MarketDataNotFoundError

   try:
       result = registry.price_bond(bond, "discounting", market)
   except MarketDataNotFoundError as e:
       print(f"Missing curve: {e.curve_id}")
   except CurrencyMismatchError as e:
       print(f"Currency error: {e.expected} vs {e.found}")

**No Silent Failures**: Every error is actionable.

Extension Points
----------------

finstack provides hooks for customization:

* **Custom pricers**: Register new (InstrumentType, ModelKey) pairs
* **Custom metrics**: Add new risk calculations
* **Custom calendars**: Define holiday rules
* **Custom interpolation**: Implement InterpFn trait

.. code-block:: python

   # Example: Custom metric
   from finstack import MetricCalculator

   class MyMetric(MetricCalculator):
       def calculate(self, instrument, market):
           # Your logic here
           return value

   registry.register_metric("my_metric", MyMetric())

Next Steps
----------

Now that you understand the core principles:

* Try the :doc:`beginner/index` tutorials
* Explore real-world :doc:`../examples/cookbook/index`
* Dive into the :doc:`../api/index` API reference
