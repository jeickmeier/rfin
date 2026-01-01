Scenarios API
=============

The scenarios module provides stress testing and what-if analysis.

.. toctree::
   :maxdepth: 2

   spec
   engine
   dsl
   builder

Overview
--------

The scenarios module includes:

* **ScenarioSpec**: Declarative scenario definitions
* **ScenarioEngine**: Deterministic scenario execution
* **DSL Parser**: Text-based scenario syntax
* **Builder API**: Fluent scenario construction
* **Composition**: Merge and chain scenarios

Scenario Specification
----------------------

.. automodule:: finstack.scenarios
   :members: ScenarioSpec, OperationSpec
   :undoc-members:
   :show-inheritance:

Define scenarios declaratively with JSON or Python objects.

Example:

.. code-block:: python

   from finstack import ScenarioSpec, OperationSpec

   spec = ScenarioSpec(
       id="rates_shock",
       name="50bp Parallel Shift",
       operations=[
           OperationSpec.curve_parallel_bp(
               curve_id="USD.OIS",
               shift_bp=50.0
           )
       ]
   )

Scenario Engine
---------------

.. automodule:: finstack.scenarios
   :members: ScenarioEngine, ApplicationReport
   :undoc-members:
   :show-inheritance:

Execute scenarios deterministically with metadata tracking.

Example:

.. code-block:: python

   from finstack import ScenarioEngine

   engine = ScenarioEngine()
   shocked_market, report = engine.apply(spec, market)
   
   # Re-price under stress
   stressed_result = registry.price_bond(bond, "discounting", shocked_market)
   
   # Compare
   pnl = stressed_result.present_value.amount - base_result.present_value.amount

DSL Parser
----------

.. automodule:: finstack.scenarios.dsl
   :members:
   :undoc-members:

Parse text-based scenario syntax into ScenarioSpec.

Example:

.. code-block:: python

   spec = ScenarioSpec.from_dsl("""
       # Curve shifts
       shift USD.OIS +50bp
       shift EUR.OIS +40bp
       
       # Equity and FX
       shift equities -10%
       shift fx USD/EUR +3%
       
       # Time
       roll forward 1m
   """)

Builder API
-----------

.. automodule:: finstack.scenarios.builder
   :members: scenario
   :undoc-members:

Fluent API for scenario construction.

Example:

.. code-block:: python

   from finstack.scenarios.builder import scenario

   spec = (
       scenario("stress_test")
       .name("Q1 2024 Stress")
       .shift_discount_curve("USD.OIS", 50)
       .shift_equities(-10)
       .shift_fx("USD", "EUR", 3)
       .roll_forward("1m")
       .build()
   )

Composition
-----------

Merge multiple scenarios:

.. code-block:: python

   # Define base and overlay
   base = ScenarioSpec.from_dsl("shift USD.OIS +50bp")
   overlay = ScenarioSpec.from_dsl("shift equities -10%")
   
   # Compose (base applied first, then overlay)
   engine = ScenarioEngine()
   composed = engine.compose([base, overlay])
   
   # Apply composed scenario
   shocked_market, report = engine.apply(composed, market)

Operation Types
---------------

The following operation types are supported:

**Curve Operations**:

* ``shift_discount_curve(curve_id, shift_bp)`` - Parallel shift
* ``shift_forward_curve(curve_id, shift_bp)``
* ``shift_hazard_curve(curve_id, shift_bp)``
* ``shift_inflation_curve(curve_id, shift_bp)``

**Equity and FX**:

* ``shift_equities(shift_pct)`` - All equities
* ``shift_equity(ticker, shift_pct)`` - Single equity
* ``shift_fx(from_ccy, to_ccy, shift_pct)``

**Volatility**:

* ``shift_vol_surface(surface_id, shift_pct)``

**Time**:

* ``roll_forward(period)`` - Advance horizon

**Statements**:

* ``adjust_forecast(node_id, shift_pct)``
* ``set_forecast(node_id, value)``

See Also
--------

* :doc:`../../tutorials/beginner/06_basic_scenarios`
* :doc:`../../tutorials/intermediate/04_scenario_analysis`
* :doc:`../../examples/scenarios/index`
