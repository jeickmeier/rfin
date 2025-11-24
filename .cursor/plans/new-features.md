1. EBITDA Normalization & Adjustments Module
Current State: The system handles raw financials well, but private credit relies heavily on "Adjusted EBITDA". Recommendation: Implement a structured AdjustmentsBuilder or NormalizationEngine.

Features:
Add-backs Tracker: Explicitly track adjustments like "Synergies", "Management Fees", "One-time Legal Costs", and "Restructuring Charges".
Audit Trail: Maintain a lineage of Raw EBITDA -> Adjustments -> Pro Forma Adjusted EBITDA.
Capping Logic: Automatically cap certain add-backs (e.g., "Synergies capped at 20% of EBITDA" is a common credit agreement clause).
2. Cash Flow Waterfall & Sweep Mechanics
Current State: capital_structure tracks interest and principal but doesn't seem to model the flow of cash through the priority stack. Recommendation: Create a CashFlowWaterfall module.

Features:
Priority of Payments: Define the order: Fees -> Cash Interest -> Scheduled Amortization -> Mandatory Prepayments -> Voluntary Prepayments -> Equity Distributions.
Excess Cash Flow (ECF) Sweep: Automatically calculate ECF (EBITDA - Taxes - Capex - Interest - Working Capital) and apply the sweep percentage (e.g., 50% or 75%) to pay down the Term Loan.
PIK Toggles: Logic to switch between Cash and PIK interest based on liquidity conditions.
3. Scenario Manager & Stress Testing
Current State: corporate.rs has basic sensitivity analysis (WACC +/- 1%), but analysts need full multi-variable scenarios. Recommendation: Implement a ScenarioManager.

Features:
Case Management: Define "Base Case", "Sponsor Case", "Downside/Stress Case", and "Upside Case".
Variable Overrides: Allow overriding growth rates, margins, and interest rate curves per scenario.
Side-by-Side Comparison: Generate reports that show key credit stats (Leverage, Interest Coverage) across all scenarios in a single view.
4. Advanced Covenant Modeling
Current State: covenants.rs forecasts breaches based on simple thresholds. Recommendation: Enhance support for complex credit agreement structures.

Features:
Maintenance vs. Incurrence: Distinguish between covenants that are tested quarterly vs. those tested only upon taking an action (debt incurrence).
Springing Covenants: Logic for covenants that only activate when the Revolver is utilized > X%.
Baskets & Headroom: Track "Available Baskets" (e.g., "General Debt Basket", "Permitted Acquisitions") and calculate "Headroom" (how much EBITDA can drop before a breach).
5. Deal Sizing & LBO Solver
Current State: goal_seek.rs solves for single variables. Recommendation: A specialized DealStructurer or LBOSolver.

Features:
Multi-Constraint Solving: "Maximize Total Debt" subject to: Leverage < 4.5x AND FCCR > 1.2x.
Returns Analysis: Calculate Sponsor IRR and MOIC (Multiple on Invested Capital) based on entry/exit multiples and leverage.
Sources & Uses: Automatically generate a balanced "Sources and Uses" table for the transaction.
6. Unit Economics & KPI Drivers
Current State: Forecasting seems generic (ForecastSpec::growth). Recommendation: Add a UnitEconomics driver module.

Features:
Price x Volume: Model revenue as Price * Volume rather than just a growth rate.
SaaS Metrics: For tech deals, explicitly model ARR Waterfall (New Bookings, Churn, Upsell, Downsell).
Cohort Analysis: If data permits, forecast based on customer cohorts.
7. Peer Benchmarking
Current State: Analysis is isolated to the single company. Recommendation: A Benchmarking module.

Features:
Industry Comparison: Compare the target's margins, leverage, and growth against a dataset of industry peers or indices.
Quartile Analysis: "Is this company in the top quartile for Gross Margin but bottom quartile for Cash Conversion?"