# Revolving Credit IRR Distribution Analysis

## Overview

The `revolving_credit_irr_analysis.py` script analyzes the Internal Rate of Return (IRR) distributions for revolving credit facilities using Monte Carlo simulation with different volatility scenarios for utilization rates and credit spreads.

## Features

### 1. Single Scenario Analysis (10% Utilization Vol, 30% Credit Spread Vol)

- Calculates IRR distribution from 1000 Monte Carlo paths
- Overlays deterministic IRR as a vertical line for comparison
- Visualizes utilization rate and credit spread paths over time
- Shows mean, 5th, and 95th percentile bands for factor paths
- Displays path volatility evolution over the facility lifetime

### 2. Extreme Paths Analysis (NEW)

- Identifies top 5 and bottom 5 performing paths by IRR
- Creates detailed cashflow bar charts for each extreme path
- Color-codes cashflows by type (Fees, Interest, Principal, Other)
- Shows timing and magnitude of cashflows over facility lifetime
- Compares cashflow patterns between best and worst performers
- Provides insights into what drives performance differences

### 3. Volatility Grid Comparison

- Tests 9 scenarios: 3 utilization volatilities (10%, 20%, 30%) × 3 credit spread volatilities (20%, 30%, 40%)
- Overlays IRR distributions for all scenarios on a single chart
- Creates box plots for statistical comparison
- Scatter plot showing mean IRR vs volatility parameters
- Summary statistics table with mean, std dev, and percentiles

## Key Outputs

### CSV Files Generated

The script now exports detailed cashflow schedules for the top and bottom 5 IRR performers:

1. **Individual Cashflow Files** (10 files total):
   - `cashflows_bottom_[1-5]_irr_[value].csv`: Detailed cashflows for bottom 5 performers
   - `cashflows_top_[1-5]_irr_[value].csv`: Detailed cashflows for top 5 performers
   - Each file contains:
     - Date and Days_From_Start
     - Cashflow_Type (from Rust CFKind enum)
     - Amount and Currency
     - Day_Count_Fraction, Discount_Factor, Present_Value (if available)
     - Utilization_Rate and Credit_Spread at each cashflow date
     - IRR and Path_Rank

2. **Summary File** (`cashflows_summary.csv`):
   - Aggregated metrics for all extreme paths
   - Contains:
     - Path_Type (Top/Bottom) and Rank
     - IRR value
     - Total_Fees, Total_Interest, Total_Notional, Total_Cashflow
     - Number of cashflows
     - Average/Min/Max Utilization rates
     - Average/Min/Max Credit Spreads
   - Useful for comparing key metrics between best and worst performers

### Charts Generated

1. **irr_single_scenario.png**: Comprehensive single scenario analysis with 4 subplots:
   - IRR distribution histogram with KDE overlay
   - Utilization rate paths (sample paths + statistics)
   - Credit spread paths (sample paths + statistics)
   - Path volatility over time

2. **irr_extreme_paths.png**: Top 5 vs Bottom 5 performers analysis:
   - 20 panels (10 paths x 2 panels each):
     - **Top panel**: Stacked bar chart of cashflows over time
     - **Bottom panel**: Cumulative cashflows by category
   - Color-coded by Rust cashflow types (from bindings):
     - **Notional**: Blue (initial draw, interim draws/repayments, final principal)
     - **Fees**: Orange (upfront, facility, commitment, and usage fees)
     - **Fixed Interest**: Green (fixed rate interest payments)
     - **Floating Interest**: Lime (floating rate interest payments)
   - Cumulative panel shows:
     - Individual cumulative lines for each cashflow category
     - Total Net cumulative line (black dashed) showing overall position
     - Clear visualization of when the investment breaks even
   - Categorization comes directly from Rust CFKind enum
   - Shows timing and magnitude of each cashflow
   - Summary statistics comparing average IRRs and spread

3. **irr_volatility_grid.png**: Grid comparison with 4 visualizations:
   - Overlay of all IRR distributions
   - Box plot comparison across scenarios
   - Mean IRR vs volatility scatter plot
   - Summary statistics table

## Usage

### Main Analysis Script

```bash
# Run with default parameters
uv run python finstack-py/examples/scripts/valuations/instruments/revolving_credit/revolving_credit_irr_analysis.py

# The script will:
# 1. Create a test market with discount, forward, and hazard curves
# 2. Set up a $100MM revolving credit facility with 25% initial utilization
# 3. Run Monte Carlo simulations with specified volatilities
# 4. Calculate IRR for each path and create distributions
# 5. Generate visualization charts
# 6. Export detailed cashflow schedules to CSV files
```

### CSV Analysis Script

```bash
# Analyze the exported CSV files
uv run python finstack-py/examples/scripts/valuations/instruments/revolving_credit/analyze_cashflow_csvs.py finstack-py/examples/scripts/valuations/instruments/revolving_credit/

# This script will:
# 1. Load and display summary statistics
# 2. Compare top vs bottom performer metrics
# 3. Generate analysis charts showing:
#    - IRR distributions
#    - Total fees and interest comparisons
#    - Utilization vs IRR correlations
#    - Credit spread impacts
# 4. Export a comprehensive analysis chart (cashflow_analysis.png)
```

## Configuration

Key parameters can be adjusted in the script:

- **Facility parameters**:
  - Commitment: $100MM
  - Initial utilization: 50%
  - Maturity: 2 years
  - Base rate: SOFR + 250bps
  - Tiered commitment fees: 50/35/25 bps
  - Usage fee: 15 bps above 75% utilization
  - Facility fee: 10 bps
  - Upfront fee: $500K

- **Monte Carlo settings**:
  - Single scenario: 1000 paths
  - Grid scenarios: 500 paths each
  - Antithetic variates enabled
  - Mean reversion speed: 2.0 for utilization
  - CIR parameters for credit spreads

## Technical Notes

### IRR Calculation

- Uses `xirr` from finstack for precise date-based IRR calculation
- Cashflows are from the lender's perspective (initial outflow, periodic inflows)
- Filters out near-zero cashflows to avoid numerical issues
- As_of date is set before commitment date to include initial draw in Rust cashflows

### Cashflow Categorization

- All cashflow types come directly from Rust via CFKind enum
- No manual categorization based on amounts in Python
- Ensures consistency with Rust valuation engine
- Categories: Notional, Fees, Fixed Interest, Floating Interest

### Credit Spread Modeling

- Uses CIR (Cox-Ingersoll-Ross) model for credit spreads
- Market-realistic parameters may violate Feller condition - this is expected
- QE discretization handles boundary conditions gracefully
- **Zero Credit Spreads**: The CIR model correctly allows spreads to hit 0:
  - Occurs when Feller condition (2κθ ≥ σ²) is violated
  - Realistic in benign market conditions with tight spreads
  - Not a bug - reflects actual model dynamics
  - Zero spread = minimal/no credit risk at that time point

### Dependencies

- finstack-py (with revolving credit support)
- matplotlib (for visualization)
- scipy (for kernel density estimation)
- pandas (for data manipulation)
- numpy (for numerical operations)

## Interpreting Results

### IRR Distribution Characteristics

- **Mean IRR**: Expected return across all simulated paths
- **Standard Deviation**: Measure of IRR uncertainty/risk
- **5th/95th Percentiles**: 90% confidence interval for returns
- **Skewness**: Asymmetry in the distribution (often negative due to default risk)

### Extreme Paths Insights

- **Top Performers**: Paths with highest IRRs often show:
  - Consistent high utilization (more interest income)
  - Lower realized credit spreads (lower default risk)
  - Favorable timing of cashflows
- **Bottom Performers**: Paths with lowest IRRs typically have:
  - Lower average utilization (less interest income, more commitment fees)
  - Higher realized credit spreads or defaults
  - Unfavorable cashflow timing

### Volatility Impact

- **Utilization Volatility**: Primarily affects IRR dispersion (uncertainty)
- **Credit Spread Volatility**: Affects both mean and dispersion through default risk
- **Correlation Effects**: Future versions may include util-credit correlation

## Cumulative Cashflow Analysis

The cumulative panels in the extreme paths chart provide valuable insights:

- **Break-even timing**: Shows when cumulative cashflows turn positive
- **Investment recovery**: Visualizes the path to full principal recovery
- **Category contribution**: Shows how each cashflow type contributes to returns
- **Risk visualization**: Compares cashflow accumulation patterns between best and worst performers

## Example Output

```
Deterministic IRR: 8.25%
Monte Carlo Results (10% util vol, 30% CS vol):
  Mean IRR: 8.33%
  Std Dev: 0.18%
  5th Percentile: 8.01%
  95th Percentile: 8.65%

Extreme Paths Analysis:
  Bottom 5 Average IRR: 7.95%
  Top 5 Average IRR: 8.72%
  Spread: 0.77%
```

## CSV Export Features

The script automatically exports comprehensive cashflow data for further analysis:

### Individual Cashflow Files

**Two types of CSV files are generated for debugging:**

1. **PV Cashflow Files** (`cashflows_pv_[top/bottom]_*.csv`):
   - **For PV debugging**: Complete cashflow schedule with pricing calculations
   - Contains all CashFlow fields from Rust:
     - Date, Amount, Cashflow_Type (CFKind enum)
     - Accrual_Factor, Reset_Date, Outstanding
   - Plus calculated pricing fields:
     - Year_Fraction (ACT/365F from as_of to payment date)
     - Discount_Factor (from USD discount curve)
     - Present_Value (Amount × Discount_Factor)
   - Total PV is printed for each file for verification
   - **Use these files to debug PV calculations**

2. **MC Path Cashflow Files** (`cashflows_[top/bottom]_*.csv`):
   - Cashflows with Monte Carlo path data
   - MC_Utilization and MC_Credit_Spread at each payment date
   - **Note**: Credit spreads can correctly go to 0 in CIR model
   - Use these to analyze how stochastic paths affect cashflows

### Summary Statistics

- Aggregated metrics comparing top and bottom performers
- Shows correlation between path characteristics and IRR
- Includes average/min/max utilization and credit spreads
- Total fees, interest, and notional for each path

### Analysis Capabilities

The `analyze_cashflow_csvs.py` script provides:

- Statistical comparison of performer groups
- Visual correlation analysis
- Break-even timing calculation
- Cashflow categorization and aggregation
- Export of comprehensive analysis charts

## Future Enhancements

- Add interest rate stochasticity (currently deterministic)
- Include correlation between utilization and credit spreads
- Support for different draw/repay patterns
- Sensitivity analysis for fee structures
- Multi-currency facilities
