### Feature 1: Cross-Currency Swaps (XCCY Swaps)

**Persona Pain**: Quants and FX traders cannot value multi-currency fixed-float or float-float swaps with notional exchange and basis spreads, forcing manual spreadsheet replication or external system dependencies.

**User Story**: *As a quant on an FX derivatives desk, I need to price cross-currency basis swaps (e.g., USD SOFR vs EUR €STR + basis) with initial and final notional exchange, bucketed DV01 per currency leg, and accurate cross-gamma, so that I can manage multi-currency portfolios and report FX basis risk.*

**Scope (what's new)**:

**Data**:
- Inputs: `(notional_domestic, notional_foreign, fx_rate_at_trade, domestic_leg, foreign_leg, basis_spread_bp)`
- Domestic/Foreign legs: each a `FloatLegSpec` or `FixedLegSpec`
- Notional exchange: `(initial: bool, final: bool, intermediate_resets: Vec<Date>)`
- Outputs: PV per leg (domestic/foreign), basis value, FX delta, DV01 per currency

```rust
pub struct CrossCurrencySwap {
    pub id: InstrumentId,
    pub domestic_notional: Money,      // e.g., USD 10M
    pub foreign_notional: Money,       // e.g., EUR 8M
    pub fx_rate_at_trade: f64,         // spot at inception (1.25)
    pub domestic_leg: XccyLegSpec,     // fixed or floating
    pub foreign_leg: XccyLegSpec,
    pub basis_spread_bp: f64,          // cross-currency basis (e.g., -25bp on EUR leg)
    pub notional_exchange: NotionalExchangeSpec,
    pub discount_curve_domestic: CurveId,
    pub discount_curve_foreign: CurveId,
    pub forward_curve_domestic: Option<CurveId>,
    pub forward_curve_foreign: Option<CurveId>,
    pub fx_pair: String,               // "USDEUR"
    pub attributes: Attributes,
}

pub enum XccyLegSpec {
    Fixed { rate: f64, schedule: ScheduleSpec },
    Floating { index: String, spread_bp: f64, schedule: ScheduleSpec },
}

pub struct NotionalExchangeSpec {
    pub initial: bool,     // pay foreign notional, receive domestic at inception
    pub final: bool,       // reverse exchange at maturity
    pub intermediate_resets: Vec<Date>,  // for MTM XCCY swaps
}
```

**APIs**:

*Rust*:
```rust
let xccy = CrossCurrencySwap::new(
    "XCCY-001",
    Money::new(10_000_000.0, Currency::USD),
    Money::new(8_000_000.0, Currency::EUR),
    1.25,  // USD/EUR at trade
)
.domestic_leg_floating("USD-SOFR", 0.0, quarterly_schedule)
.foreign_leg_floating("EUR-ESTR", -25.0, quarterly_schedule)  // -25bp basis
.with_notional_exchange(true, true, vec![])
.build()?;

let result = xccy.price_with_metrics(&market, as_of, &[
    MetricId::Dv01Domestic,
    MetricId::Dv01Foreign,
    MetricId::FxDelta,
    MetricId::BasisDv01,  // new: sensitivity to cross-currency basis
])?;
```

*Python*:
```python
xccy = CrossCurrencySwap(
    id="XCCY-001",
    domestic_notional=Money(10_000_000, "USD"),
    foreign_notional=Money(8_000_000, "EUR"),
    fx_rate_at_trade=1.25,
    domestic_leg=FloatingLeg("USD-SOFR", spread_bp=0, schedule=...),
    foreign_leg=FloatingLeg("EUR-ESTR", spread_bp=-25, schedule=...),
    notional_exchange=NotionalExchange(initial=True, final=True),
)
result = xccy.price_with_metrics(market, as_of, ["dv01_domestic", "dv01_foreign", "fx_delta"])
df = result.to_polars()  # columns: [metric, value, ccy]
```

**Explainability**: `explain()` output shows:
```
Cross-Currency Swap XCCY-001:
  PV_domestic_leg: $45,230 USD
  PV_foreign_leg: -€36,100 EUR (= -$45,050 USD @ 1.248)
  FX_basis_value: $180 USD (difference driven by -25bp basis)
  Net_PV: $360 USD
  Sensitivities:
    DV01_USD: -$8,450 per bp
    DV01_EUR: $6,920 per bp (in USD equiv)
    FX_Delta: $8,000,000 (notional exposure)
    Basis_DV01: -$120 per bp basis widening
```

**Validation**: Test cases:
- Par cross-currency basis swap (NPV = 0 at market basis)
- Mark-to-market XCCY with intermediate resets
- Comparison with Bloomberg SWPM (`XCCY <GO>`) for 5Y EUR/USD basis swap
- Property test: `NPV(domestic→foreign) = -NPV(foreign→domestic) * FX`

**Impact & Effort**: P0; Medium (4-6 weeks)
- **Dependencies**: FxMatrix (exists), dual-curve discounting (exists), basis spread handling (add)
- **Risks**: Correct notional exchange timing, handling MTM resets with collateral
- **Mitigations**: Follow Bloomberg/ISDA conventions; unit test against industry examples

**Demo Outline**:
```python
# Notebook: xccy_basis_swap_analysis.ipynb
# 1. Build 5Y USD SOFR vs EUR ESTR + basis swap
# 2. Calibrate USD/EUR OIS curves from market quotes
# 3. Price swap, compute bucketed DV01 per currency
# 4. Scenario: basis widens -25bp → -30bp, show P&L impact
# 5. Export cashflow waterfall to DataFrame for audit
```

**Why Now**: Cross-currency basis trading exploded post-2008 (multi-curve framework). Essential for:
- Bank treasury ALM (fund foreign currency assets)
- Corporate hedging (foreign debt issuance)
- Regulatory reporting (FRTB, SA-CCR capital)

**De-Dup Evidence**: 
- Searched: `"CrossCurrencySwap"`, `"XCCY"`, `"cross currency"` → **Not found**
- FxSwap exists (near/far leg FX forwards) but does NOT handle interest rate legs with basis spreads

---

### Feature 2: XVA Framework (CVA, DVA, FVA, MVA, KVA)

**Persona Pain**: Credit analysts and FX traders cannot compute counterparty credit risk adjustments (CVA/DVA) or funding costs (FVA) for OTC derivatives portfolios, requiring expensive third-party XVA engines.

**User Story**: *As a credit quant, I need to calculate CVA (credit valuation adjustment) for a portfolio of uncollateralized swaps by simulating exposure paths, applying counterparty default probabilities, and integrating over expected positive exposure, so that I can price counterparty risk into derivative valuations and allocate economic capital.*

**Scope (what's new)**:

**Data**:
- Inputs: `(netting_set, counterparty_hazard_curve, own_hazard_curve, collateral_agreement)`
- Exposure simulation: Monte Carlo paths of mark-to-market (requires `mc` feature)
- Outputs: CVA, DVA, FVA, bilateral CVA (BCVA), allocated by instrument

```rust
pub struct XvaCalculator {
    pub netting_set_id: String,
    pub instruments: Vec<Arc<dyn Instrument>>,
    pub counterparty_curve: CurveId,  // hazard curve
    pub own_curve: Option<CurveId>,   // for DVA
    pub csa: Option<CsaAgreement>,
    pub config: XvaConfig,
}

pub struct CsaAgreement {
    pub threshold: Money,
    pub minimum_transfer_amount: Money,
    pub independent_amount: Money,
    pub collateral_haircut: f64,
    pub remargin_frequency_days: u32,
}

pub struct XvaConfig {
    pub simulation_dates: Vec<Date>,
    pub num_paths: usize,
    pub risk_free_curve: CurveId,
    pub funding_spread_bp: f64,  // for FVA
    pub include_cva: bool,
    pub include_dva: bool,
    pub include_fva: bool,
}

pub struct XvaResult {
    pub cva: Money,               // credit valuation adjustment
    pub dva: Money,               // debit valuation adjustment (own default)
    pub fva: Money,               // funding valuation adjustment
    pub mva: Money,               // margin valuation adjustment
    pub kva: Money,               // capital valuation adjustment
    pub bcva: Money,              // bilateral CVA = CVA - DVA
    pub expected_exposure: Vec<(Date, Money)>,  // EE curve
    pub potential_future_exposure_95: Vec<(Date, Money)>,  // PFE 95th percentile
    pub explanation: XvaExplanation,
}
```

**APIs**:

*Rust*:
```rust
let xva_calc = XvaCalculator::new("NettingSet-001")
    .add_instruments(vec![swap1, swap2, swap3])
    .counterparty_curve("CORP-A-USD")
    .own_curve("BANK-USD")
    .csa(CsaAgreement {
        threshold: Money::new(5_000_000.0, Currency::USD),
        minimum_transfer_amount: Money::new(500_000.0, Currency::USD),
        independent_amount: Money::zero(Currency::USD),
        collateral_haircut: 0.02,
        remargin_frequency_days: 1,
    })
    .config(XvaConfig {
        simulation_dates: monthly_dates,
        num_paths: 10_000,
        risk_free_curve: "USD-OIS".into(),
        funding_spread_bp: 50.0,
        include_cva: true,
        include_dva: true,
        include_fva: true,
    })
    .build()?;

let xva_result = xva_calc.calculate(&market, as_of)?;
println!("CVA: {}", xva_result.cva);          // e.g., -$125,000
println!("DVA: {}", xva_result.dva);          // e.g., +$30,000
println!("BCVA: {}", xva_result.bcva);        // e.g., -$95,000
println!("FVA: {}", xva_result.fva);          // e.g., -$18,000

// Export EE curve to DataFrame
let ee_df = xva_result.expected_exposure_to_polars();
```

*Python*:
```python
xva = XvaCalculator(
    netting_set_id="NettingSet-001",
    instruments=[swap1, swap2, swap3],
    counterparty_curve="CORP-A-USD",
    own_curve="BANK-USD",
    csa=CsaAgreement(threshold=5_000_000, mta=500_000, ia=0, haircut=0.02),
)
result = xva.calculate(market, as_of, num_paths=10_000, funding_spread_bp=50)
print(f"CVA: {result.cva}, DVA: {result.dva}, BCVA: {result.bcva}")
ee_df = result.expected_exposure_df()  # Polars/Pandas DataFrame
```

**Explainability**: `explain()` output:
```
XVA for Netting Set NettingSet-001:
  Instruments: 3 swaps (IRS-001, IRS-002, CDS-001)
  Counterparty: CORP-A (hazard rate 150bp)
  CSA: Threshold $5M, daily VM
  
  CVA (Credit Valuation Adjustment): -$125,000
    - Expected Positive Exposure (EPE): $2.1M average
    - Counterparty PD (5Y): 7.2%
    - Loss Given Default: 60%
    
  DVA (Debit Valuation Adjustment): +$30,000
    - Expected Negative Exposure (ENE): $0.8M average
    - Own PD (5Y): 1.5%
    
  FVA (Funding Valuation Adjustment): -$18,000
    - Funding spread: 50bp
    - Average unfunded exposure: $1.2M
    
  Bilateral CVA (BCVA): -$95,000
```

**Validation**:
- Test against **Pyth** or **QuantLib** XVA examples
- Reproduce CVA for single uncollateralized IRS from Gregory (2012) *Counterparty Credit Risk and CVA*
- Property test: CVA ≥ 0 (cannot benefit from counterparty default), DVA ≥ 0
- Regression test: CVA with CSA threshold → lower than uncollateralized

**Impact & Effort**: P0; Large (8-12 weeks)
- **Dependencies**: Monte Carlo framework (exists with `mc` feature), hazard curves (exist), netting sets (new)
- **Risks**: Computational cost (10K paths × 100 dates × 10 instruments), wrong-way risk not modeled initially
- **Mitigations**: Start with independent exposures/defaults; add WWR in phase 2; use Rayon for parallelism

**Demo Outline**:
```python
# Notebook: cva_calculation_example.ipynb
# 1. Build portfolio of 3 IRS (receiver swaps, NPV = +$2M)
# 2. Calibrate counterparty hazard curve from CDS spreads (BBB-rated)
# 3. Simulate exposure paths (1000 scenarios, monthly grid)
# 4. Calculate CVA with 60% LGD
# 5. Compare: (a) uncollateralized, (b) CSA with $5M threshold, (c) daily VM
# 6. Show EE/PFE curves, CVA breakdown by instrument
```

**Why Now**: 
- **Regulatory**: Basel III CVA capital charge, FRTB SA-CVA, IFRS 13 fair value adjustments
- **Risk management**: Counterparty limits, credit exposure reporting
- **Pricing**: Accurate OTC derivative mid-market pricing includes XVA

**De-Dup Evidence**:
- Searched: `"CVA"`, `"XVA"`, `"counterparty"`, `"credit valuation adjustment"` → **Not found**
- Attribution module exists (P&L decomposition) but NOT counterparty credit risk

---

### Feature 3: ISDA SIMM (Standard Initial Margin Model)

**Persona Pain**: Traders and middle-office risk teams cannot calculate initial margin (IM) for uncleared OTC derivatives portfolios under regulatory mandates (UMR), forcing reliance on third-party vendors (AcadiaSoft, Numerix).

**User Story**: *As a derivatives middle-office analyst, I need to compute ISDA SIMM initial margin for a portfolio of uncleared swaps, options, and credit derivatives across IR, FX, EQ, and Credit delta/vega risk classes, with proper netting and correlation, so that I can estimate margin calls, optimize portfolio composition, and comply with UMR (Uncleared Margin Rules).*

**Scope (what's new)**:

**Data**:
- Inputs: Portfolio of instruments → sensitivities (Delta, Vega, Curvature per risk factor)
- Risk classes: IR (interest rate), FX, EQ (equity), CO (commodity), CR (credit)
- SIMM risk weights, correlations (published by ISDA annually)
- Outputs: IM per risk class, total IM with diversification benefit

```rust
pub struct SimmCalculator {
    pub portfolio_id: String,
    pub version: SimmVersion,  // e.g., SIMM 2.6 (Dec 2023)
}

pub enum SimmVersion {
    Simm26,  // December 2023
    Simm25,
}

pub struct SimmInputs {
    pub sensitivities: Vec<RiskFactorSensitivity>,
    pub product_class: ProductClass,  // RatesFX, Credit, Equity, Commodity
}

pub struct RiskFactorSensitivity {
    pub risk_type: RiskType,       // Delta, Vega, Curvature
    pub risk_class: RiskClass,     // IR, FX, EQ, CO, CR
    pub qualifier: String,         // e.g., "USD", "EUR", "AAPL", "CDX.IG"
    pub bucket: Option<String>,    // SIMM bucket (e.g., "1" for G10 FX)
    pub label1: Option<String>,    // e.g., tenor "2Y"
    pub label2: Option<String>,    // e.g., strike for vega
    pub amount_base_ccy: f64,      // sensitivity in base currency
}

pub struct SimmResult {
    pub initial_margin: Money,
    pub im_by_risk_class: HashMap<RiskClass, Money>,
    pub diversification_benefit: Money,
    pub gross_im: Money,  // sum before diversification
    pub explanation: SimmExplanation,
}
```

**APIs**:

*Rust*:
```rust
let simm = SimmCalculator::new("Portfolio-001", SimmVersion::Simm26);

// Collect sensitivities from instruments
let mut sensitivities = vec![];
for inst in portfolio.instruments() {
    let result = inst.price_with_metrics(&market, as_of, &[
        MetricId::BucketedDv01,
        MetricId::BucketedVega,
        MetricId::FxDelta,
    ])?;
    sensitivities.extend(extract_simm_sensitivities(&result, inst)?);
}

let inputs = SimmInputs {
    sensitivities,
    product_class: ProductClass::RatesFX,
};

let simm_result = simm.calculate(inputs, Currency::USD)?;
println!("Total IM: {}", simm_result.initial_margin);  // e.g., $8.2M
println!("IM (IR): {}", simm_result.im_by_risk_class[&RiskClass::IR]);  // $6.5M
println!("IM (FX): {}", simm_result.im_by_risk_class[&RiskClass::FX]);  // $1.9M
println!("Diversification benefit: {}", simm_result.diversification_benefit);  // -$0.2M
```

*Python*:
```python
simm = SimmCalculator(portfolio_id="Portfolio-001", version="SIMM_2.6")
sensitivities = collect_sensitivities_from_portfolio(portfolio, market, as_of)
result = simm.calculate(sensitivities, base_currency="USD")
print(f"Total IM: {result.initial_margin}")
df = result.to_dataframe()  # breakdown by risk class, bucket
```

**Explainability**: `explain()` output:
```
ISDA SIMM 2.6 for Portfolio-001:
  Product Class: RatesFX
  
  Initial Margin by Risk Class:
    IR (Interest Rate): $6.5M
      - Delta: $6.2M (USD: $4.1M, EUR: $1.8M, GBP: $0.3M)
      - Vega: $0.3M
    FX (Foreign Exchange): $1.9M
      - Delta: $1.9M (EURUSD: $1.2M, GBPUSD: $0.7M)
    
  Gross IM (before diversification): $8.4M
  Diversification benefit: -$0.2M (2.4%)
  Total IM: $8.2M
```

**Validation**:
- Test against **ISDA SIMM white paper examples** (publicly available test cases)
- Reproduce IM for standard portfolio from ISDA SIMM FAQ
- Property test: IM increases monotonically with position size
- Compare with **AcadiaSoft IM calculator** (online tool) for small portfolios

**Impact & Effort**: P0; Large (10-14 weeks)
- **Dependencies**: Bucketed sensitivities (exist: BucketedDv01, BucketedVega), SIMM parameter tables (add)
- **Risks**: Complex correlation matrices, annual SIMM version updates, regulatory scrutiny
- **Mitigations**: Parameterize risk weights/correlations from ISDA docs; version-controlled SIMM configs

**Demo Outline**:
```python
# Notebook: isda_simm_margin_calculation.ipynb
# 1. Build portfolio: 5 IRS (USD, EUR), 2 FX Options, 1 CDS
# 2. Compute bucketed sensitivities (DV01 by tenor, vega by expiry/strike)
# 3. Calculate SIMM IM per ISDA methodology
# 4. Show impact of adding offsetting position (IM reduction from netting)
# 5. Export IM breakdown to CSV for regulatory reporting
```

**Why Now**:
- **Regulatory mandate**: UMR Phases 5-6 (in-scope firms must post IM for uncleared derivatives)
- **Optimization**: Margin-efficient portfolio construction
- **Capital**: IM drives funding costs and balance sheet optimization

**De-Dup Evidence**:
- Searched: `"SIMM"`, `"initial margin"`, `"ISDA"`, `"UMR"` → **Not found**
- Margin/haircut exist for Repo but NOT regulatory IM for OTC derivatives

---

### Feature 4: Commodity Instruments (Futures, Forwards, Options)

**Persona Pain**: Energy traders and commodity risk managers cannot price oil/gas/power/metals derivatives, forcing separate systems or manual valuation.

**User Story**: *As an energy trader, I need to price crude oil futures, natural gas swaps, and European call options on WTI with correct forward curve interpolation, storage costs, and convenience yield, so that I can hedge physical commodity exposures and mark-to-market derivative portfolios.*

**Scope (what's new)**:

**Data**:
- Commodity forward curves (analogous to ForwardCurve but for commodities)
- Storage costs, convenience yield
- Instruments: `CommodityForward`, `CommodityFuture`, `CommodityOption`, `CommoditySwap`

```rust
pub struct CommodityForward {
    pub id: InstrumentId,
    pub commodity: Commodity,  // enum: WTI, Brent, NatGas, Gold, ...
    pub quantity: f64,         // barrels, MMBtu, troy oz
    pub delivery_date: Date,
    pub forward_price: f64,    // locked-in price
    pub forward_curve_id: CurveId,
    pub discount_curve_id: CurveId,
    pub attributes: Attributes,
}

pub enum Commodity {
    WtiCrude,
    BrentCrude,
    NaturalGas,   // Henry Hub
    Gold,
    Silver,
    Copper,
    Power(PowerHub),  // e.g., PJM West, ERCOT North
}

pub struct CommodityOption {
    pub underlying: Commodity,
    pub strike: f64,
    pub expiry: Date,
    pub option_type: OptionType,  // Call, Put
    pub quantity: f64,
    pub forward_curve_id: CurveId,
    pub vol_surface_id: String,  // 2D surface: expiry × moneyness
    pub discount_curve_id: CurveId,
}
```

**APIs**:

*Rust*:
```rust
let oil_forward = CommodityForward::new(
    "OIL-FWD-001",
    Commodity::WtiCrude,
    1000.0,  // 1000 barrels
    maturity_date,
    85.50,   // $85.50/bbl locked in
)
.forward_curve("WTI-USD")
.discount_curve("USD-OIS")
.build()?;

let pv = oil_forward.value(&market, as_of)?;

let oil_call = CommodityOption::european_call(
    "OIL-CALL-001",
    Commodity::WtiCrude,
    90.0,    // $90 strike
    expiry,
    1000.0,
)
.vol_surface("WTI-VOL")
.forward_curve("WTI-USD")
.build()?;

let greeks = oil_call.price_with_metrics(&market, as_of, &[
    MetricId::Delta,
    MetricId::Gamma,
    MetricId::Vega,
])?;
```

*Python*:
```python
oil_fwd = CommodityForward(
    id="OIL-FWD-001",
    commodity="WTI",
    quantity=1000,
    delivery_date=maturity,
    forward_price=85.50,
    forward_curve="WTI-USD",
)
pv = oil_fwd.value(market, as_of)

oil_call = CommodityOption.european_call(
    commodity="WTI",
    strike=90.0,
    expiry=expiry,
    quantity=1000,
    vol_surface="WTI-VOL",
)
result = oil_call.price_with_metrics(market, as_of, ["delta", "vega"])
```

**Explainability**: Standard option Greeks plus commodity-specific:
```
Commodity Call Option (WTI Crude):
  Spot: $87.20/bbl
  Forward (3M): $88.10/bbl
  Strike: $90.00/bbl
  Vol: 28% (ATM)
  PV: $2,150 (for 1000 bbls)
  
  Greeks:
    Delta: 0.48
    Gamma: 0.032
    Vega: $45 per 1% vol
    Theta: -$8 per day
  
  Convenience yield implied: 1.8% p.a.
```

**Validation**:
- Compare WTI option pricing with **CME Nymex WTI options** market quotes
- Test forward curve interpolation with Bloomberg `OMON <GO>` (Oil Monitor)
- Property test: put-call parity for European commodity options

**Impact & Effort**: P1; Medium (6-8 weeks)
- **Dependencies**: Forward curves (exist for rates, adapt for commodities), vol surfaces (exist), Black-76 pricer (exists)
- **Risks**: Commodity-specific features (seasonality for nat gas/power, delivery specs)
- **Mitigations**: Start with crude oil (simple), expand to nat gas/power (complex) in phase 2

**Demo Outline**:
```python
# Notebook: commodity_derivatives_pricing.ipynb
# 1. Build WTI forward curve from futures prices (NYMEX CL1-CL12)
# 2. Calibrate ATM vol surface from market option quotes
# 3. Price European call on WTI (strike $90, 3M expiry)
# 4. Compute delta hedge: short 480 barrels of CL1 futures
# 5. Scenario: oil vol spikes 28% → 35%, show P&L impact
```

**Why Now**:
- Energy transition → increased commodity derivatives activity (carbon credits, renewables)
- Inflation hedge → institutional interest in commodity exposure
- Common ask from energy/mining clients

**De-Dup Evidence**:
- Searched: `"Commodity"`, `"oil"`, `"natural gas"`, `"WTI"`, `"Brent"` → **Not found**
- No commodity instruments exist; only financial (rates, FX, equity, credit)

---

### Feature 5: Credit Migration Models (Rating Transition Matrices)

**Persona Pain**: Credit portfolio managers cannot model credit migration risk (rating upgrades/downgrades) or expected rating transitions, limiting credit VaR and expected shortfall calculations.

**User Story**: *As a credit portfolio manager, I need to model the probability of a BBB-rated bond migrating to BB (downgrade to high-yield) over 1 year using a credit transition matrix (e.g., Moody's/S&P historical matrices), apply migration-adjusted spreads, and compute credit migration VaR, so that I can estimate unexpected credit losses beyond default risk.*

**Scope (what's new)**:

**Data**:
- Transition matrix: `[Rating × Rating]` with 1-year migration probabilities
- Rating-dependent spread curves
- Expected credit loss from migration (spread widening)

```rust
pub struct TransitionMatrix {
    pub id: String,
    pub ratings: Vec<CreditRating>,  // AAA, AA, A, BBB, BB, B, CCC, D
    pub matrix: Array2<f64>,         // [from_rating, to_rating] = probability
    pub horizon_years: f64,          // e.g., 1.0 for annual matrix
    pub source: MatrixSource,        // Moody's, S&P, Fitch, Custom
}

pub enum MatrixSource {
    MoodysHistorical,
    SPHistorical,
    FitchHistorical,
    Custom(String),
}

pub struct CreditMigrationCalculator {
    pub transition_matrix: TransitionMatrix,
    pub spread_curves: HashMap<CreditRating, CurveId>,  // hazard curve per rating
    pub config: MigrationConfig,
}

pub struct MigrationVaRResult {
    pub var_99: Money,  // 99th percentile loss from migration
    pub expected_loss: Money,
    pub unexpected_loss: Money,  // sqrt(variance)
    pub migration_scenarios: Vec<MigrationScenario>,
}

pub struct MigrationScenario {
    pub from_rating: CreditRating,
    pub to_rating: CreditRating,
    pub probability: f64,
    pub spread_change_bp: f64,
    pub pv_impact: Money,
}
```

**APIs**:

*Rust*:
```rust
// Load Moody's 1-year transition matrix
let transition_matrix = TransitionMatrix::load_moodys_annual()?;

let migration_calc = CreditMigrationCalculator::new(transition_matrix)
    .add_spread_curve(CreditRating::AAA, "AAA-USD")
    .add_spread_curve(CreditRating::AA, "AA-USD")
    .add_spread_curve(CreditRating::A, "A-USD")
    .add_spread_curve(CreditRating::BBB, "BBB-USD")
    .add_spread_curve(CreditRating::BB, "BB-USD")
    .build()?;

// Calculate migration VaR for a BBB-rated bond
let bond = Bond::fixed("CORP-BBB-001", notional, 0.05, issue, maturity, "BBB-USD");
let current_rating = CreditRating::BBB;

let migration_result = migration_calc.calculate_migration_var(
    &bond,
    current_rating,
    &market,
    as_of,
    horizon_years: 1.0,
)?;

println!("Migration VaR (99%): {}", migration_result.var_99);  // e.g., -$85,000
println!("Expected credit loss: {}", migration_result.expected_loss);  // e.g., -$12,000
```

*Python*:
```python
transition_matrix = TransitionMatrix.load_moodys_annual()
migration_calc = CreditMigrationCalculator(
    transition_matrix=transition_matrix,
    spread_curves={"AAA": "AAA-USD", "BBB": "BBB-USD", ...},
)

bond = Bond.fixed("CORP-BBB-001", notional=10_000_000, coupon=0.05, ...)
result = migration_calc.calculate_migration_var(
    bond=bond,
    current_rating="BBB",
    market=market,
    as_of=as_of,
    horizon_years=1.0,
)
print(f"Migration VaR (99%): {result.var_99}")
df = result.migration_scenarios_df()  # Polars DataFrame
```

**Explainability**: `explain()` output:
```
Credit Migration VaR for CORP-BBB-001:
  Current Rating: BBB
  Horizon: 1 year
  
  Migration Probabilities (from Moody's):
    Stay BBB: 86.5%
    Upgrade to A: 5.2%
    Downgrade to BB: 6.8%
    Downgrade to B: 0.9%
    Default: 0.3%
    
  Spread Changes:
    BBB → BB: +180bp (fallen angel), PV impact: -$850K (prob: 6.8%)
    BBB → B: +420bp, PV impact: -$1.9M (prob: 0.9%)
    BBB → D: total loss, PV impact: -$10M (prob: 0.3%)
    
  Migration VaR (99%): -$85,000
  Expected credit loss: -$12,000 (weighted average)
```

**Validation**:
- Reproduce **CreditMetrics™** example from JP Morgan (1997) technical document
- Test transition matrix properties: rows sum to 1.0, absorbing state at Default
- Compare migration VaR with **RiskMetrics CreditManager** (if available)

**Impact & Effort**: P1; Medium (6-8 weeks)
- **Dependencies**: Hazard curves (exist), rating enums (add), transition matrix data (add)
- **Risks**: Matrix stability over time, rating agency differences
- **Mitigations**: Version-control matrices; allow user-supplied custom matrices

**Demo Outline**:
```python
# Notebook: credit_migration_var.ipynb
# 1. Load S&P 1-year transition matrix (1981-2020 historical)
# 2. Build BBB-rated corporate bond portfolio (10 bonds)
# 3. Calibrate spread curves per rating (AAA to CCC)
# 4. Calculate migration VaR (99%) for 1-year horizon
# 5. Show dominant risk: BBB → BB fallen angel scenario
# 6. Compare with default-only VaR (understates risk)
```

**Why Now**:
- **Regulatory**: IFRS 9 expected credit loss requires multi-period migration scenarios
- **Risk management**: Rating migration is larger risk than default for IG portfolios
- **Portfolio optimization**: Maximize Sharpe ratio accounting for migration volatility

**De-Dup Evidence**:
- Searched: `"transition matrix"`, `"credit migration"`, `"rating transition"` → **Not found**
- Hazard curves exist (default probability) but NOT migration between non-default states

---

## 3) Quick Wins ("Fast-Follow", 10 items)

1. **Advanced Cap/Floor Pricing**: Add Normal vol and shifted lognormal pricing models (currently only Black76). Useful for negative/low rate environments (EUR, JPY). **Effort: 3-5 days**

2. **Credit-Linked Notes (CLN)**: Thin wrapper around `CreditDefaultSwap` + `Bond` with principal-at-risk feature. **Effort: 1 week**

3. **Real Yield Curves**: Explicit real rate term structure = `(discount_curve / inflation_curve) - 1`. Add `RealYieldCurve` type with breakeven inflation helpers. **Effort: 1 week**

4. **Multi-Callable Bonds**: Extend `Bond` to support call schedules (currently supports single call date). Add `CallSchedule` with make-whole provisions. **Effort: 1-2 weeks**

5. **Rating-Triggered Step-Ups**: Extend `TermLoan` margin step-ups to trigger on rating downgrades (currently date-based). **Effort: 3-5 days**

6. **Equity Forward/Future**: Simple instrument = `spot * exp((r - q) * T)` with DV01 and dividend yield sensitivities. **Effort: 3 days**

7. **Quanto CMS Options**: Combine existing `CmsOption` + `QuantoOption` logic. **Effort: 1 week**

8. **Dividend Discount Models (DDM)**: Add `EquityDdm` instrument with Gordon growth model and multi-stage DCF for fundamental valuation. **Effort: 1 week**

9. **Inflation Seasonality**: Add seasonal adjustment factors to `InflationCurve` (e.g., Dec CPI often higher). **Effort: 3 days**

10. **Export Sensitivities to Arrow/Parquet**: Batch export of bucketed DV01/vega to Parquet for data lake ingestion. **Effort**: 2 days**

---

## 4) De-Dup Check

### Evidence of Absence

| Feature                    | Search Terms                                      | Result      | Evidence |
|----------------------------|---------------------------------------------------|-------------|----------|
| Cross-Currency Swaps       | `"CrossCurrencySwap"`, `"XCCY"`, `"cross currency"` | Not found   | FxSwap exists (FX forwards) but no interest rate legs with basis |
| XVA Framework              | `"CVA"`, `"XVA"`, `"counterparty credit"`          | Not found   | Attribution exists (P&L) but NOT counterparty risk adjustments |
| ISDA SIMM                  | `"SIMM"`, `"initial margin"`, `"ISDA"`, `"UMR"`    | Not found   | Repo has haircuts but NOT regulatory IM for OTC derivatives |
| Commodity Instruments      | `"Commodity"`, `"oil"`, `"WTI"`, `"natural gas"`   | Not found   | No commodity forward curves or instruments |
| Credit Migration Models    | `"transition matrix"`, `"credit migration"`        | Not found   | Hazard curves exist but NOT rating transitions |
| Advanced Cap/Floor Pricing | `"Normal volatility"`, `"shifted lognormal"`       | Partial     | Black76 exists; Normal/Shifted missing |
| Credit-Linked Notes        | `"CLN"`, `"credit linked note"`                    | Not found   | CDS exists, Bond exists, but no CLN wrapper |
| Real Yield Curves          | `"real yield"`, `"real rate curve"`                | Partial     | Inflation curves exist; explicit real yield missing |
| Equity DDM                 | `"Gordon growth"`, `"dividend discount"`           | Not found   | Equity spot exists; no DCF valuation |
| Quanto CMS                 | `"Quanto CMS"`, `"quanto constant maturity"`       | Not found   | Quanto options + CMS options exist separately |

### Partial Implementations (Deltas Only)

- **Cap/Floor**: Black76 exists → Add Normal vol, Shifted lognormal models
- **Bond Callability**: Single call date exists → Add `CallSchedule` with multiple dates/prices
- **Real Yield**: Inflation curves exist → Add derived real rate curve with helpers
- **Term Loan Step-Ups**: Date-based step-ups exist → Add rating-triggered logic

---

## 5) Appendix: API Snippets & Schemas

### Cross-Currency Swap Cashflow Schema (JSON)

```json
{
  "id": "XCCY-001",
  "domestic_notional": { "amount": 10000000.0, "currency": "USD" },
  "foreign_notional": { "amount": 8000000.0, "currency": "EUR" },
  "fx_rate_at_trade": 1.25,
  "domestic_leg": {
    "type": "floating",
    "index": "USD-SOFR",
    "spread_bp": 0.0,
    "schedule": { "frequency": "3M", "start": "2025-01-15", "end": "2030-01-15" }
  },
  "foreign_leg": {
    "type": "floating",
    "index": "EUR-ESTR",
    "spread_bp": -25.0,
    "schedule": { "frequency": "3M", "start": "2025-01-15", "end": "2030-01-15" }
  },
  "notional_exchange": { "initial": true, "final": true, "intermediate_resets": [] }
}
```

### ISDA SIMM Sensitivity Schema

```json
{
  "risk_type": "Delta",
  "risk_class": "IR",
  "qualifier": "USD",
  "bucket": null,
  "label1": "2Y",
  "label2": null,
  "amount_base_ccy": -125000.0
}
```

### Commodity Option Pricing (Python Notebook Sketch)

```python
# Step 1: Build WTI forward curve
wti_curve = CommodityForwardCurve.from_futures_prices(
    commodity="WTI",
    futures_prices={
        "2025-03": 87.20,
        "2025-06": 88.10,
        "2025-09": 88.80,
        "2025-12": 89.30,
    },
    discount_curve="USD-OIS",
)

# Step 2: Calibrate vol surface
wti_vol_surface = VolSurface.from_market_quotes(
    surface_id="WTI-VOL",
    quotes=[
        {"expiry": "3M", "strike_delta": 0.5, "vol": 0.28},  # ATM
        {"expiry": "3M", "strike_delta": 0.25, "vol": 0.32},  # OTM call
        # ... more quotes
    ],
)

# Step 3: Price European call
oil_call = CommodityOption.european_call(
    commodity="WTI",
    strike=90.0,
    expiry=date(2025, 6, 15),
    quantity=1000,
    vol_surface="WTI-VOL",
    forward_curve="WTI-USD",
)
result = oil_call.price_with_metrics(market, as_of, ["pv", "delta", "vega"])
print(result.to_dataframe())
```

```rust
pub struct CrossCurrencySwap {
    pub id: InstrumentId,
    pub domestic_notional: Money,      // e.g., USD 10M
    pub foreign_notional: Money,       // e.g., EUR 8M
    pub fx_rate_at_trade: f64,         // spot at inception (1.25)
    pub domestic_leg: XccyLegSpec,     // fixed or floating
    pub foreign_leg: XccyLegSpec,
    pub basis_spread_bp: f64,          // cross-currency basis (e.g., -25bp on EUR leg)
    pub notional_exchange: NotionalExchangeSpec,
    pub discount_curve_domestic: CurveId,
    pub discount_curve_foreign: CurveId,
    pub forward_curve_domestic: Option<CurveId>,
    pub forward_curve_foreign: Option<CurveId>,
    pub fx_pair: String,               // "USDEUR"
    pub attributes: Attributes,
}

pub enum XccyLegSpec {
    Fixed { rate: f64, schedule: ScheduleSpec },
    Floating { index: String, spread_bp: f64, schedule: ScheduleSpec },
}

pub struct NotionalExchangeSpec {
    pub initial: bool,     // pay foreign notional, receive domestic at inception
    pub final: bool,       // reverse exchange at maturity
    pub intermediate_resets: Vec<Date>,  // for MTM XCCY swaps
}
```

```rust
let xccy = CrossCurrencySwap::new(
    "XCCY-001",
    Money::new(10_000_000.0, Currency::USD),
    Money::new(8_000_000.0, Currency::EUR),
    1.25,  // USD/EUR at trade
)
.domestic_leg_floating("USD-SOFR", 0.0, quarterly_schedule)
.foreign_leg_floating("EUR-ESTR", -25.0, quarterly_schedule)  // -25bp basis
.with_notional_exchange(true, true, vec![])
.build()?;

let result = xccy.price_with_metrics(&market, as_of, &[
    MetricId::Dv01Domestic,
    MetricId::Dv01Foreign,
    MetricId::FxDelta,
    MetricId::BasisDv01,  // new: sensitivity to cross-currency basis
])?;
```

```python
xccy = CrossCurrencySwap(
    id="XCCY-001",
    domestic_notional=Money(10_000_000, "USD"),
    foreign_notional=Money(8_000_000, "EUR"),
    fx_rate_at_trade=1.25,
    domestic_leg=FloatingLeg("USD-SOFR", spread_bp=0, schedule=...),
    foreign_leg=FloatingLeg("EUR-ESTR", spread_bp=-25, schedule=...),
    notional_exchange=NotionalExchange(initial=True, final=True),
)
result = xccy.price_with_metrics(market, as_of, ["dv01_domestic", "dv01_foreign", "fx_delta"])
df = result.to_polars()  # columns: [metric, value, ccy]
```

```plaintext
Cross-Currency Swap XCCY-001:
  PV_domestic_leg: $45,230 USD
  PV_foreign_leg: -€36,100 EUR (= -$45,050 USD @ 1.248)
  FX_basis_value: $180 USD (difference driven by -25bp basis)
  Net_PV: $360 USD
  Sensitivities:
    DV01_USD: -$8,450 per bp
    DV01_EUR: $6,920 per bp (in USD equiv)
    FX_Delta: $8,000,000 (notional exposure)
    Basis_DV01: -$120 per bp basis widening
```

```python
# Notebook: xccy_basis_swap_analysis.ipynb
# 1. Build 5Y USD SOFR vs EUR ESTR + basis swap
# 2. Calibrate USD/EUR OIS curves from market quotes
# 3. Price swap, compute bucketed DV01 per currency
# 4. Scenario: basis widens -25bp → -30bp, show P&L impact
# 5. Export cashflow waterfall to DataFrame for audit
```

```rust
pub struct XvaCalculator {
    pub netting_set_id: String,
    pub instruments: Vec<Arc<dyn Instrument>>,
    pub counterparty_curve: CurveId,  // hazard curve
    pub own_curve: Option<CurveId>,   // for DVA
    pub csa: Option<CsaAgreement>,
    pub config: XvaConfig,
}

pub struct CsaAgreement {
    pub threshold: Money,
    pub minimum_transfer_amount: Money,
    pub independent_amount: Money,
    pub collateral_haircut: f64,
    pub remargin_frequency_days: u32,
}

pub struct XvaConfig {
    pub simulation_dates: Vec<Date>,
    pub num_paths: usize,
    pub risk_free_curve: CurveId,
    pub funding_spread_bp: f64,  // for FVA
    pub include_cva: bool,
    pub include_dva: bool,
    pub include_fva: bool,
}

pub struct XvaResult {
    pub cva: Money,               // credit valuation adjustment
    pub dva: Money,               // debit valuation adjustment (own default)
    pub fva: Money,               // funding valuation adjustment
    pub mva: Money,               // margin valuation adjustment
    pub kva: Money,               // capital valuation adjustment
    pub bcva: Money,              // bilateral CVA = CVA - DVA
    pub expected_exposure: Vec<(Date, Money)>,  // EE curve
    pub potential_future_exposure_95: Vec<(Date, Money)>,  // PFE 95th percentile
    pub explanation: XvaExplanation,
}
```

```rust
let xva_calc = XvaCalculator::new("NettingSet-001")
    .add_instruments(vec![swap1, swap2, swap3])
    .counterparty_curve("CORP-A-USD")
    .own_curve("BANK-USD")
    .csa(CsaAgreement {
        threshold: Money::new(5_000_000.0, Currency::USD),
        minimum_transfer_amount: Money::new(500_000.0, Currency::USD),
        independent_amount: Money::zero(Currency::USD),
        collateral_haircut: 0.02,
        remargin_frequency_days: 1,
    })
    .config(XvaConfig {
        simulation_dates: monthly_dates,
        num_paths: 10_000,
        risk_free_curve: "USD-OIS".into(),
        funding_spread_bp: 50.0,
        include_cva: true,
        include_dva: true,
        include_fva: true,
    })
    .build()?;

let xva_result = xva_calc.calculate(&market, as_of)?;
println!("CVA: {}", xva_result.cva);          // e.g., -$125,000
println!("DVA: {}", xva_result.dva);          // e.g., +$30,000
println!("BCVA: {}", xva_result.bcva);        // e.g., -$95,000
println!("FVA: {}", xva_result.fva);          // e.g., -$18,000

// Export EE curve to DataFrame
let ee_df = xva_result.expected_exposure_to_polars();
```

```python
xva = XvaCalculator(
    netting_set_id="NettingSet-001",
    instruments=[swap1, swap2, swap3],
    counterparty_curve="CORP-A-USD",
    own_curve="BANK-USD",
    csa=CsaAgreement(threshold=5_000_000, mta=500_000, ia=0, haircut=0.02),
)
result = xva.calculate(market, as_of, num_paths=10_000, funding_spread_bp=50)
print(f"CVA: {result.cva}, DVA: {result.dva}, BCVA: {result.bcva}")
ee_df = result.expected_exposure_df()  # Polars/Pandas DataFrame
```

```plaintext
XVA for Netting Set NettingSet-001:
  Instruments: 3 swaps (IRS-001, IRS-002, CDS-001)
  Counterparty: CORP-A (hazard rate 150bp)
  CSA: Threshold $5M, daily VM
  
  CVA (Credit Valuation Adjustment): -$125,000
    - Expected Positive Exposure (EPE): $2.1M average
    - Counterparty PD (5Y): 7.2%
    - Loss Given Default: 60%
    
  DVA (Debit Valuation Adjustment): +$30,000
    - Expected Negative Exposure (ENE): $0.8M average
    - Own PD (5Y): 1.5%
    
  FVA (Funding Valuation Adjustment): -$18,000
    - Funding spread: 50bp
    - Average unfunded exposure: $1.2M
    
  Bilateral CVA (BCVA): -$95,000
```

```python
# Notebook: cva_calculation_example.ipynb
# 1. Build portfolio of 3 IRS (receiver swaps, NPV = +$2M)
# 2. Calibrate counterparty hazard curve from CDS spreads (BBB-rated)
# 3. Simulate exposure paths (1000 scenarios, monthly grid)
# 4. Calculate CVA with 60% LGD
# 5. Compare: (a) uncollateralized, (b) CSA with $5M threshold, (c) daily VM
# 6. Show EE/PFE curves, CVA breakdown by instrument
```

```rust
pub struct SimmCalculator {
    pub portfolio_id: String,
    pub version: SimmVersion,  // e.g., SIMM 2.6 (Dec 2023)
}

pub enum SimmVersion {
    Simm26,  // December 2023
    Simm25,
}

pub struct SimmInputs {
    pub sensitivities: Vec<RiskFactorSensitivity>,
    pub product_class: ProductClass,  // RatesFX, Credit, Equity, Commodity
}

pub struct RiskFactorSensitivity {
    pub risk_type: RiskType,       // Delta, Vega, Curvature
    pub risk_class: RiskClass,     // IR, FX, EQ, CO, CR
    pub qualifier: String,         // e.g., "USD", "EUR", "AAPL", "CDX.IG"
    pub bucket: Option<String>,    // SIMM bucket (e.g., "1" for G10 FX)
    pub label1: Option<String>,    // e.g., tenor "2Y"
    pub label2: Option<String>,    // e.g., strike for vega
    pub amount_base_ccy: f64,      // sensitivity in base currency
}

pub struct SimmResult {
    pub initial_margin: Money,
    pub im_by_risk_class: HashMap<RiskClass, Money>,
    pub diversification_benefit: Money,
    pub gross_im: Money,  // sum before diversification
    pub explanation: SimmExplanation,
}
```

```rust
let simm = SimmCalculator::new("Portfolio-001", SimmVersion::Simm26);

// Collect sensitivities from instruments
let mut sensitivities = vec![];
for inst in portfolio.instruments() {
    let result = inst.price_with_metrics(&market, as_of, &[
        MetricId::BucketedDv01,
        MetricId::BucketedVega,
        MetricId::FxDelta,
    ])?;
    sensitivities.extend(extract_simm_sensitivities(&result, inst)?);
}

let inputs = SimmInputs {
    sensitivities,
    product_class: ProductClass::RatesFX,
};

let simm_result = simm.calculate(inputs, Currency::USD)?;
println!("Total IM: {}", simm_result.initial_margin);  // e.g., $8.2M
println!("IM (IR): {}", simm_result.im_by_risk_class[&RiskClass::IR]);  // $6.5M
println!("IM (FX): {}", simm_result.im_by_risk_class[&RiskClass::FX]);  // $1.9M
println!("Diversification benefit: {}", simm_result.diversification_benefit);  // -$0.2M
```

```python
simm = SimmCalculator(portfolio_id="Portfolio-001", version="SIMM_2.6")
sensitivities = collect_sensitivities_from_portfolio(portfolio, market, as_of)
result = simm.calculate(sensitivities, base_currency="USD")
print(f"Total IM: {result.initial_margin}")
df = result.to_dataframe()  # breakdown by risk class, bucket
```

```plaintext
ISDA SIMM 2.6 for Portfolio-001:
  Product Class: RatesFX
  
  Initial Margin by Risk Class:
    IR (Interest Rate): $6.5M
      - Delta: $6.2M (USD: $4.1M, EUR: $1.8M, GBP: $0.3M)
      - Vega: $0.3M
    FX (Foreign Exchange): $1.9M
      - Delta: $1.9M (EURUSD: $1.2M, GBPUSD: $0.7M)
    
  Gross IM (before diversification): $8.4M
  Diversification benefit: -$0.2M (2.4%)
  Total IM: $8.2M
```

```python
# Notebook: isda_simm_margin_calculation.ipynb
# 1. Build portfolio: 5 IRS (USD, EUR), 2 FX Options, 1 CDS
# 2. Compute bucketed sensitivities (DV01 by tenor, vega by expiry/strike)
# 3. Calculate SIMM IM per ISDA methodology
# 4. Show impact of adding offsetting position (IM reduction from netting)
# 5. Export IM breakdown to CSV for regulatory reporting
```

```rust
pub struct CommodityForward {
    pub id: InstrumentId,
    pub commodity: Commodity,  // enum: WTI, Brent, NatGas, Gold, ...
    pub quantity: f64,         // barrels, MMBtu, troy oz
    pub delivery_date: Date,
    pub forward_price: f64,    // locked-in price
    pub forward_curve_id: CurveId,
    pub discount_curve_id: CurveId,
    pub attributes: Attributes,
}

pub enum Commodity {
    WtiCrude,
    BrentCrude,
    NaturalGas,   // Henry Hub
    Gold,
    Silver,
    Copper,
    Power(PowerHub),  // e.g., PJM West, ERCOT North
}

pub struct CommodityOption {
    pub underlying: Commodity,
    pub strike: f64,
    pub expiry: Date,
    pub option_type: OptionType,  // Call, Put
    pub quantity: f64,
    pub forward_curve_id: CurveId,
    pub vol_surface_id: String,  // 2D surface: expiry × moneyness
    pub discount_curve_id: CurveId,
}
```

```rust
let oil_forward = CommodityForward::new(
    "OIL-FWD-001",
    Commodity::WtiCrude,
    1000.0,  // 1000 barrels
    maturity_date,
    85.50,   // $85.50/bbl locked in
)
.forward_curve("WTI-USD")
.discount_curve("USD-OIS")
.build()?;

let pv = oil_forward.value(&market, as_of)?;

let oil_call = CommodityOption::european_call(
    "OIL-CALL-001",
    Commodity::WtiCrude,
    90.0,    // $90 strike
    expiry,
    1000.0,
)
.vol_surface("WTI-VOL")
.forward_curve("WTI-USD")
.build()?;

let greeks = oil_call.price_with_metrics(&market, as_of, &[
    MetricId::Delta,
    MetricId::Gamma,
    MetricId::Vega,
])?;
```

```python
oil_fwd = CommodityForward(
    id="OIL-FWD-001",
    commodity="WTI",
    quantity=1000,
    delivery_date=maturity,
    forward_price=85.50,
    forward_curve="WTI-USD",
)
pv = oil_fwd.value(market, as_of)

oil_call = CommodityOption.european_call(
    commodity="WTI",
    strike=90.0,
    expiry=expiry,
    quantity=1000,
    vol_surface="WTI-VOL",
)
result = oil_call.price_with_metrics(market, as_of, ["delta", "vega"])
```

```plaintext
Commodity Call Option (WTI Crude):
  Spot: $87.20/bbl
  Forward (3M): $88.10/bbl
  Strike: $90.00/bbl
  Vol: 28% (ATM)
  PV: $2,150 (for 1000 bbls)
  
  Greeks:
    Delta: 0.48
    Gamma: 0.032
    Vega: $45 per 1% vol
    Theta: -$8 per day
  
  Convenience yield implied: 1.8% p.a.
```

```python
# Notebook: commodity_derivatives_pricing.ipynb
# 1. Build WTI forward curve from futures prices (NYMEX CL1-CL12)
# 2. Calibrate ATM vol surface from market option quotes
# 3. Price European call on WTI (strike $90, 3M expiry)
# 4. Compute delta hedge: short 480 barrels of CL1 futures
# 5. Scenario: oil vol spikes 28% → 35%, show P&L impact
```

```rust
pub struct TransitionMatrix {
    pub id: String,
    pub ratings: Vec<CreditRating>,  // AAA, AA, A, BBB, BB, B, CCC, D
    pub matrix: Array2<f64>,         // [from_rating, to_rating] = probability
    pub horizon_years: f64,          // e.g., 1.0 for annual matrix
    pub source: MatrixSource,        // Moody's, S&P, Fitch, Custom
}

pub enum MatrixSource {
    MoodysHistorical,
    SPHistorical,
    FitchHistorical,
    Custom(String),
}

pub struct CreditMigrationCalculator {
    pub transition_matrix: TransitionMatrix,
    pub spread_curves: HashMap<CreditRating, CurveId>,  // hazard curve per rating
    pub config: MigrationConfig,
}

pub struct MigrationVaRResult {
    pub var_99: Money,  // 99th percentile loss from migration
    pub expected_loss: Money,
    pub unexpected_loss: Money,  // sqrt(variance)
    pub migration_scenarios: Vec<MigrationScenario>,
}

pub struct MigrationScenario {
    pub from_rating: CreditRating,
    pub to_rating: CreditRating,
    pub probability: f64,
    pub spread_change_bp: f64,
    pub pv_impact: Money,
}
```

```rust
// Load Moody's 1-year transition matrix
let transition_matrix = TransitionMatrix::load_moodys_annual()?;

let migration_calc = CreditMigrationCalculator::new(transition_matrix)
    .add_spread_curve(CreditRating::AAA, "AAA-USD")
    .add_spread_curve(CreditRating::AA, "AA-USD")
    .add_spread_curve(CreditRating::A, "A-USD")
    .add_spread_curve(CreditRating::BBB, "BBB-USD")
    .add_spread_curve(CreditRating::BB, "BB-USD")
    .build()?;

// Calculate migration VaR for a BBB-rated bond
let bond = Bond::fixed("CORP-BBB-001", notional, 0.05, issue, maturity, "BBB-USD");
let current_rating = CreditRating::BBB;

let migration_result = migration_calc.calculate_migration_var(
    &bond,
    current_rating,
    &market,
    as_of,
    horizon_years: 1.0,
)?;

println!("Migration VaR (99%): {}", migration_result.var_99);  // e.g., -$85,000
println!("Expected credit loss: {}", migration_result.expected_loss);  // e.g., -$12,000
```

```python
transition_matrix = TransitionMatrix.load_moodys_annual()
migration_calc = CreditMigrationCalculator(
    transition_matrix=transition_matrix,
    spread_curves={"AAA": "AAA-USD", "BBB": "BBB-USD", ...},
)

bond = Bond.fixed("CORP-BBB-001", notional=10_000_000, coupon=0.05, ...)
result = migration_calc.calculate_migration_var(
    bond=bond,
    current_rating="BBB",
    market=market,
    as_of=as_of,
    horizon_years=1.0,
)
print(f"Migration VaR (99%): {result.var_99}")
df = result.migration_scenarios_df()  # Polars DataFrame
```

```plaintext
Credit Migration VaR for CORP-BBB-001:
  Current Rating: BBB
  Horizon: 1 year
  
  Migration Probabilities (from Moody's):
    Stay BBB: 86.5%
    Upgrade to A: 5.2%
    Downgrade to BB: 6.8%
    Downgrade to B: 0.9%
    Default: 0.3%
    
  Spread Changes:
    BBB → BB: +180bp (fallen angel), PV impact: -$850K (prob: 6.8%)
    BBB → B: +420bp, PV impact: -$1.9M (prob: 0.9%)
    BBB → D: total loss, PV impact: -$10M (prob: 0.3%)
    
  Migration VaR (99%): -$85,000
  Expected credit loss: -$12,000 (weighted average)
```

```python
# Notebook: credit_migration_var.ipynb
# 1. Load S&P 1-year transition matrix (1981-2020 historical)
# 2. Build BBB-rated corporate bond portfolio (10 bonds)
# 3. Calibrate spread curves per rating (AAA to CCC)
# 4. Calculate migration VaR (99%) for 1-year horizon
# 5. Show dominant risk: BBB → BB fallen angel scenario
# 6. Compare with default-only VaR (understates risk)
```

```json
{
  "id": "XCCY-001",
  "domestic_notional": { "amount": 10000000.0, "currency": "USD" },
  "foreign_notional": { "amount": 8000000.0, "currency": "EUR" },
  "fx_rate_at_trade": 1.25,
  "domestic_leg": {
    "type": "floating",
    "index": "USD-SOFR",
    "spread_bp": 0.0,
    "schedule": { "frequency": "3M", "start": "2025-01-15", "end": "2030-01-15" }
  },
  "foreign_leg": {
    "type": "floating",
    "index": "EUR-ESTR",
    "spread_bp": -25.0,
    "schedule": { "frequency": "3M", "start": "2025-01-15", "end": "2030-01-15" }
  },
  "notional_exchange": { "initial": true, "final": true, "intermediate_resets": [] }
}
```

```json
{
  "risk_type": "Delta",
  "risk_class": "IR",
  "qualifier": "USD",
  "bucket": null,
  "label1": "2Y",
  "label2": null,
  "amount_base_ccy": -125000.0
}
```

```python
# Step 1: Build WTI forward curve
wti_curve = CommodityForwardCurve.from_futures_prices(
    commodity="WTI",
    futures_prices={
        "2025-03": 87.20,
        "2025-06": 88.10,
        "2025-09": 88.80,
        "2025-12": 89.30,
    },
    discount_curve="USD-OIS",
)

# Step 2: Calibrate vol surface
wti_vol_surface = VolSurface.from_market_quotes(
    surface_id="WTI-VOL",
    quotes=[
        {"expiry": "3M", "strike_delta": 0.5, "vol": 0.28},  # ATM
        {"expiry": "3M", "strike_delta": 0.25, "vol": 0.32},  # OTM call
        # ... more quotes
    ],
)

# Step 3: Price European call
oil_call = CommodityOption.european_call(
    commodity="WTI",
    strike=90.0,
    expiry=date(2025, 6, 15),
    quantity=1000,
    vol_surface="WTI-VOL",
    forward_curve="WTI-USD",
)
result = oil_call.price_with_metrics(market, as_of, ["pv", "delta", "vega"])
print(result.to_dataframe())
```

# 📎 Fast-Follow Features (Detailed)

## Quick Win 1: Advanced Cap/Floor Pricing (Normal & Shifted Lognormal Volatility)

**Persona Pain**: Quants pricing EUR or JPY interest rate caps cannot use Normal (Bachelier) volatility or shifted lognormal models, which are market-standard in negative/low rate environments. Current Black76 pricing breaks down when rates approach zero or go negative.

**User Story**: *As a rates quant, I need to price EUR interest rate caps using Normal volatility quotes (e.g., 50bp vol instead of 20% lognormal vol) with Bachelier formula, or use shifted lognormal with a shift parameter to handle negative rates, so that I can match broker quotes and accurately hedge cap positions in low-rate currencies.*

**Scope (what's new)**:

**Data**:
- Volatility model enum: `Black76`, `Normal` (Bachelier), `ShiftedLognormal`
- Shift parameter for shifted lognormal (e.g., 3% shift for EUR)
- Vol surface stores model type + parameters

```rust
pub enum VolatilityModel {
    /// Black76 (lognormal): assumes dF = σF dW
    Black76,
    /// Normal (Bachelier): assumes dF = σ dW (absolute vol in bp)
    Normal,
    /// Shifted Lognormal: assumes d(F+shift) = σ(F+shift) dW
    ShiftedLognormal { shift: f64 },
}

pub struct CapFloorPricingConfig {
    pub vol_model: VolatilityModel,
    pub smile_interpolation: SmileInterpolation,  // e.g., SABR, Linear
}

// Extend existing CapFloor instrument
impl RateOptionType {
    pub fn price_with_model(
        &self,
        forward_rate: f64,
        strike: f64,
        vol: f64,
        time_to_expiry: f64,
        discount_factor: f64,
        model: VolatilityModel,
    ) -> Result<f64> {
        match model {
            VolatilityModel::Black76 => {
                // Existing Black76 formula
                black76::caplet_price(...)
            }
            VolatilityModel::Normal => {
                // New: Bachelier formula
                bachelier::caplet_price(forward_rate, strike, vol, time_to_expiry, discount_factor)
            }
            VolatilityModel::ShiftedLognormal { shift } => {
                // New: Black76 with shifted forward
                black76::caplet_price(forward_rate + shift, strike + shift, vol, ...)
            }
        }
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::cap_floor::RateOptionType;
use finstack_valuations::instruments::common::models::volatility::VolatilityModel;

// Price EUR cap with Normal vol (market convention)
let cap = RateOptionType::Cap
    .with_strikes(vec![0.00, 0.005, 0.01])  // 0%, 0.5%, 1% strikes
    .with_volatility_model(VolatilityModel::Normal)
    .build()?;

let pricing_config = PricingOverrides::default()
    .with_vol_model(VolatilityModel::Normal);

let result = cap.price_with_metrics(&market, as_of, &[MetricId::Vega])?;

// Alternative: Shifted lognormal for negative rates
let config_shifted = PricingOverrides::default()
    .with_vol_model(VolatilityModel::ShiftedLognormal { shift: 0.03 });  // 3% shift
```

*Python*:
```python
from finstack import CapFloor, VolatilityModel

# Normal vol model (Bachelier)
cap = CapFloor(
    id="EUR-CAP-001",
    notional=10_000_000,
    strike=0.005,  # 50bp
    index="EUR-EURIBOR-6M",
    vol_model=VolatilityModel.NORMAL,
)
result = cap.price(market, as_of, vol=0.0050)  # 50bp normal vol

# Shifted lognormal
cap_shifted = cap.with_vol_model(VolatilityModel.SHIFTED_LOGNORMAL, shift=0.03)
result2 = cap_shifted.price(market, as_of, vol=0.20)  # 20% lognormal vol on shifted rate
```

**Explainability**: `explain()` output:
```
Interest Rate Cap EUR-CAP-001:
  Model: Normal (Bachelier)
  Strike: 0.50%
  Forward rate: 0.35%
  Normal vol: 50bp (0.50%)
  Time to expiry: 2.5Y
  
  Caplet prices (per period):
    2025-06-15: €1,250 (forward 0.30%, strike 0.50%, ITM prob: 15%)
    2025-12-15: €1,580 (forward 0.35%, strike 0.50%, ITM prob: 18%)
    2026-06-15: €1,820 (forward 0.40%, strike 0.50%, ITM prob: 21%)
    
  Total PV: €12,450
  Vega (per 1bp vol): €820
  
  Note: Normal vol model used (market standard for EUR caps in low-rate environment)
```

**Validation**:
- Compare EUR cap prices with **Bloomberg VCUB** (Vol Cube) using Normal vol quotes
- Test shifted lognormal: verify `price(F, K, shift=s) = price_black76(F+s, K+s, shift=0)`
- Property test: Normal vol → 0 smoothly as rates → 0 (unlike Black76 which diverges)
- Regression test: Reproduce caplet prices from Brigo & Mercurio *Interest Rate Models* (Section 1.5)

**Impact & Effort**: P1; Small (3-5 days)
- **Dependencies**: Black76 formula exists, add Bachelier formula (textbook implementation)
- **Risks**: Vol surface conversion between models (Normal ↔ Black76)
- **Mitigations**: Provide vol conversion utilities; document model choice in pricing metadata

**Demo Outline**:
```python
# Notebook: eur_cap_normal_vol_pricing.ipynb
# 1. Load EUR cap market quotes (Normal vol from broker)
# 2. Price 5Y EUR cap (strike 50bp) with Normal model
# 3. Compare: (a) Normal vol, (b) Black76 with implied lognormal vol
# 4. Show breakdown: Black76 overprices when forward rate near zero
# 5. Compute vega in bp terms (Normal vol sensitivity)
```

**Why Now**:
- **Market standard**: EUR and JPY caps quoted in Normal vol since 2015+
- **Negative rates**: Black76 fails for negative strikes; Normal model works
- **Regulatory**: FRTB requires accurate vol sensitivities in all rate regimes

**De-Dup Evidence**:
- Searched: `"Normal volatility"`, `"Bachelier"`, `"shifted lognormal"` in `cap_floor/` → **Not found**
- `cap_floor/pricing/black.rs` exists with Black76 only
- No `VolatilityModel` enum in common models

---

## Quick Win 2: Credit-Linked Notes (CLN)

**Persona Pain**: Structured credit traders cannot price credit-linked notes (bonds with embedded CDS protection) where principal is at risk if reference entity defaults, forcing manual combination of bond + CDS pricing.

**User Story**: *As a structured credit trader, I need to price a credit-linked note that pays 5% coupon quarterly but loses principal if Corporation X defaults (linked to CDS), with both bond cashflows and CDS protection mechanics integrated, so that I can offer CLNs to investors seeking credit exposure with bond-like payoff.*

**Scope (what's new)**:

**Data**:
- Wrapper instrument combining `Bond` + `CreditDefaultSwap` reference
- Principal-at-risk if reference entity defaults
- Enhanced coupon to compensate for credit risk

```rust
pub struct CreditLinkedNote {
    pub id: InstrumentId,
    pub bond: Bond,  // Underlying bond structure
    pub reference_entity: String,  // CDS reference
    pub credit_curve_id: CurveId,  // Hazard curve for reference
    pub recovery_rate: f64,
    pub notional_at_risk: bool,  // true = principal lost on default
    pub coupon_enhancement_bp: f64,  // additional spread vs. risk-free
    pub attributes: Attributes,
}

impl CreditLinkedNote {
    /// Price CLN as bond discounted with credit-adjusted cashflows
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let hazard = context.get_hazard_ref(&self.credit_curve_id)?;
        let disc = context.get_discount_ref(&self.bond.discount_curve_id)?;
        
        // Generate bond cashflows
        let schedule = self.bond.build_schedule(context, as_of)?;
        
        let mut pv = Money::zero(self.bond.notional.currency());
        for (date, flow) in schedule {
            let t = disc.year_fraction_to_date(as_of, date)?;
            let df = disc.df(t);
            let sp = hazard.sp(t);  // Survival probability to date
            
            // Coupon: paid if survived
            let expected_flow = flow.amount() * sp;
            pv = pv.checked_add(Money::new(expected_flow * df, flow.currency()))?;
        }
        
        // Principal: at risk if notional_at_risk = true
        if self.notional_at_risk {
            let maturity_t = disc.year_fraction_to_date(as_of, self.bond.maturity)?;
            let sp_maturity = hazard.sp(maturity_t);
            let df_maturity = disc.df(maturity_t);
            let expected_principal = self.bond.notional.amount() * sp_maturity * df_maturity;
            
            // Add recovery value on default
            let expected_recovery = self.bond.notional.amount() * (1.0 - sp_maturity) * self.recovery_rate * df_maturity;
            
            pv = pv.checked_add(Money::new(expected_principal + expected_recovery, self.bond.notional.currency()))?;
        }
        
        Ok(pv)
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::credit_linked_note::CreditLinkedNote;

// Build 5Y CLN linked to CORP-A default
let bond_spec = Bond::fixed(
    "CLN-BOND",
    Money::new(10_000_000.0, Currency::USD),
    0.05,  // 5% coupon
    issue_date,
    maturity_date,
    "USD-OIS",
);

let cln = CreditLinkedNote::new("CLN-001", bond_spec)
    .reference_entity("CORP-A")
    .credit_curve("CORP-A-USD-CDS")
    .recovery_rate(0.40)
    .notional_at_risk(true)  // Principal lost on default
    .coupon_enhancement_bp(200.0)  // 200bp over risk-free for credit risk
    .build()?;

let pv = cln.value(&market, as_of)?;
let metrics = cln.price_with_metrics(&market, as_of, &[
    MetricId::Cs01,
    MetricId::Recovery01,
    MetricId::JumpToDefault,
])?;
```

*Python*:
```python
from finstack import CreditLinkedNote, Bond, Money

bond = Bond.fixed(
    id="CLN-BOND",
    notional=Money(10_000_000, "USD"),
    coupon=0.05,
    issue=issue_date,
    maturity=maturity_date,
)

cln = CreditLinkedNote(
    id="CLN-001",
    bond=bond,
    reference_entity="CORP-A",
    credit_curve="CORP-A-USD-CDS",
    recovery_rate=0.40,
    notional_at_risk=True,
    coupon_enhancement_bp=200,
)

result = cln.price_with_metrics(market, as_of, ["pv", "cs01", "recovery01"])
print(f"CLN PV: {result.pv}")
print(f"CS01: {result.metrics['cs01']}")  # Credit spread sensitivity
```

**Explainability**: `explain()` output:
```
Credit-Linked Note CLN-001:
  Reference Entity: CORP-A
  Bond Structure:
    Notional: $10,000,000
    Coupon: 5.00% (includes 200bp credit enhancement)
    Maturity: 5 years
    
  Credit Risk:
    Hazard rate: 150bp (BBB-rated)
    5Y survival probability: 92.8%
    Recovery rate: 40%
    
  Cashflow Valuation:
    Expected coupons: $2,320,000 (= $2,500,000 × 92.8%)
    Expected principal: $9,280,000 (survived) + $288,000 (recovery) = $9,568,000
    
  PV: $9,888,000 (98.88% of par)
  
  Sensitivities:
    CS01: -$4,850 per bp (widening hurts)
    Recovery01: +$7,200 per 1% recovery
    Jump-to-default: -$6,000,000 (immediate default scenario)
```

**Validation**:
- Test CLN pricing vs. synthetic: `CLN = Risk-free Bond - CDS protection sold`
- Verify `CLN_PV + CDS_protection_bought = Risk_free_bond_PV` (replication)
- Compare with Bloomberg `SRCH CLN <GO>` for market CLN prices
- Property test: Higher hazard rate → lower CLN price

**Impact & Effort**: P1; Small (1 week)
- **Dependencies**: Bond (exists), CDS pricing (exists), hazard curves (exist)
- **Risks**: Accrued interest on default, auction settlement mechanics
- **Mitigations**: Start with simple cash settlement; add auction mechanics in phase 2

**Demo Outline**:
```python
# Notebook: credit_linked_note_structuring.ipynb
# 1. Build vanilla 5Y bond (5% coupon, risk-free)
# 2. Convert to CLN: link principal to CORP-A credit (BBB-rated)
# 3. Calculate required coupon enhancement (200bp) for par pricing
# 4. Compare: (a) CLN PV, (b) Bond PV - CDS PV (should match)
# 5. Scenario: CORP-A downgrade (spread 150bp → 250bp), show CLN mark-down
```

**Why Now**:
- **Investor demand**: Retail/institutional seeking credit exposure without CDS mechanics
- **Structured products**: CLNs are building blocks for bespoke structures
- **Balance sheet**: Banks use CLNs to transfer credit risk off balance sheet

**De-Dup Evidence**:
- Searched: `"CreditLinkedNote"`, `"CLN"`, `"credit linked note"` → **Not found**
- Bond exists, CDS exists, but no combined CLN wrapper

---

## Quick Win 3: Real Yield Curves (Explicit Real Rate Term Structure)

**Persona Pain**: Inflation traders and TIPS analysts cannot extract real yields directly from nominal and inflation curves, forcing manual spreadsheet calculations to back out breakeven inflation and real rates.

**User Story**: *As a TIPS trader, I need to compute the real yield curve (real zero rates) from nominal Treasury curve and inflation swap curve, derive breakeven inflation at each tenor, and price inflation-linked bonds directly off the real curve, so that I can identify rich/cheap opportunities in TIPS vs. nominals.*

**Scope (what's new)**:

**Data**:
- Derived curve type: `RealYieldCurve = (1 + nominal) / (1 + inflation) - 1`
- Breakeven inflation helpers
- Direct TIPS pricing from real curve

```rust
pub struct RealYieldCurve {
    pub id: CurveId,
    pub nominal_curve_id: CurveId,
    pub inflation_curve_id: CurveId,
    pub base_date: Date,
    pub day_count: DayCount,
}

impl RealYieldCurve {
    /// Compute real discount factor: DF_real(t) = DF_nominal(t) / (1 + inflation(t))
    pub fn real_df(&self, t: f64, context: &MarketContext) -> Result<f64> {
        let nominal_disc = context.get_discount_ref(&self.nominal_curve_id)?;
        let inflation = context.get_inflation_ref(&self.inflation_curve_id)?;
        
        let df_nominal = nominal_disc.df(t);
        let inflation_index = inflation.index_at(t)?;  // CPI ratio
        
        // Real discount factor
        Ok(df_nominal / inflation_index)
    }
    
    /// Compute real zero rate: r_real = (DF_real(0) / DF_real(t))^(1/t) - 1
    pub fn real_zero_rate(&self, t: f64, context: &MarketContext) -> Result<f64> {
        if t <= 0.0 {
            return Ok(0.0);
        }
        let df_real = self.real_df(t, context)?;
        Ok((1.0 / df_real).powf(1.0 / t) - 1.0)
    }
    
    /// Breakeven inflation: (1 + nominal) / (1 + real) - 1
    pub fn breakeven_inflation(&self, t: f64, context: &MarketContext) -> Result<f64> {
        let nominal_disc = context.get_discount_ref(&self.nominal_curve_id)?;
        let df_nominal = nominal_disc.df(t);
        let df_real = self.real_df(t, context)?;
        
        let nominal_zero = (1.0 / df_nominal).powf(1.0 / t) - 1.0;
        let real_zero = (1.0 / df_real).powf(1.0 / t) - 1.0;
        
        // Fisher equation: (1 + nominal) = (1 + real)(1 + inflation)
        Ok((1.0 + nominal_zero) / (1.0 + real_zero) - 1.0)
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_core::market_data::RealYieldCurve;

// Build real yield curve from nominal + inflation
let real_curve = RealYieldCurve::new("USD-REAL")
    .nominal_curve("USD-TREASURY")
    .inflation_curve("USD-CPI-SWAP")
    .base_date(as_of)
    .build()?;

// Add to market context
let market = market.insert_real_yield_curve(real_curve)?;

// Extract real rates and breakevens
let tenors = vec![1.0, 2.0, 5.0, 10.0, 30.0];
for t in tenors {
    let real_rate = real_curve.real_zero_rate(t, &market)?;
    let breakeven = real_curve.breakeven_inflation(t, &market)?;
    println!("{:.0}Y: Real={:.2}%, Breakeven={:.2}%", t, real_rate * 100.0, breakeven * 100.0);
}

// Price TIPS directly using real curve
let tips = InflationLinkedBond::new(...)
    .real_curve("USD-REAL")  // Direct pricing from real curve
    .build()?;
```

*Python*:
```python
from finstack import RealYieldCurve

# Derive real curve
real_curve = RealYieldCurve(
    id="USD-REAL",
    nominal_curve="USD-TREASURY",
    inflation_curve="USD-CPI-SWAP",
)
market = market.add_curve(real_curve)

# Extract term structure
tenors = [1, 2, 5, 10, 30]
real_rates = [real_curve.real_zero_rate(t, market) for t in tenors]
breakevens = [real_curve.breakeven_inflation(t, market) for t in tenors]

# DataFrame export
df = pl.DataFrame({
    "tenor": tenors,
    "real_rate": real_rates,
    "breakeven_inflation": breakevens,
})
print(df)

# Price TIPS using real curve
tips = InflationLinkedBond(...)
result = tips.price(market, as_of, real_curve="USD-REAL")
```

**Explainability**: `explain()` output:
```
Real Yield Curve USD-REAL:
  Derived from:
    Nominal: USD-TREASURY (nominal zero rates)
    Inflation: USD-CPI-SWAP (zero-coupon inflation swaps)
    
  Term Structure (as of 2025-01-15):
    
    Tenor | Nominal | Real | Breakeven
    ------|---------|------|----------
      1Y  |  4.50%  | 1.80%|  2.64%
      2Y  |  4.35%  | 1.65%|  2.66%
      5Y  |  4.20%  | 1.50%|  2.66%
     10Y  |  4.30%  | 1.60%|  2.66%
     30Y  |  4.50%  | 1.75%|  2.70%
     
  Interpretation:
    - 10Y breakeven inflation: 2.66% (market expects 2.66% avg CPI over 10Y)
    - 10Y real rate: 1.60% (inflation-adjusted return on TIPS)
    - Fisher equation: (1.043) ≈ (1.016)(1.0266) ✓
```

**Validation**:
- Verify Fisher equation: `(1 + nominal) = (1 + real)(1 + inflation)` holds to 1bp
- Compare 10Y breakeven with Bloomberg `USGG10YR <Govt> DES <GO>` → Breakeven field
- Test TIPS pricing: TIPS via real curve = TIPS via nominal curve + inflation adjustments
- Property test: Real rate < Nominal rate (unless deflation expected)

**Impact & Effort**: P1; Small (1 week)
- **Dependencies**: DiscountCurve (exists), InflationCurve (exists), Fisher equation (trivial math)
- **Risks**: Compounding frequency mismatches, CPI lag conventions
- **Mitigations**: Document clearly: assumes continuous compounding; handle CPI lag in inflation curve

**Demo Outline**:
```python
# Notebook: real_yield_curve_analysis.ipynb
# 1. Calibrate USD nominal curve from Treasury yields
# 2. Calibrate USD inflation curve from ZC inflation swaps
# 3. Derive real yield curve
# 4. Plot all three: nominal, real, breakeven over 1-30Y
# 5. Compare TIPS pricing: (a) direct from inflation-linked bond pricer, (b) from real curve
# 6. Identify: 10Y TIPS trading rich vs. breakeven (arbitrage opportunity)
```

**Why Now**:
- **Inflation regime**: Post-2021 inflation surge → renewed focus on real yields
- **TIPS liquidity**: Growing TIPS market, institutional demand
- **Regulatory**: IFRS requires breakeven inflation disclosure for pension liabilities

**De-Dup Evidence**:
- Searched: `"real yield"`, `"real rate"`, `"RealYieldCurve"` → **Not found**
- `InflationCurve` exists, `DiscountCurve` exists, but no derived real curve type
- `InflationLinkedBond` prices using nominal + CPI indexing, NOT direct real curve

---

## Quick Win 4: Multi-Callable Bonds (Call Schedules)

**Persona Pain**: Corporate bond traders cannot price bonds with multiple call dates (e.g., callable quarterly after 5Y lockout) or make-whole call provisions, limiting yield-to-worst and OAS calculations.

**User Story**: *As a corporate bond trader, I need to price a 10Y bond callable quarterly starting year 5 at par, with make-whole provisions before year 5, to compute yield-to-worst (minimum of YTM vs. all call yields) and option-adjusted spread, so that I can value embedded optionality and advise issuers on call timing.*

**Scope (what's new)**:

**Data**:
- Extend `Bond` to support `CallSchedule` (currently single `call_date`)
- Make-whole call: redemption at PV of remaining cashflows + spread
- Yield-to-worst: min yield across all possible call dates

```rust
pub struct CallSchedule {
    pub lockout_date: Option<Date>,  // No calls before this date
    pub call_dates: Vec<CallEvent>,
    pub make_whole: Option<MakeWholeSpec>,
}

pub struct CallEvent {
    pub call_date: Date,
    pub call_price: f64,  // % of par (e.g., 100.0, 102.5)
    pub is_mandatory: bool,  // true = issuer must call
}

pub struct MakeWholeSpec {
    pub treasury_curve: CurveId,
    pub spread_bp: f64,  // e.g., Treasury + 50bp for PV calc
}

// Extend Bond
pub struct Bond {
    // ... existing fields ...
    pub call_schedule: Option<CallSchedule>,  // Replaces single call_date
}

impl Bond {
    /// Compute yield to worst: min of YTM and yields to all call dates
    pub fn yield_to_worst(&self, market: &MarketContext, as_of: Date, dirty_price: f64) -> Result<f64> {
        let mut ytw = self.yield_to_maturity(dirty_price, as_of)?;
        
        if let Some(schedule) = &self.call_schedule {
            for call_event in &schedule.call_dates {
                if call_event.call_date > as_of {
                    let ytc = self.yield_to_call(dirty_price, as_of, call_event)?;
                    ytw = ytw.min(ytc);
                }
            }
        }
        
        Ok(ytw)
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::bond::{Bond, CallSchedule, CallEvent, MakeWholeSpec};

// Build 10Y corporate bond, callable quarterly starting year 5
let call_schedule = CallSchedule::new()
    .lockout_date(issue_date.add_years(5)?)
    .add_call_events(
        (issue_date.add_years(5)?..=maturity_date)
            .step_by_months(3)
            .map(|date| CallEvent {
                call_date: date,
                call_price: 100.0,  // Par
                is_mandatory: false,
            })
            .collect()
    )
    .make_whole(MakeWholeSpec {
        treasury_curve: "USD-TREASURY".into(),
        spread_bp: 50.0,  // Treasury + 50bp discount rate
    })
    .build()?;

let bond = Bond::fixed("CORP-CALL-001", notional, 0.045, issue, maturity, "USD-OIS")
    .callable(call_schedule)
    .build()?;

// Compute yield to worst
let dirty_price = 98.5;  // % of par
let ytw = bond.yield_to_worst(&market, as_of, dirty_price)?;
println!("YTW: {:.2}%", ytw * 100.0);

// Compute OAS with embedded optionality
let oas = bond.price_with_metrics(&market, as_of, &[MetricId::Oas])?
    .measures.get(&MetricId::Oas).unwrap();
```

*Python*:
```python
from finstack import Bond, CallSchedule, CallEvent, MakeWholeSpec
from datetime import timedelta

# Build call schedule: callable quarterly from year 5 onward
call_dates = [issue_date + timedelta(days=365*5 + 90*i) for i in range(20)]
call_schedule = CallSchedule(
    lockout_date=issue_date + timedelta(days=365*5),
    call_events=[CallEvent(date=d, call_price=100.0) for d in call_dates],
    make_whole=MakeWholeSpec(treasury_curve="USD-TREASURY", spread_bp=50),
)

bond = Bond.fixed(
    id="CORP-CALL-001",
    notional=10_000_000,
    coupon=0.045,
    issue=issue_date,
    maturity=maturity_date,
    callable=call_schedule,
)

# Compute yield to worst
ytw = bond.yield_to_worst(market, as_of, dirty_price=98.5)
print(f"YTW: {ytw:.2%}")

# All possible yields
ytm = bond.yield_to_maturity(dirty_price=98.5, as_of=as_of)
ytc_5y = bond.yield_to_call(dirty_price=98.5, as_of=as_of, call_date=call_dates[0])
print(f"YTM: {ytm:.2%}, YTC (5Y): {ytc_5y:.2%}, YTW: {ytw:.2%}")
```

**Explainability**: `explain()` output:
```
Callable Bond CORP-CALL-001:
  Coupon: 4.50%
  Maturity: 10 years (2035-01-15)
  
  Call Schedule:
    Lockout: 5 years (no calls before 2030-01-15)
    Call frequency: Quarterly from 2030-01-15 to maturity
    Call price: 100 (par)
    Total call dates: 20
    
  Make-Whole Provision (before lockout):
    Discount rate: Treasury curve + 50bp
    Redemption price: PV of remaining cashflows at Treasury+50bp
    
  Yields (at dirty price 98.5):
    YTM: 4.72%
    YTC (1st call, 5Y): 4.89% ← Worst case
    YTC (10th call, 7.5Y): 4.75%
    YTW: 4.89% (assumes called at first opportunity)
    
  OAS (option-adjusted spread): 85bp
  (Spread over Treasury after stripping out call option value)
```

**Validation**:
- Test YTW: verify `YTW = min(YTM, YTC_1, YTC_2, ...)` across all call dates
- Bloomberg comparison: Match YTW calculation vs. `YA <GO>` (Yield Analysis)
- Property test: Adding call dates → YTW decreases (more optionality hurts bondholder)
- Make-whole pricing: Verify redemption price = PV(remaining CFs, Treasury+spread)

**Impact & Effort**: P1; Small (1-2 weeks)
- **Dependencies**: Bond YTM calculation (exists), OAS tree pricer (exists), schedule generation (exists)
- **Risks**: Complex call decision rules (e.g., call only if rate drops 100bp+)
- **Mitigations**: Start with simple calls (issuer calls optimally); add decision rules in phase 2

**Demo Outline**:
```python
# Notebook: callable_bond_ytw_analysis.ipynb
# 1. Build 10Y corporate bond with quarterly calls starting year 5
# 2. Price at 98.5 (below par) → compute YTM, YTW
# 3. Show call option value: Noncallable spread - Callable spread
# 4. Interest rate scenario: rates drop 100bp → likelihood of call increases
# 5. Compute OAS using Hull-White tree to value embedded option
```

**Why Now**:
- **Market structure**: Most corporate bonds are callable; single-call assumption is limiting
- **Refinancing**: Low-rate environment → issuers calling bonds frequently
- **Portfolio analytics**: Accurate duration/convexity requires modeling all call dates

**De-Dup Evidence**:
- Searched: `"CallSchedule"`, `"make whole"`, `"yield to worst"` in `bond/` → **Partial**
- Single `call_date` field exists in Bond, but NOT call schedules
- YTM calculation exists, but YTW (yield to worst) NOT implemented

---

## Quick Win 5: Rating-Triggered Margin Step-Ups (Term Loan)

**Persona Pain**: Credit analysts modeling leveraged loans cannot capture rating-triggered margin step-ups (e.g., margin increases 50bp if borrower downgraded to B+), limiting accurate cashflow forecasting and pricing.

**User Story**: *As a credit analyst, I need to model a term loan where the margin steps up by 50bp if the borrower is downgraded from BB to B+, with automatic reset if upgraded back, so that I can forecast interest payments under rating migration scenarios and price the loan correctly.*

**Scope (what's new)**:

**Data**:
- Extend `TermLoan` margin step-ups to trigger on rating changes (currently date-based)
- Rating watch: monitor credit rating at each reset date
- Bidirectional: step-up on downgrade, step-down on upgrade

```rust
pub enum MarginStepUpTrigger {
    /// Step-up occurs on a specific date (existing)
    Date { date: Date, delta_bp: i32 },
    
    /// Step-up if rating falls to/below threshold (new)
    RatingDowngrade {
        threshold_rating: CreditRating,  // e.g., B+
        delta_bp: i32,                   // e.g., +50bp
        is_cumulative: bool,             // true = adds to existing margin
    },
    
    /// Step-down if rating improves to/above threshold (new)
    RatingUpgrade {
        threshold_rating: CreditRating,
        delta_bp: i32,  // e.g., -50bp (negative)
        is_cumulative: bool,
    },
}

pub struct CovenantSpec {
    // ... existing fields ...
    pub margin_stepups: Vec<MarginStepUpTrigger>,
    pub rating_provider: Option<RatingProvider>,  // Moody's, S&P, Fitch
}

pub enum RatingProvider {
    Moodys,
    SP,
    Fitch,
    Internal,  // Bank's internal rating
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::term_loan::{TermLoan, MarginStepUpTrigger, CreditRating};

let loan = TermLoan::floating("LOAN-001", notional, "USD-SOFR", 250.0, issue, maturity)
    .add_covenant(CovenantSpec {
        margin_stepups: vec![
            // Date-based step-up (existing functionality)
            MarginStepUpTrigger::Date {
                date: issue.add_years(2)?,
                delta_bp: 25,
            },
            // Rating-triggered step-up (new)
            MarginStepUpTrigger::RatingDowngrade {
                threshold_rating: CreditRating::BP,  // B+ or worse
                delta_bp: 50,
                is_cumulative: true,  // Adds to existing margin
            },
            // Reverse: step-down on upgrade
            MarginStepUpTrigger::RatingUpgrade {
                threshold_rating: CreditRating::BB,  // BB or better
                delta_bp: -50,  // Remove the 50bp penalty
                is_cumulative: true,
            },
        ],
        rating_provider: Some(RatingProvider::SP),
        ..Default::default()
    })
    .build()?;

// Price loan with rating scenario
let market_with_rating = market.insert_rating("BORROWER-A", CreditRating::BP)?;
let pv_downgrade = loan.value(&market_with_rating, as_of)?;

let market_with_upgrade = market.insert_rating("BORROWER-A", CreditRating::BB)?;
let pv_upgrade = loan.value(&market_with_upgrade, as_of)?;
```

*Python*:
```python
from finstack import TermLoan, MarginStepUpTrigger, CreditRating, CovenantSpec

loan = TermLoan.floating(
    id="LOAN-001",
    notional=50_000_000,
    index="USD-SOFR",
    spread_bp=250,
    issue=issue_date,
    maturity=maturity_date,
    covenants=CovenantSpec(
        margin_stepups=[
            # Rating-triggered step-ups
            MarginStepUpTrigger.rating_downgrade(
                threshold=CreditRating.B_PLUS,
                delta_bp=50,
            ),
            MarginStepUpTrigger.rating_upgrade(
                threshold=CreditRating.BB,
                delta_bp=-50,
            ),
        ],
        rating_provider="SP",
    ),
)

# Scenario 1: Borrower downgraded to B+
market_downgrade = market.with_rating("BORROWER-A", CreditRating.B_PLUS)
pv_downgrade = loan.value(market_downgrade, as_of)

# Scenario 2: Borrower stays at BB
market_stable = market.with_rating("BORROWER-A", CreditRating.BB)
pv_stable = loan.value(market_stable, as_of)

print(f"PV (B+ rating): {pv_downgrade} (margin 300bp)")
print(f"PV (BB rating): {pv_stable} (margin 250bp)")
```

**Explainability**: `explain()` output:
```
Term Loan LOAN-001:
  Notional: $50,000,000
  Index: USD-SOFR
  Base margin: 250bp
  
  Margin Step-Up Triggers:
    1. Date-based (2027-01-15): +25bp
    2. Rating downgrade to B+ or worse: +50bp (S&P rating)
    3. Rating upgrade to BB or better: -50bp (reverses step-up)
    
  Current Scenario:
    Borrower rating: B+ (S&P)
    Trigger status: Downgrade trigger ACTIVE (+50bp)
    
  Effective margin: 250bp (base) + 25bp (date) + 50bp (rating) = 325bp
  
  Next coupon payment (2025-04-15):
    SOFR forward: 4.85%
    All-in rate: 4.85% + 3.25% = 8.10%
    Interest payment: $1,012,500
```

**Validation**:
- Test bidirectional step-ups: downgrade → +50bp, upgrade → -50bp
- Verify cumulative logic: multiple triggers stack correctly
- Bloomberg LoanX comparison (if available): Match margin calculation with rating inputs
- Property test: Margin increases monotonically with rating deterioration

**Impact & Effort**: P1; Small (3-5 days)
- **Dependencies**: CreditRating enum (exists in structured_credit), covenant framework (exists)
- **Risks**: Rating lag (rating agencies update quarterly), trigger definitions vary by deal
- **Mitigations**: Document clearly: trigger evaluates at reset dates only; support custom trigger rules

**Demo Outline**:
```python
# Notebook: rating_triggered_loan_pricing.ipynb
# 1. Build leveraged loan with BB rating, margin 250bp
# 2. Add covenant: +50bp if downgraded to B+
# 3. Scenario analysis: (a) BB stable, (b) downgrade to B+, (c) upgrade to BBB
# 4. Show cashflow waterfall: interest payments increase $250K/year post-downgrade
# 5. Price loan under each scenario; compute expected value with rating transition probabilities
```

**Why Now**:
- **Market practice**: Common in leveraged loans, unitranche debt
- **Credit monitoring**: Lenders actively track borrower ratings
- **Pricing**: Rating-triggered margins affect loan valuation significantly

**De-Dup Evidence**:
- Searched: `"RatingDowngrade"`, `"rating trigger"` in `term_loan/` → **Not found**
- Date-based margin step-ups exist in `CovenantSpec`, but NO rating triggers
- `CreditRating` enum exists in `structured_credit/components/types.rs`

---

## Quick Win 6: Equity Forward/Future

**Persona Pain**: Equity traders cannot price equity forwards or futures directly, forcing manual cost-of-carry calculations or treating them as synthetic equity positions.

**User Story**: *As an equity derivatives trader, I need to price equity forwards (long forward = buy stock at future date for fixed price) with correct cost-of-carry (risk-free rate - dividend yield), compute forward price, and calculate DV01/dividend01 sensitivities, so that I can hedge equity exposure and manage basis risk between spot and forward.*

**Scope (what's new)**:

**Data**:
- New instrument: `EquityForward` (spot-settled or physically-settled)
- Forward pricing: `F = S * exp((r - q) * T)`
- Sensitivities: DV01 (rate sensitivity), Dividend01 (yield sensitivity), Delta (spot)

```rust
pub struct EquityForward {
    pub id: InstrumentId,
    pub underlying_ticker: String,  // e.g., "SPX", "AAPL"
    pub quantity: f64,
    pub forward_price: f64,  // Locked-in forward price
    pub maturity: Date,
    pub settlement_type: SettlementType,  // Cash or Physical
    pub discount_curve_id: CurveId,
    pub dividend_yield: Option<f64>,  // If constant yield model
    pub attributes: Attributes,
}

impl EquityForward {
    /// Fair forward price: F = S * exp((r - q) * T)
    pub fn fair_forward_price(
        &self,
        spot: f64,
        risk_free_rate: f64,
        dividend_yield: f64,
        time_to_maturity: f64,
    ) -> f64 {
        spot * ((risk_free_rate - dividend_yield) * time_to_maturity).exp()
    }
    
    /// Mark-to-market value: PV( (F_market - F_contract) * quantity )
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let spot = context.get_price(&self.underlying_ticker)?;
        let disc = context.get_discount_ref(&self.discount_curve_id)?;
        let div_yield = self.dividend_yield
            .or_else(|| context.get_dividend_yield(&self.underlying_ticker))
            .unwrap_or(0.0);
        
        let t = disc.year_fraction_to_date(as_of, self.maturity)?;
        let df = disc.df(t);
        
        // Fair forward price at current market
        let fair_forward = self.fair_forward_price(spot, disc.zero_rate(t)?, div_yield, t);
        
        // MTM: PV of difference between fair forward and contract forward
        let mtm = (fair_forward - self.forward_price) * self.quantity * df;
        
        Ok(Money::new(mtm, context.base_currency()))
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::equity_forward::EquityForward;

// Long SPX forward: buy S&P 500 in 6 months at 4500
let eq_fwd = EquityForward::new("SPX-FWD-001", "SPX", 100.0, 4500.0, maturity_date)
    .discount_curve("USD-OIS")
    .dividend_yield(0.015)  // 1.5% dividend yield
    .settlement_type(SettlementType::Cash)
    .build()?;

let pv = eq_fwd.value(&market, as_of)?;

let metrics = eq_fwd.price_with_metrics(&market, as_of, &[
    MetricId::Delta,       // Spot sensitivity
    MetricId::Dv01,        // Rate sensitivity
    MetricId::Dividend01,  // Dividend yield sensitivity
])?;
```

*Python*:
```python
from finstack import EquityForward, SettlementType

# Long equity forward on AAPL
eq_fwd = EquityForward(
    id="AAPL-FWD-001",
    underlying="AAPL",
    quantity=1000,
    forward_price=180.0,  # Locked-in price
    maturity=maturity_date,
    settlement_type=SettlementType.CASH,
    dividend_yield=0.005,  # 0.5%
)

result = eq_fwd.price_with_metrics(market, as_of, ["pv", "delta", "dv01", "dividend01"])
print(f"PV: {result.pv}")
print(f"Delta: {result.metrics['delta']}")  # ~1000 shares exposure
print(f"DV01: {result.metrics['dv01']}")    # Rate sensitivity
```

**Explainability**: `explain()` output:
```
Equity Forward SPX-FWD-001:
  Underlying: S&P 500 Index (SPX)
  Quantity: 100 contracts
  Contract forward price: 4500.00
  Maturity: 6 months (2025-07-15)
  
  Market Inputs:
    Spot: 4520.00
    Risk-free rate: 4.50%
    Dividend yield: 1.50%
    Time to maturity: 0.5Y
    
  Fair Forward Price:
    F = S * exp((r - q) * T)
      = 4520 * exp((0.045 - 0.015) * 0.5)
      = 4520 * 1.0151
      = 4588.32
      
  Mark-to-Market:
    Fair forward: 4588.32
    Contract forward: 4500.00
    Difference: +88.32 per index point
    PV = 88.32 * 100 * DF(6M) = $8,745
    
  Sensitivities:
    Delta: 100 (equivalent to 100 shares exposure)
    DV01: +$1,200 per bp (rates up → forward price up → gain)
    Dividend01: -$950 per bp (divs up → forward price down → loss)
```

**Validation**:
- Test cost-of-carry: `F = S * exp((r - q) * T)` matches textbook formula
- Bloomberg comparison: `EQF <GO>` for equity forward calculator
- Property test: Forward price > Spot if `r > q`, Forward price < Spot if `q > r`
- Synthetic replication: Long forward = Long call + Short put (put-call parity)

**Impact & Effort**: P2; Small (3 days)
- **Dependencies**: Equity spot pricer (exists), discount curves (exist), dividend yield (exists in MarketContext)
- **Risks**: Discrete dividend handling (vs. continuous yield), corporate actions (stock splits)
- **Mitigations**: Start with continuous yield model; add discrete dividends in phase 2

**Demo Outline**:
```python
# Notebook: equity_forward_pricing.ipynb
# 1. Price SPX 6M forward (spot 4520, contract 4500)
# 2. Compute fair forward using cost-of-carry: F = S * e^((r-q)*T)
# 3. Show MTM: PV of (fair forward - contract forward)
# 4. Scenario: rates rise 50bp → forward price increases → MTM gain
# 5. Hedge: Long forward + short 100 shares → eliminate delta, isolate carry
```

**Why Now**:
- **Institutional use**: Equity forwards common in index rebalancing, structured products
- **Basis trading**: Spot-forward basis arbitrage opportunities
- **Simplicity**: Low-hanging fruit (3 days to implement)

**De-Dup Evidence**:
- Searched: `"EquityForward"`, `"equity forward"`, `"equity future"` → **Not found**
- Equity spot exists, but no forward/future instrument

---

## Quick Win 7: Quanto CMS Options

**Persona Pain**: Structured products traders cannot price quanto CMS options (constant maturity swap rate payoff in foreign currency), common in cross-currency structured notes for APAC investors.

**User Story**: *As a structured products quant, I need to price a quanto CMS option where the payoff is based on USD 10Y swap rate but paid in JPY, with quanto drift adjustment and correlation between USD rates and USDJPY FX, so that I can offer cross-currency yield enhancement products to Japanese investors.*

**Scope (what's new)**:

**Data**:
- Combine `CmsOption` (existing) + `QuantoOption` (existing) logic
- Quanto drift for interest rate underlyings
- Correlation: USD rates ↔ USDJPY FX

```rust
pub struct QuantoCmsOption {
    pub id: InstrumentId,
    pub cms_spec: CmsOptionSpec,  // CMS rate (e.g., USD 10Y swap rate)
    pub quanto_currency: Currency,  // Payment currency (e.g., JPY)
    pub domestic_currency: Currency,  // Rate currency (e.g., USD)
    pub notional_quanto: Money,  // Notional in JPY
    pub strike: f64,  // Strike on swap rate (e.g., 4.50%)
    pub option_type: OptionType,
    pub expiry: Date,
    pub fx_vol: f64,  // USDJPY vol
    pub rate_fx_correlation: f64,  // Correlation(USD rates, USDJPY)
    pub discount_curve_quanto: CurveId,  // JPY discount
    pub discount_curve_domestic: CurveId,  // USD discount
    pub attributes: Attributes,
}

impl QuantoCmsOption {
    /// Price using Hull-White + quanto adjustment
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Step 1: Price CMS option in domestic currency (USD)
        let cms_pv_usd = price_cms_option_hull_white(
            &self.cms_spec,
            self.strike,
            self.option_type,
            context,
            as_of,
        )?;
        
        // Step 2: Apply quanto drift adjustment
        let fx_rate = context.get_fx_rate(self.domestic_currency, self.quanto_currency)?;
        let quanto_adjustment = calculate_quanto_drift(
            self.rate_fx_correlation,
            self.fx_vol,
            time_to_expiry,
        );
        
        // Step 3: Convert to quanto currency
        let pv_quanto = cms_pv_usd * fx_rate * quanto_adjustment;
        
        Ok(Money::new(pv_quanto, self.quanto_currency))
    }
}

fn calculate_quanto_drift(corr: f64, fx_vol: f64, t: f64) -> f64 {
    // Quanto drift: exp(-corr * fx_vol * rate_vol * t)
    // Simplified for CMS (rate vol from swaption surface)
    (1.0 - corr * fx_vol * 0.5 * t)  // Approximation
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::quanto_cms_option::QuantoCmsOption;

let quanto_cms = QuantoCmsOption::new("QUANTO-CMS-001")
    .cms_rate("USD-10Y-SWAP")  // USD 10Y swap rate
    .strike(0.045)  // 4.50%
    .option_type(OptionType::Call)
    .expiry(expiry_date)
    .notional_quanto(Money::new(1_000_000_000.0, Currency::JPY))  // ¥1B notional
    .quanto_currency(Currency::JPY)
    .domestic_currency(Currency::USD)
    .fx_vol(0.10)  // 10% USDJPY vol
    .rate_fx_correlation(0.30)  // 30% correlation
    .discount_curve_quanto("JPY-OIS")
    .discount_curve_domestic("USD-OIS")
    .build()?;

let pv = quanto_cms.value(&market, as_of)?;
```

*Python*:
```python
from finstack import QuantoCmsOption, OptionType, Money

quanto_cms = QuantoCmsOption(
    id="QUANTO-CMS-001",
    cms_rate="USD-10Y-SWAP",
    strike=0.045,
    option_type=OptionType.CALL,
    expiry=expiry_date,
    notional_quanto=Money(1_000_000_000, "JPY"),
    quanto_currency="JPY",
    domestic_currency="USD",
    fx_vol=0.10,
    rate_fx_correlation=0.30,
)

result = quanto_cms.price(market, as_of)
print(f"PV: {result.pv}")  # In JPY
```

**Explainability**: `explain()` output:
```
Quanto CMS Option QUANTO-CMS-001:
  Underlying: USD 10Y swap rate
  Payment currency: JPY
  Notional: ¥1,000,000,000
  
  Option Details:
    Type: Call
    Strike: 4.50%
    Expiry: 1 year
    
  Market Inputs:
    USD 10Y swap rate: 4.20%
    USDJPY spot: 150
    USDJPY vol: 10%
    Rate-FX correlation: 30%
    
  Pricing:
    CMS option value (USD): $12,500
    Quanto adjustment: 0.985 (from correlation)
    FX conversion: × 150
    PV (JPY): ¥1,846,875
    
  Sensitivities:
    USD rate vega: ¥85,000 per 1% vol
    FX vega: ¥42,000 per 1% FX vol
    Correlation01: ¥3,200 per 1% correlation
```

**Validation**:
- Test quanto drift: zero correlation → no quanto adjustment
- Reproduce Bloomberg `OVME <GO>` (option valuation) for quanto CMS
- Property test: Higher rate-FX correlation → larger quanto effect
- Limit case: FX vol → 0 ⇒ reduces to plain CMS option

**Impact & Effort**: P2; Medium (1 week)
- **Dependencies**: CmsOption (exists), QuantoOption (exists), Hull-White model (exists)
- **Risks**: Correlation estimation (unstable), model complexity
- **Mitigations**: Provide correlation term structure if needed; start with constant correlation

**Demo Outline**:
```python
# Notebook: quanto_cms_option_pricing.ipynb
# 1. Build quanto CMS call: USD 10Y swap rate, strike 4.5%, payoff in JPY
# 2. Price with Hull-White + quanto adjustment
# 3. Show quanto drift impact: 30% correlation vs. 0% correlation
# 4. Scenario: USDJPY weakens (150 → 140) → JPY value changes
# 5. Compare: plain CMS (USD payoff) vs. quanto CMS (JPY payoff)
```

**Why Now**:
- **APAC demand**: Japanese/Korean investors seeking USD rate exposure without FX hedge
- **Structured notes**: Common in power reverse dual currency (PRDC) notes
- **Niche but growing**: Increasing issuance post-2020

**De-Dup Evidence**:
- Searched: `"QuantoCms"`, `"quanto CMS"` → **Not found**
- `CmsOption` exists, `QuantoOption` exists, but no combined quanto CMS instrument

---

## Quick Win 8: Dividend Discount Models (DDM) for Equity Valuation

**Persona Pain**: Fundamental equity analysts cannot perform intrinsic value calculations using dividend discount models (Gordon growth, multi-stage DCF), limiting equity valuation to market prices only.

**User Story**: *As a fundamental equity analyst, I need to value a dividend-paying stock using the Gordon growth model (constant perpetual growth) or a 3-stage DDM (high growth → transition → mature), with NPV of future dividends discounted at cost of equity, so that I can compare intrinsic value vs. market price and identify undervalued stocks.*

**Scope (what's new)**:

**Data**:
- New instrument: `EquityDdm` (dividend discount model valuation)
- Models: Gordon growth (single-stage), H-model (2-stage), 3-stage DDM
- Inputs: dividend forecast, growth rates, cost of equity

```rust
pub struct EquityDdm {
    pub id: InstrumentId,
    pub ticker: String,
    pub current_dividend: f64,  // D0 (most recent annual dividend)
    pub model: DdmModel,
    pub cost_of_equity: f64,  // Required return (CAPM or other)
    pub currency: Currency,
    pub attributes: Attributes,
}

pub enum DdmModel {
    /// Gordon Growth: V = D1 / (r - g)
    GordonGrowth { growth_rate: f64 },
    
    /// H-Model (2-stage): High growth transitions linearly to stable
    HModel {
        initial_growth: f64,
        stable_growth: f64,
        transition_years: f64,
    },
    
    /// 3-Stage: High growth → Transition → Mature
    ThreeStage {
        stage1_growth: f64,
        stage1_years: u32,
        stage2_growth: f64,
        stage2_years: u32,
        terminal_growth: f64,
    },
}

impl EquityDdm {
    /// Calculate intrinsic value using DDM
    pub fn intrinsic_value(&self) -> Result<f64> {
        match &self.model {
            DdmModel::GordonGrowth { growth_rate } => {
                // V = D1 / (r - g)
                let d1 = self.current_dividend * (1.0 + growth_rate);
                if self.cost_of_equity <= *growth_rate {
                    return Err(Error::InvalidInput("Cost of equity must exceed growth rate"));
                }
                Ok(d1 / (self.cost_of_equity - growth_rate))
            }
            DdmModel::ThreeStage { stage1_growth, stage1_years, stage2_growth, stage2_years, terminal_growth } => {
                let mut pv = 0.0;
                let mut div = self.current_dividend;
                
                // Stage 1: High growth
                for year in 1..=*stage1_years {
                    div *= 1.0 + stage1_growth;
                    pv += div / (1.0 + self.cost_of_equity).powi(year as i32);
                }
                
                // Stage 2: Transition
                for year in 1..=*stage2_years {
                    div *= 1.0 + stage2_growth;
                    let t = *stage1_years + year;
                    pv += div / (1.0 + self.cost_of_equity).powi(t as i32);
                }
                
                // Stage 3: Terminal value (Gordon growth)
                let terminal_div = div * (1.0 + terminal_growth);
                let terminal_value = terminal_div / (self.cost_of_equity - terminal_growth);
                let t_terminal = stage1_years + stage2_years;
                pv += terminal_value / (1.0 + self.cost_of_equity).powi(t_terminal as i32);
                
                Ok(pv)
            }
            _ => unimplemented!("H-Model pending"),
        }
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::instruments::equity_ddm::{EquityDdm, DdmModel};

// Gordon growth model (simple perpetuity)
let ddm_gordon = EquityDdm::new("AAPL-DDM", "AAPL")
    .current_dividend(0.96)  // $0.96 annual dividend
    .model(DdmModel::GordonGrowth { growth_rate: 0.05 })  // 5% perpetual growth
    .cost_of_equity(0.10)  // 10% required return
    .build()?;

let intrinsic = ddm_gordon.intrinsic_value()?;
println!("Intrinsic value: ${:.2}", intrinsic);  // e.g., $20.16

// 3-stage model (high growth → transition → mature)
let ddm_3stage = EquityDdm::new("TSLA-DDM", "TSLA")
    .current_dividend(0.0)  // Non-dividend paying (use earnings as proxy)
    .model(DdmModel::ThreeStage {
        stage1_growth: 0.20,  // 20% growth for 5 years
        stage1_years: 5,
        stage2_growth: 0.10,  // 10% for 5 years
        stage2_years: 5,
        terminal_growth: 0.03,  // 3% perpetual
    })
    .cost_of_equity(0.12)
    .build()?;

let intrinsic_3stage = ddm_3stage.intrinsic_value()?;
```

*Python*:
```python
from finstack import EquityDdm, DdmModel

# Gordon growth
ddm = EquityDdm(
    ticker="AAPL",
    current_dividend=0.96,
    model=DdmModel.gordon_growth(growth_rate=0.05),
    cost_of_equity=0.10,
)
intrinsic = ddm.intrinsic_value()
print(f"Intrinsic value: ${intrinsic:.2f}")

# Compare with market price
market_price = 175.0
upside = (intrinsic - market_price) / market_price
print(f"Upside/(Downside): {upside:.1%}")

# 3-stage model
ddm_3stage = EquityDdm(
    ticker="GOOGL",
    current_dividend=0.0,  # Use free cash flow as proxy
    model=DdmModel.three_stage(
        stage1_growth=0.15, stage1_years=5,
        stage2_growth=0.08, stage2_years=5,
        terminal_growth=0.03,
    ),
    cost_of_equity=0.11,
)
intrinsic_3stage = ddm_3stage.intrinsic_value()
```

**Explainability**: `explain()` output:
```
Dividend Discount Model (Gordon Growth) for AAPL:
  Current dividend (D0): $0.96
  Growth rate (g): 5.0%
  Cost of equity (r): 10.0%
  
  Valuation:
    Next dividend (D1): $0.96 × 1.05 = $1.008
    Intrinsic value: D1 / (r - g)
                   = $1.008 / (0.10 - 0.05)
                   = $1.008 / 0.05
                   = $20.16
                   
  Comparison:
    Intrinsic value: $20.16
    Market price: $175.00
    Implied market growth: 10.45% (back-solved from DDM)
    
  Sensitivity:
    Growth +1% (5% → 6%): Intrinsic = $25.20 (+25%)
    Cost of equity +1% (10% → 11%): Intrinsic = $16.80 (-17%)
```

**Validation**:
- Test Gordon growth: Verify `V = D1 / (r - g)` matches textbook
- Reproduce CFA Institute DDM examples (Level 1 curriculum)
- Property test: Intrinsic value increases with growth rate, decreases with cost of equity
- 3-stage NPV: Sum of discounted dividends = manual Excel calculation

**Impact & Effort**: P2; Small (1 week)
- **Dependencies**: None (pure DCF math), optional equity price fetching from MarketContext
- **Risks**: Model assumptions (perpetual growth unrealistic), cost of equity estimation
- **Mitigations**: Provide sensitivity analysis; document limitations (garbage in, garbage out)

**Demo Outline**:
```python
# Notebook: equity_ddm_valuation.ipynb
# 1. Fetch AAPL dividend history, compute D0 = $0.96
# 2. Estimate cost of equity via CAPM: r = Rf + β(Rm - Rf)
# 3. Apply Gordon growth: assume g = 5%, compute intrinsic = $20.16
# 4. Compare with market: $175 (implies market expects 10.45% growth!)
# 5. Sensitivity table: intrinsic value vs. (growth, cost of equity)
# 6. 3-stage model for high-growth stock (e.g., NVDA)
```

**Why Now**:
- **Fundamental analysis**: Essential tool for equity analysts, portfolio managers
- **Valuation**: Complements existing spot pricing with intrinsic value
- **Education**: Good demo of financial modeling principles

**De-Dup Evidence**:
- Searched: `"dividend discount"`, `"Gordon growth"`, `"DDM"`, `"intrinsic value"` → **Not found**
- Equity spot pricing exists, but no fundamental valuation models

---

## Quick Win 9: Inflation Seasonality Adjustments

**Persona Pain**: Inflation quants cannot model seasonal CPI patterns (e.g., December CPI often 20bp higher due to holiday spending), leading to mispriced inflation swaps around year-end.

**User Story**: *As an inflation trader, I need to apply seasonal adjustments to inflation forecasts (e.g., +15bp in Dec, -10bp in Jan) based on historical CPI patterns, so that I can accurately price inflation swaps with near-term fixings and avoid systematic mis-pricing around seasonal peaks.*

**Scope (what's new)**:

**Data**:
- Extend `InflationCurve` with seasonal adjustment factors (12 monthly multipliers)
- Historical CPI seasonality from BLS/Eurostat data
- Toggle: seasonal adjustments on/off

```rust
pub struct InflationSeasonality {
    pub monthly_adjustments: [f64; 12],  // Jan=0, Feb=1, ..., Dec=11
    pub is_additive: bool,  // true = add bp, false = multiply ratio
}

impl Default for InflationSeasonality {
    fn default() -> Self {
        // No seasonality (all zeros for additive, all 1.0 for multiplicative)
        Self {
            monthly_adjustments: [0.0; 12],
            is_additive: true,
        }
    }
}

// Extend InflationCurve
pub struct InflationCurve {
    // ... existing fields ...
    pub seasonality: Option<InflationSeasonality>,
}

impl InflationCurve {
    /// Compute inflation index with seasonal adjustment
    pub fn index_at(&self, t: f64) -> Result<f64> {
        let base_index = self.base_index_at(t)?;  // Existing logic
        
        if let Some(seasonal) = &self.seasonality {
            let date = self.base_date + Duration::days((t * 365.25) as i64);
            let month_idx = (date.month() as usize) - 1;  // 0-indexed
            
            let adjustment = seasonal.monthly_adjustments[month_idx];
            if seasonal.is_additive {
                // Additive: index += adjustment (in bp)
                Ok(base_index + adjustment / 10_000.0)
            } else {
                // Multiplicative: index *= (1 + adjustment)
                Ok(base_index * (1.0 + adjustment))
            }
        } else {
            Ok(base_index)
        }
    }
}

// US CPI historical seasonality (BLS 1990-2020 average)
pub fn us_cpi_seasonality() -> InflationSeasonality {
    InflationSeasonality {
        monthly_adjustments: [
            -0.0010,  // Jan: -10bp (post-holiday pullback)
             0.0005,  // Feb: +5bp
             0.0008,  // Mar: +8bp
             0.0003,  // Apr: +3bp
             0.0002,  // May: +2bp
            -0.0005,  // Jun: -5bp
             0.0000,  // Jul: neutral
             0.0003,  // Aug: +3bp
             0.0005,  // Sep: +5bp
             0.0010,  // Oct: +10bp
             0.0008,  // Nov: +8bp
             0.0020,  // Dec: +20bp (holiday spending)
        ],
        is_additive: true,
    }
}
```

**APIs**:

*Rust*:
```rust
use finstack_core::market_data::InflationCurve;
use finstack_valuations::helpers::us_cpi_seasonality;

// Build inflation curve with seasonality
let inflation_curve = InflationCurve::builder("USD-CPI")
    .base_date(as_of)
    .base_index(300.0)  // CPI = 300
    .knots([(0.0, 0.025), (10.0, 0.022)])  // 2.5% → 2.2% over 10Y
    .seasonality(us_cpi_seasonality())
    .build()?;

// Price inflation swap near year-end
let inflation_swap = InflationSwap::new("IIS-001", notional, fixed_rate, start, end)
    .inflation_curve("USD-CPI")
    .build()?;

// Seasonal adjustment automatically applied when pricing
let pv_with_seasonal = inflation_swap.value(&market, as_of)?;

// Compare: disable seasonality
let inflation_curve_flat = inflation_curve.clone().without_seasonality();
let market_no_seasonal = market.insert_inflation_curve(inflation_curve_flat)?;
let pv_no_seasonal = inflation_swap.value(&market_no_seasonal, as_of)?;

println!("PV (with seasonal): {}", pv_with_seasonal);
println!("PV (no seasonal): {}", pv_no_seasonal);
println!("Seasonal impact: ${:.0}", (pv_with_seasonal - pv_no_seasonal).amount());
```

*Python*:
```python
from finstack import InflationCurve, us_cpi_seasonality

# Apply US CPI seasonality
seasonality = us_cpi_seasonality()
inflation_curve = InflationCurve(
    id="USD-CPI",
    base_date=as_of,
    base_index=300.0,
    knots=[(0.0, 0.025), (10.0, 0.022)],
    seasonality=seasonality,
)

# Price inflation swap maturing in December (seasonal peak)
iis = InflationSwap(
    id="IIS-DEC",
    notional=10_000_000,
    fixed_rate=0.025,
    start=date(2025, 1, 1),
    end=date(2025, 12, 15),  # Dec maturity
)
pv = iis.price(market, as_of)
print(f"PV with seasonality: {pv}")

# Sensitivity: seasonal adjustment adds ~$20K value
```

**Explainability**: `explain()` output:
```
Inflation Curve USD-CPI with Seasonality:
  Base CPI: 300.0 (Jan 2025)
  Trend: 2.5% → 2.2% over 10Y
  
  Seasonal Adjustments (US CPI Historical):
    Jan: -10bp   May: +2bp    Sep: +5bp
    Feb: +5bp    Jun: -5bp    Oct: +10bp
    Mar: +8bp    Jul:  0bp    Nov: +8bp
    Apr: +3bp    Aug: +3bp    Dec: +20bp ← Peak
    
  Forward CPI (1Y ahead, Dec 2025):
    Base forecast: 307.5 (2.5% YoY)
    Seasonal adj: +0.20 (20bp for December)
    Total: 307.7
    
  Impact on Inflation Swap (Dec 2025 maturity):
    Fixed leg: $250,000 (2.5% on $10M)
    Inflation leg: $256,700 (actual CPI ratio with seasonal)
    NPV: +$6,700 (favors inflation receiver due to Dec seasonal)
```

**Validation**:
- Test seasonality: Dec index > Jul index (all else equal)
- Compare BLS actual vs. forecast with/without seasonal adjustment
- Property test: Seasonal adjustments sum to ~zero over full year
- Regression test: TIPS pricing with seasonality vs. Bloomberg `ILBE <GO>`

**Impact & Effort**: P1; Small (3 days)
- **Dependencies**: InflationCurve (exists), date utilities (exist)
- **Risks**: Seasonality changes over time, region-specific patterns
- **Mitigations**: Version seasonal factors; support custom user-defined adjustments

**Demo Outline**:
```python
# Notebook: inflation_seasonality_impact.ipynb
# 1. Load historical US CPI data (BLS 1990-2020)
# 2. Compute monthly seasonal factors (deviations from trend)
# 3. Build inflation curve with seasonality
# 4. Price 1Y inflation swap maturing in Dec vs. Jul
# 5. Show: Dec maturity pays ~15bp more due to seasonal peak
# 6. Backtest: historical forecast errors reduced by 40% with seasonality
```

**Why Now**:
- **Accuracy**: Inflation swaps with <1Y maturity mispriced without seasonality
- **Market structure**: Inflation-linked products growing (TIPS, ILBs, I/I swaps)
- **Data availability**: BLS/Eurostat publish seasonal factors freely

**De-Dup Evidence**:
- Searched: `"seasonality"`, `"seasonal adjustment"` in `inflation` → **Not found**
- InflationCurve exists with smooth interpolation, but NO seasonal factors

---

## Quick Win 10: Batch Export of Sensitivities to Arrow/Parquet

**Persona Pain**: Risk teams cannot export bucketed DV01/CS01/vega sensitivities to data lakes (Databricks, Snowflake) without manual CSV → Parquet conversion, slowing down daily risk reporting pipelines.

**User Story**: *As a risk manager, I need to export bucketed sensitivities (DV01 by tenor, vega by expiry/strike) directly to Parquet format with schema (instrument_id, risk_type, bucket, value, currency, as_of_date) for ingestion into our Databricks data lake, so that I can automate daily risk aggregation without manual file conversion.*

**Scope (what's new)**:

**Data**:
- Export `ValuationResult` with bucketed series to Arrow/Parquet
- Schema: instrument_id, metric_id, bucket_label, value, currency, as_of_date
- Batch export: multiple instruments → single Parquet file

```rust
use arrow::array::{StringArray, Float64Array, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

pub fn export_sensitivities_to_parquet(
    results: &[(InstrumentId, ValuationResult)],
    as_of: Date,
    output_path: &Path,
) -> Result<()> {
    // Define Arrow schema
    let schema = Schema::new(vec![
        Field::new("instrument_id", DataType::Utf8, false),
        Field::new("metric_id", DataType::Utf8, false),
        Field::new("bucket_label", DataType::Utf8, true),  // nullable for scalar metrics
        Field::new("value", DataType::Float64, false),
        Field::new("currency", DataType::Utf8, false),
        Field::new("as_of_date", DataType::TimestampMillisecond, false),
    ]);
    
    let mut inst_ids = vec![];
    let mut metric_ids = vec![];
    let mut bucket_labels = vec![];
    let mut values = vec![];
    let mut currencies = vec![];
    let mut as_of_dates = vec![];
    
    let timestamp_ms = as_of.to_unix_timestamp_ms();
    
    for (inst_id, result) in results {
        // Scalar metrics
        for (metric_id, value) in &result.measures {
            inst_ids.push(inst_id.as_str().to_string());
            metric_ids.push(metric_id.to_string());
            bucket_labels.push(None);  // No bucket for scalars
            values.push(*value);
            currencies.push(result.value.currency().to_string());
            as_of_dates.push(timestamp_ms);
        }
        
        // Bucketed series (e.g., bucketed_dv01::1y)
        for (metric_id, series) in &result.bucketed_series {
            for (bucket, value) in series {
                inst_ids.push(inst_id.as_str().to_string());
                metric_ids.push(metric_id.to_string());
                bucket_labels.push(Some(bucket.clone()));
                values.push(*value);
                currencies.push(result.value.currency().to_string());
                as_of_dates.push(timestamp_ms);
            }
        }
    }
    
    // Build Arrow arrays
    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(inst_ids)),
            Arc::new(StringArray::from(metric_ids)),
            Arc::new(StringArray::from(bucket_labels)),
            Arc::new(Float64Array::from(values)),
            Arc::new(StringArray::from(currencies)),
            Arc::new(TimestampMillisecondArray::from(as_of_dates)),
        ],
    )?;
    
    // Write to Parquet
    let file = File::create(output_path)?;
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    
    Ok(())
}
```

**APIs**:

*Rust*:
```rust
use finstack_valuations::export::export_sensitivities_to_parquet;

// Price portfolio with bucketed sensitivities
let metrics = vec![MetricId::BucketedDv01, MetricId::BucketedCs01, MetricId::BucketedVega];
let mut results = vec![];
for inst in portfolio.instruments() {
    let result = inst.price_with_metrics(&market, as_of, &metrics)?;
    results.push((inst.id().clone(), result));
}

// Export to Parquet
export_sensitivities_to_parquet(
    &results,
    as_of,
    Path::new("/data/risk/sensitivities_2025-01-15.parquet"),
)?;
```

*Python*:
```python
from finstack import export_sensitivities_to_parquet
import polars as pl

# Price portfolio
results = []
for inst in portfolio.instruments():
    result = inst.price_with_metrics(market, as_of, ["bucketed_dv01", "bucketed_cs01"])
    results.append((inst.id, result))

# Export to Parquet
export_sensitivities_to_parquet(
    results,
    as_of=as_of,
    output_path="s3://data-lake/risk/sensitivities_2025-01-15.parquet",
)

# Read back in Polars for verification
df = pl.read_parquet("s3://data-lake/risk/sensitivities_2025-01-15.parquet")
print(df.head())
# ┌──────────────┬────────────────┬──────────────┬─────────┬──────────┬─────────────┐
# │ instrument_id│ metric_id      │ bucket_label │ value   │ currency │ as_of_date  │
# ├──────────────┼────────────────┼──────────────┼─────────┼──────────┼─────────────┤
# │ IRS-001      │ bucketed_dv01  │ 1Y           │ -125.3  │ USD      │ 2025-01-15  │
# │ IRS-001      │ bucketed_dv01  │ 2Y           │ -230.8  │ USD      │ 2025-01-15  │
# └──────────────┴────────────────┴──────────────┴─────────┴──────────┴─────────────┘
```

**Explainability**: Log output:
```
Exporting sensitivities to Parquet:
  Instruments: 250
  Metrics per instrument: ~35 (5 scalar + 30 bucketed)
  Total rows: 8,750
  
  Schema:
    - instrument_id: string (e.g., "IRS-001")
    - metric_id: string (e.g., "bucketed_dv01")
    - bucket_label: string (e.g., "1Y", "3M", "5Y")
    - value: float64 (sensitivity value)
    - currency: string (e.g., "USD")
    - as_of_date: timestamp (e.g., 2025-01-15T00:00:00)
    
  Output: /data/risk/sensitivities_2025-01-15.parquet (compressed, 145 KB)
  Compression: Snappy
  Write time: 23ms
```

**Validation**:
- Test schema: Verify all required columns present
- Round-trip: Export → read back in Polars → compare values (should match)
- Integration: Load Parquet in Databricks/Snowflake, query aggregations
- Property test: File size scales linearly with number of instruments

**Impact & Effort**: P1; Small (2 days)
- **Dependencies**: Arrow/Parquet crates (likely already in `io` crate), ValuationResult (exists)
- **Risks**: Large files (>1M rows) may require partitioning
- **Mitigations**: Support chunked writes; add partitioning by as_of_date

**Demo Outline**:
```python
# Notebook: parquet_export_risk_pipeline.ipynb
# 1. Price 100-instrument portfolio with bucketed DV01/CS01
# 2. Export to Parquet: sensitivities_2025-01-15.parquet
# 3. Load in Polars, aggregate by bucket: total_dv01_1y = sum(value where bucket='1Y')
# 4. Upload to S3, query from Databricks SQL
# 5. Show: Daily risk report automation (replaces manual CSV processing)
```

**Why Now**:
- **Data lakes**: Growing adoption of Databricks/Snowflake for risk analytics
- **Performance**: Parquet is 10x faster to read than CSV, 1/3 the size
- **Automation**: Eliminates manual CSV export → conversion pipeline

**De-Dup Evidence**:
- Searched: `"parquet"`, `"arrow export"`, `"to_parquet"` in `valuations/` → **Not found**
- CSV export exists (`dataframe.rs` in attribution), but NO Arrow/Parquet export
- `io` crate has Parquet support for market data, but NOT for sensitivities

---

**End of Fast-Follow Features**

All 10 quick wins are now detailed with the same structure as the major features: persona pain, user story, scope, APIs, explainability, validation, impact/effort, demo outline, why now, and de-dup evidence.

```rust
pub enum VolatilityModel {
    /// Black76 (lognormal): assumes dF = σF dW
    Black76,
    /// Normal (Bachelier): assumes dF = σ dW (absolute vol in bp)
    Normal,
    /// Shifted Lognormal: assumes d(F+shift) = σ(F+shift) dW
    ShiftedLognormal { shift: f64 },
}

pub struct CapFloorPricingConfig {
    pub vol_model: VolatilityModel,
    pub smile_interpolation: SmileInterpolation,  // e.g., SABR, Linear
}

// Extend existing CapFloor instrument
impl RateOptionType {
    pub fn price_with_model(
        &self,
        forward_rate: f64,
        strike: f64,
        vol: f64,
        time_to_expiry: f64,
        discount_factor: f64,
        model: VolatilityModel,
    ) -> Result<f64> {
        match model {
            VolatilityModel::Black76 => {
                // Existing Black76 formula
                black76::caplet_price(...)
            }
            VolatilityModel::Normal => {
                // New: Bachelier formula
                bachelier::caplet_price(forward_rate, strike, vol, time_to_expiry, discount_factor)
            }
            VolatilityModel::ShiftedLognormal { shift } => {
                // New: Black76 with shifted forward
                black76::caplet_price(forward_rate + shift, strike + shift, vol, ...)
            }
        }
    }
}
```

```rust
use finstack_valuations::instruments::cap_floor::RateOptionType;
use finstack_valuations::instruments::common::models::volatility::VolatilityModel;

// Price EUR cap with Normal vol (market convention)
let cap = RateOptionType::Cap
    .with_strikes(vec![0.00, 0.005, 0.01])  // 0%, 0.5%, 1% strikes
    .with_volatility_model(VolatilityModel::Normal)
    .build()?;

let pricing_config = PricingOverrides::default()
    .with_vol_model(VolatilityModel::Normal);

let result = cap.price_with_metrics(&market, as_of, &[MetricId::Vega])?;

// Alternative: Shifted lognormal for negative rates
let config_shifted = PricingOverrides::default()
    .with_vol_model(VolatilityModel::ShiftedLognormal { shift: 0.03 });  // 3% shift
```

```python
from finstack import CapFloor, VolatilityModel

# Normal vol model (Bachelier)
cap = CapFloor(
    id="EUR-CAP-001",
    notional=10_000_000,
    strike=0.005,  # 50bp
    index="EUR-EURIBOR-6M",
    vol_model=VolatilityModel.NORMAL,
)
result = cap.price(market, as_of, vol=0.0050)  # 50bp normal vol

# Shifted lognormal
cap_shifted = cap.with_vol_model(VolatilityModel.SHIFTED_LOGNORMAL, shift=0.03)
result2 = cap_shifted.price(market, as_of, vol=0.20)  # 20% lognormal vol on shifted rate
```

```plaintext
Interest Rate Cap EUR-CAP-001:
  Model: Normal (Bachelier)
  Strike: 0.50%
  Forward rate: 0.35%
  Normal vol: 50bp (0.50%)
  Time to expiry: 2.5Y
  
  Caplet prices (per period):
    2025-06-15: €1,250 (forward 0.30%, strike 0.50%, ITM prob: 15%)
    2025-12-15: €1,580 (forward 0.35%, strike 0.50%, ITM prob: 18%)
    2026-06-15: €1,820 (forward 0.40%, strike 0.50%, ITM prob: 21%)
    
  Total PV: €12,450
  Vega (per 1bp vol): €820
  
  Note: Normal vol model used (market standard for EUR caps in low-rate environment)
```

```python
# Notebook: eur_cap_normal_vol_pricing.ipynb
# 1. Load EUR cap market quotes (Normal vol from broker)
# 2. Price 5Y EUR cap (strike 50bp) with Normal model
# 3. Compare: (a) Normal vol, (b) Black76 with implied lognormal vol
# 4. Show breakdown: Black76 overprices when forward rate near zero
# 5. Compute vega in bp terms (Normal vol sensitivity)
```

```rust
pub struct CreditLinkedNote {
    pub id: InstrumentId,
    pub bond: Bond,  // Underlying bond structure
    pub reference_entity: String,  // CDS reference
    pub credit_curve_id: CurveId,  // Hazard curve for reference
    pub recovery_rate: f64,
    pub notional_at_risk: bool,  // true = principal lost on default
    pub coupon_enhancement_bp: f64,  // additional spread vs. risk-free
    pub attributes: Attributes,
}

impl CreditLinkedNote {
    /// Price CLN as bond discounted with credit-adjusted cashflows
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let hazard = context.get_hazard_ref(&self.credit_curve_id)?;
        let disc = context.get_discount_ref(&self.bond.discount_curve_id)?;
        
        // Generate bond cashflows
        let schedule = self.bond.build_schedule(context, as_of)?;
        
        let mut pv = Money::zero(self.bond.notional.currency());
        for (date, flow) in schedule {
            let t = disc.year_fraction_to_date(as_of, date)?;
            let df = disc.df(t);
            let sp = hazard.sp(t);  // Survival probability to date
            
            // Coupon: paid if survived
            let expected_flow = flow.amount() * sp;
            pv = pv.checked_add(Money::new(expected_flow * df, flow.currency()))?;
        }
        
        // Principal: at risk if notional_at_risk = true
        if self.notional_at_risk {
            let maturity_t = disc.year_fraction_to_date(as_of, self.bond.maturity)?;
            let sp_maturity = hazard.sp(maturity_t);
            let df_maturity = disc.df(maturity_t);
            let expected_principal = self.bond.notional.amount() * sp_maturity * df_maturity;
            
            // Add recovery value on default
            let expected_recovery = self.bond.notional.amount() * (1.0 - sp_maturity) * self.recovery_rate * df_maturity;
            
            pv = pv.checked_add(Money::new(expected_principal + expected_recovery, self.bond.notional.currency()))?;
        }
        
        Ok(pv)
    }
}
```

```rust
use finstack_valuations::instruments::credit_linked_note::CreditLinkedNote;

// Build 5Y CLN linked to CORP-A default
let bond_spec = Bond::fixed(
    "CLN-BOND",
    Money::new(10_000_000.0, Currency::USD),
    0.05,  // 5% coupon
    issue_date,
    maturity_date,
    "USD-OIS",
);

let cln = CreditLinkedNote::new("CLN-001", bond_spec)
    .reference_entity("CORP-A")
    .credit_curve("CORP-A-USD-CDS")
    .recovery_rate(0.40)
    .notional_at_risk(true)  // Principal lost on default
    .coupon_enhancement_bp(200.0)  // 200bp over risk-free for credit risk
    .build()?;

let pv = cln.value(&market, as_of)?;
let metrics = cln.price_with_metrics(&market, as_of, &[
    MetricId::Cs01,
    MetricId::Recovery01,
    MetricId::JumpToDefault,
])?;
```

```python
from finstack import CreditLinkedNote, Bond, Money

bond = Bond.fixed(
    id="CLN-BOND",
    notional=Money(10_000_000, "USD"),
    coupon=0.05,
    issue=issue_date,
    maturity=maturity_date,
)

cln = CreditLinkedNote(
    id="CLN-001",
    bond=bond,
    reference_entity="CORP-A",
    credit_curve="CORP-A-USD-CDS",
    recovery_rate=0.40,
    notional_at_risk=True,
    coupon_enhancement_bp=200,
)

result = cln.price_with_metrics(market, as_of, ["pv", "cs01", "recovery01"])
print(f"CLN PV: {result.pv}")
print(f"CS01: {result.metrics['cs01']}")  # Credit spread sensitivity
```

```plaintext
Credit-Linked Note CLN-001:
  Reference Entity: CORP-A
  Bond Structure:
    Notional: $10,000,000
    Coupon: 5.00% (includes 200bp credit enhancement)
    Maturity: 5 years
    
  Credit Risk:
    Hazard rate: 150bp (BBB-rated)
    5Y survival probability: 92.8%
    Recovery rate: 40%
    
  Cashflow Valuation:
    Expected coupons: $2,320,000 (= $2,500,000 × 92.8%)
    Expected principal: $9,280,000 (survived) + $288,000 (recovery) = $9,568,000
    
  PV: $9,888,000 (98.88% of par)
  
  Sensitivities:
    CS01: -$4,850 per bp (widening hurts)
    Recovery01: +$7,200 per 1% recovery
    Jump-to-default: -$6,000,000 (immediate default scenario)
```

```python
# Notebook: credit_linked_note_structuring.ipynb
# 1. Build vanilla 5Y bond (5% coupon, risk-free)
# 2. Convert to CLN: link principal to CORP-A credit (BBB-rated)
# 3. Calculate required coupon enhancement (200bp) for par pricing
# 4. Compare: (a) CLN PV, (b) Bond PV - CDS PV (should match)
# 5. Scenario: CORP-A downgrade (spread 150bp → 250bp), show CLN mark-down
```

```rust
pub struct RealYieldCurve {
    pub id: CurveId,
    pub nominal_curve_id: CurveId,
    pub inflation_curve_id: CurveId,
    pub base_date: Date,
    pub day_count: DayCount,
}

impl RealYieldCurve {
    /// Compute real discount factor: DF_real(t) = DF_nominal(t) / (1 + inflation(t))
    pub fn real_df(&self, t: f64, context: &MarketContext) -> Result<f64> {
        let nominal_disc = context.get_discount_ref(&self.nominal_curve_id)?;
        let inflation = context.get_inflation_ref(&self.inflation_curve_id)?;
        
        let df_nominal = nominal_disc.df(t);
        let inflation_index = inflation.index_at(t)?;  // CPI ratio
        
        // Real discount factor
        Ok(df_nominal / inflation_index)
    }
    
    /// Compute real zero rate: r_real = (DF_real(0) / DF_real(t))^(1/t) - 1
    pub fn real_zero_rate(&self, t: f64, context: &MarketContext) -> Result<f64> {
        if t <= 0.0 {
            return Ok(0.0);
        }
        let df_real = self.real_df(t, context)?;
        Ok((1.0 / df_real).powf(1.0 / t) - 1.0)
    }
    
    /// Breakeven inflation: (1 + nominal) / (1 + real) - 1
    pub fn breakeven_inflation(&self, t: f64, context: &MarketContext) -> Result<f64> {
        let nominal_disc = context.get_discount_ref(&self.nominal_curve_id)?;
        let df_nominal = nominal_disc.df(t);
        let df_real = self.real_df(t, context)?;
        
        let nominal_zero = (1.0 / df_nominal).powf(1.0 / t) - 1.0;
        let real_zero = (1.0 / df_real).powf(1.0 / t) - 1.0;
        
        // Fisher equation: (1 + nominal) = (1 + real)(1 + inflation)
        Ok((1.0 + nominal_zero) / (1.0 + real_zero) - 1.0)
    }
}
```

```rust
use finstack_core::market_data::RealYieldCurve;

// Build real yield curve from nominal + inflation
let real_curve = RealYieldCurve::new("USD-REAL")
    .nominal_curve("USD-TREASURY")
    .inflation_curve("USD-CPI-SWAP")
    .base_date(as_of)
    .build()?;

// Add to market context
let market = market.insert_real_yield_curve(real_curve)?;

// Extract real rates and breakevens
let tenors = vec![1.0, 2.0, 5.0, 10.0, 30.0];
for t in tenors {
    let real_rate = real_curve.real_zero_rate(t, &market)?;
    let breakeven = real_curve.breakeven_inflation(t, &market)?;
    println!("{:.0}Y: Real={:.2}%, Breakeven={:.2}%", t, real_rate * 100.0, breakeven * 100.0);
}

// Price TIPS directly using real curve
let tips = InflationLinkedBond::new(...)
    .real_curve("USD-REAL")  // Direct pricing from real curve
    .build()?;
```

```python
from finstack import RealYieldCurve

# Derive real curve
real_curve = RealYieldCurve(
    id="USD-REAL",
    nominal_curve="USD-TREASURY",
    inflation_curve="USD-CPI-SWAP",
)
market = market.add_curve(real_curve)

# Extract term structure
tenors = [1, 2, 5, 10, 30]
real_rates = [real_curve.real_zero_rate(t, market) for t in tenors]
breakevens = [real_curve.breakeven_inflation(t, market) for t in tenors]

# DataFrame export
df = pl.DataFrame({
    "tenor": tenors,
    "real_rate": real_rates,
    "breakeven_inflation": breakevens,
})
print(df)

# Price TIPS using real curve
tips = InflationLinkedBond(...)
result = tips.price(market, as_of, real_curve="USD-REAL")
```

```plaintext
Real Yield Curve USD-REAL:
  Derived from:
    Nominal: USD-TREASURY (nominal zero rates)
    Inflation: USD-CPI-SWAP (zero-coupon inflation swaps)
    
  Term Structure (as of 2025-01-15):
    
    Tenor | Nominal | Real | Breakeven
    ------|---------|------|----------
      1Y  |  4.50%  | 1.80%|  2.64%
      2Y  |  4.35%  | 1.65%|  2.66%
      5Y  |  4.20%  | 1.50%|  2.66%
     10Y  |  4.30%  | 1.60%|  2.66%
     30Y  |  4.50%  | 1.75%|  2.70%
     
  Interpretation:
    - 10Y breakeven inflation: 2.66% (market expects 2.66% avg CPI over 10Y)
    - 10Y real rate: 1.60% (inflation-adjusted return on TIPS)
    - Fisher equation: (1.043) ≈ (1.016)(1.0266) ✓
```

```python
# Notebook: real_yield_curve_analysis.ipynb
# 1. Calibrate USD nominal curve from Treasury yields
# 2. Calibrate USD inflation curve from ZC inflation swaps
# 3. Derive real yield curve
# 4. Plot all three: nominal, real, breakeven over 1-30Y
# 5. Compare TIPS pricing: (a) direct from inflation-linked bond pricer, (b) from real curve
# 6. Identify: 10Y TIPS trading rich vs. breakeven (arbitrage opportunity)
```

```rust
pub struct CallSchedule {
    pub lockout_date: Option<Date>,  // No calls before this date
    pub call_dates: Vec<CallEvent>,
    pub make_whole: Option<MakeWholeSpec>,
}

pub struct CallEvent {
    pub call_date: Date,
    pub call_price: f64,  // % of par (e.g., 100.0, 102.5)
    pub is_mandatory: bool,  // true = issuer must call
}

pub struct MakeWholeSpec {
    pub treasury_curve: CurveId,
    pub spread_bp: f64,  // e.g., Treasury + 50bp for PV calc
}

// Extend Bond
pub struct Bond {
    // ... existing fields ...
    pub call_schedule: Option<CallSchedule>,  // Replaces single call_date
}

impl Bond {
    /// Compute yield to worst: min of YTM and yields to all call dates
    pub fn yield_to_worst(&self, market: &MarketContext, as_of: Date, dirty_price: f64) -> Result<f64> {
        let mut ytw = self.yield_to_maturity(dirty_price, as_of)?;
        
        if let Some(schedule) = &self.call_schedule {
            for call_event in &schedule.call_dates {
                if call_event.call_date > as_of {
                    let ytc = self.yield_to_call(dirty_price, as_of, call_event)?;
                    ytw = ytw.min(ytc);
                }
            }
        }
        
        Ok(ytw)
    }
}
```

```rust
use finstack_valuations::instruments::bond::{Bond, CallSchedule, CallEvent, MakeWholeSpec};

// Build 10Y corporate bond, callable quarterly starting year 5
let call_schedule = CallSchedule::new()
    .lockout_date(issue_date.add_years(5)?)
    .add_call_events(
        (issue_date.add_years(5)?..=maturity_date)
            .step_by_months(3)
            .map(|date| CallEvent {
                call_date: date,
                call_price: 100.0,  // Par
                is_mandatory: false,
            })
            .collect()
    )
    .make_whole(MakeWholeSpec {
        treasury_curve: "USD-TREASURY".into(),
        spread_bp: 50.0,  // Treasury + 50bp discount rate
    })
    .build()?;

let bond = Bond::fixed("CORP-CALL-001", notional, 0.045, issue, maturity, "USD-OIS")
    .callable(call_schedule)
    .build()?;

// Compute yield to worst
let dirty_price = 98.5;  // % of par
let ytw = bond.yield_to_worst(&market, as_of, dirty_price)?;
println!("YTW: {:.2}%", ytw * 100.0);

// Compute OAS with embedded optionality
let oas = bond.price_with_metrics(&market, as_of, &[MetricId::Oas])?
    .measures.get(&MetricId::Oas).unwrap();
```

```python
from finstack import Bond, CallSchedule, CallEvent, MakeWholeSpec
from datetime import timedelta

# Build call schedule: callable quarterly from year 5 onward
call_dates = [issue_date + timedelta(days=365*5 + 90*i) for i in range(20)]
call_schedule = CallSchedule(
    lockout_date=issue_date + timedelta(days=365*5),
    call_events=[CallEvent(date=d, call_price=100.0) for d in call_dates],
    make_whole=MakeWholeSpec(treasury_curve="USD-TREASURY", spread_bp=50),
)

bond = Bond.fixed(
    id="CORP-CALL-001",
    notional=10_000_000,
    coupon=0.045,
    issue=issue_date,
    maturity=maturity_date,
    callable=call_schedule,
)

# Compute yield to worst
ytw = bond.yield_to_worst(market, as_of, dirty_price=98.5)
print(f"YTW: {ytw:.2%}")

# All possible yields
ytm = bond.yield_to_maturity(dirty_price=98.5, as_of=as_of)
ytc_5y = bond.yield_to_call(dirty_price=98.5, as_of=as_of, call_date=call_dates[0])
print(f"YTM: {ytm:.2%}, YTC (5Y): {ytc_5y:.2%}, YTW: {ytw:.2%}")
```

```plaintext
Callable Bond CORP-CALL-001:
  Coupon: 4.50%
  Maturity: 10 years (2035-01-15)
  
  Call Schedule:
    Lockout: 5 years (no calls before 2030-01-15)
    Call frequency: Quarterly from 2030-01-15 to maturity
    Call price: 100 (par)
    Total call dates: 20
    
  Make-Whole Provision (before lockout):
    Discount rate: Treasury curve + 50bp
    Redemption price: PV of remaining cashflows at Treasury+50bp
    
  Yields (at dirty price 98.5):
    YTM: 4.72%
    YTC (1st call, 5Y): 4.89% ← Worst case
    YTC (10th call, 7.5Y): 4.75%
    YTW: 4.89% (assumes called at first opportunity)
    
  OAS (option-adjusted spread): 85bp
  (Spread over Treasury after stripping out call option value)
```

```python
# Notebook: callable_bond_ytw_analysis.ipynb
# 1. Build 10Y corporate bond with quarterly calls starting year 5
# 2. Price at 98.5 (below par) → compute YTM, YTW
# 3. Show call option value: Noncallable spread - Callable spread
# 4. Interest rate scenario: rates drop 100bp → likelihood of call increases
# 5. Compute OAS using Hull-White tree to value embedded option
```

```rust
pub enum MarginStepUpTrigger {
    /// Step-up occurs on a specific date (existing)
    Date { date: Date, delta_bp: i32 },
    
    /// Step-up if rating falls to/below threshold (new)
    RatingDowngrade {
        threshold_rating: CreditRating,  // e.g., B+
        delta_bp: i32,                   // e.g., +50bp
        is_cumulative: bool,             // true = adds to existing margin
    },
    
    /// Step-down if rating improves to/above threshold (new)
    RatingUpgrade {
        threshold_rating: CreditRating,
        delta_bp: i32,  // e.g., -50bp (negative)
        is_cumulative: bool,
    },
}

pub struct CovenantSpec {
    // ... existing fields ...
    pub margin_stepups: Vec<MarginStepUpTrigger>,
    pub rating_provider: Option<RatingProvider>,  // Moody's, S&P, Fitch
}

pub enum RatingProvider {
    Moodys,
    SP,
    Fitch,
    Internal,  // Bank's internal rating
}
```

```rust
use finstack_valuations::instruments::term_loan::{TermLoan, MarginStepUpTrigger, CreditRating};

let loan = TermLoan::floating("LOAN-001", notional, "USD-SOFR", 250.0, issue, maturity)
    .add_covenant(CovenantSpec {
        margin_stepups: vec![
            // Date-based step-up (existing functionality)
            MarginStepUpTrigger::Date {
                date: issue.add_years(2)?,
                delta_bp: 25,
            },
            // Rating-triggered step-up (new)
            MarginStepUpTrigger::RatingDowngrade {
                threshold_rating: CreditRating::BP,  // B+ or worse
                delta_bp: 50,
                is_cumulative: true,  // Adds to existing margin
            },
            // Reverse: step-down on upgrade
            MarginStepUpTrigger::RatingUpgrade {
                threshold_rating: CreditRating::BB,  // BB or better
                delta_bp: -50,  // Remove the 50bp penalty
                is_cumulative: true,
            },
        ],
        rating_provider: Some(RatingProvider::SP),
        ..Default::default()
    })
    .build()?;

// Price loan with rating scenario
let market_with_rating = market.insert_rating("BORROWER-A", CreditRating::BP)?;
let pv_downgrade = loan.value(&market_with_rating, as_of)?;

let market_with_upgrade = market.insert_rating("BORROWER-A", CreditRating::BB)?;
let pv_upgrade = loan.value(&market_with_upgrade, as_of)?;
```

```python
from finstack import TermLoan, MarginStepUpTrigger, CreditRating, CovenantSpec

loan = TermLoan.floating(
    id="LOAN-001",
    notional=50_000_000,
    index="USD-SOFR",
    spread_bp=250,
    issue=issue_date,
    maturity=maturity_date,
    covenants=CovenantSpec(
        margin_stepups=[
            # Rating-triggered step-ups
            MarginStepUpTrigger.rating_downgrade(
                threshold=CreditRating.B_PLUS,
                delta_bp=50,
            ),
            MarginStepUpTrigger.rating_upgrade(
                threshold=CreditRating.BB,
                delta_bp=-50,
            ),
        ],
        rating_provider="SP",
    ),
)

# Scenario 1: Borrower downgraded to B+
market_downgrade = market.with_rating("BORROWER-A", CreditRating.B_PLUS)
pv_downgrade = loan.value(market_downgrade, as_of)

# Scenario 2: Borrower stays at BB
market_stable = market.with_rating("BORROWER-A", CreditRating.BB)
pv_stable = loan.value(market_stable, as_of)

print(f"PV (B+ rating): {pv_downgrade} (margin 300bp)")
print(f"PV (BB rating): {pv_stable} (margin 250bp)")
```

```plaintext
Term Loan LOAN-001:
  Notional: $50,000,000
  Index: USD-SOFR
  Base margin: 250bp
  
  Margin Step-Up Triggers:
    1. Date-based (2027-01-15): +25bp
    2. Rating downgrade to B+ or worse: +50bp (S&P rating)
    3. Rating upgrade to BB or better: -50bp (reverses step-up)
    
  Current Scenario:
    Borrower rating: B+ (S&P)
    Trigger status: Downgrade trigger ACTIVE (+50bp)
    
  Effective margin: 250bp (base) + 25bp (date) + 50bp (rating) = 325bp
  
  Next coupon payment (2025-04-15):
    SOFR forward: 4.85%
    All-in rate: 4.85% + 3.25% = 8.10%
    Interest payment: $1,012,500
```

```python
# Notebook: rating_triggered_loan_pricing.ipynb
# 1. Build leveraged loan with BB rating, margin 250bp
# 2. Add covenant: +50bp if downgraded to B+
# 3. Scenario analysis: (a) BB stable, (b) downgrade to B+, (c) upgrade to BBB
# 4. Show cashflow waterfall: interest payments increase $250K/year post-downgrade
# 5. Price loan under each scenario; compute expected value with rating transition probabilities
```

```rust
pub struct EquityForward {
    pub id: InstrumentId,
    pub underlying_ticker: String,  // e.g., "SPX", "AAPL"
    pub quantity: f64,
    pub forward_price: f64,  // Locked-in forward price
    pub maturity: Date,
    pub settlement_type: SettlementType,  // Cash or Physical
    pub discount_curve_id: CurveId,
    pub dividend_yield: Option<f64>,  // If constant yield model
    pub attributes: Attributes,
}

impl EquityForward {
    /// Fair forward price: F = S * exp((r - q) * T)
    pub fn fair_forward_price(
        &self,
        spot: f64,
        risk_free_rate: f64,
        dividend_yield: f64,
        time_to_maturity: f64,
    ) -> f64 {
        spot * ((risk_free_rate - dividend_yield) * time_to_maturity).exp()
    }
    
    /// Mark-to-market value: PV( (F_market - F_contract) * quantity )
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let spot = context.get_price(&self.underlying_ticker)?;
        let disc = context.get_discount_ref(&self.discount_curve_id)?;
        let div_yield = self.dividend_yield
            .or_else(|| context.get_dividend_yield(&self.underlying_ticker))
            .unwrap_or(0.0);
        
        let t = disc.year_fraction_to_date(as_of, self.maturity)?;
        let df = disc.df(t);
        
        // Fair forward price at current market
        let fair_forward = self.fair_forward_price(spot, disc.zero_rate(t)?, div_yield, t);
        
        // MTM: PV of difference between fair forward and contract forward
        let mtm = (fair_forward - self.forward_price) * self.quantity * df;
        
        Ok(Money::new(mtm, context.base_currency()))
    }
}
```

```rust
use finstack_valuations::instruments::equity_forward::EquityForward;

// Long SPX forward: buy S&P 500 in 6 months at 4500
let eq_fwd = EquityForward::new("SPX-FWD-001", "SPX", 100.0, 4500.0, maturity_date)
    .discount_curve("USD-OIS")
    .dividend_yield(0.015)  // 1.5% dividend yield
    .settlement_type(SettlementType::Cash)
    .build()?;

let pv = eq_fwd.value(&market, as_of)?;

let metrics = eq_fwd.price_with_metrics(&market, as_of, &[
    MetricId::Delta,       // Spot sensitivity
    MetricId::Dv01,        // Rate sensitivity
    MetricId::Dividend01,  // Dividend yield sensitivity
])?;
```

```python
from finstack import EquityForward, SettlementType

# Long equity forward on AAPL
eq_fwd = EquityForward(
    id="AAPL-FWD-001",
    underlying="AAPL",
    quantity=1000,
    forward_price=180.0,  # Locked-in price
    maturity=maturity_date,
    settlement_type=SettlementType.CASH,
    dividend_yield=0.005,  # 0.5%
)

result = eq_fwd.price_with_metrics(market, as_of, ["pv", "delta", "dv01", "dividend01"])
print(f"PV: {result.pv}")
print(f"Delta: {result.metrics['delta']}")  # ~1000 shares exposure
print(f"DV01: {result.metrics['dv01']}")    # Rate sensitivity
```

```plaintext
Equity Forward SPX-FWD-001:
  Underlying: S&P 500 Index (SPX)
  Quantity: 100 contracts
  Contract forward price: 4500.00
  Maturity: 6 months (2025-07-15)
  
  Market Inputs:
    Spot: 4520.00
    Risk-free rate: 4.50%
    Dividend yield: 1.50%
    Time to maturity: 0.5Y
    
  Fair Forward Price:
    F = S * exp((r - q) * T)
      = 4520 * exp((0.045 - 0.015) * 0.5)
      = 4520 * 1.0151
      = 4588.32
      
  Mark-to-Market:
    Fair forward: 4588.32
    Contract forward: 4500.00
    Difference: +88.32 per index point
    PV = 88.32 * 100 * DF(6M) = $8,745
    
  Sensitivities:
    Delta: 100 (equivalent to 100 shares exposure)
    DV01: +$1,200 per bp (rates up → forward price up → gain)
    Dividend01: -$950 per bp (divs up → forward price down → loss)
```

```python
# Notebook: equity_forward_pricing.ipynb
# 1. Price SPX 6M forward (spot 4520, contract 4500)
# 2. Compute fair forward using cost-of-carry: F = S * e^((r-q)*T)
# 3. Show MTM: PV of (fair forward - contract forward)
# 4. Scenario: rates rise 50bp → forward price increases → MTM gain
# 5. Hedge: Long forward + short 100 shares → eliminate delta, isolate carry
```

```rust
pub struct QuantoCmsOption {
    pub id: InstrumentId,
    pub cms_spec: CmsOptionSpec,  // CMS rate (e.g., USD 10Y swap rate)
    pub quanto_currency: Currency,  // Payment currency (e.g., JPY)
    pub domestic_currency: Currency,  // Rate currency (e.g., USD)
    pub notional_quanto: Money,  // Notional in JPY
    pub strike: f64,  // Strike on swap rate (e.g., 4.50%)
    pub option_type: OptionType,
    pub expiry: Date,
    pub fx_vol: f64,  // USDJPY vol
    pub rate_fx_correlation: f64,  // Correlation(USD rates, USDJPY)
    pub discount_curve_quanto: CurveId,  // JPY discount
    pub discount_curve_domestic: CurveId,  // USD discount
    pub attributes: Attributes,
}

impl QuantoCmsOption {
    /// Price using Hull-White + quanto adjustment
    pub fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Step 1: Price CMS option in domestic currency (USD)
        let cms_pv_usd = price_cms_option_hull_white(
            &self.cms_spec,
            self.strike,
            self.option_type,
            context,
            as_of,
        )?;
        
        // Step 2: Apply quanto drift adjustment
        let fx_rate = context.get_fx_rate(self.domestic_currency, self.quanto_currency)?;
        let quanto_adjustment = calculate_quanto_drift(
            self.rate_fx_correlation,
            self.fx_vol,
            time_to_expiry,
        );
        
        // Step 3: Convert to quanto currency
        let pv_quanto = cms_pv_usd * fx_rate * quanto_adjustment;
        
        Ok(Money::new(pv_quanto, self.quanto_currency))
    }
}

fn calculate_quanto_drift(corr: f64, fx_vol: f64, t: f64) -> f64 {
    // Quanto drift: exp(-corr * fx_vol * rate_vol * t)
    // Simplified for CMS (rate vol from swaption surface)
    (1.0 - corr * fx_vol * 0.5 * t)  // Approximation
}
```

```rust
use finstack_valuations::instruments::quanto_cms_option::QuantoCmsOption;

let quanto_cms = QuantoCmsOption::new("QUANTO-CMS-001")
    .cms_rate("USD-10Y-SWAP")  // USD 10Y swap rate
    .strike(0.045)  // 4.50%
    .option_type(OptionType::Call)
    .expiry(expiry_date)
    .notional_quanto(Money::new(1_000_000_000.0, Currency::JPY))  // ¥1B notional
    .quanto_currency(Currency::JPY)
    .domestic_currency(Currency::USD)
    .fx_vol(0.10)  // 10% USDJPY vol
    .rate_fx_correlation(0.30)  // 30% correlation
    .discount_curve_quanto("JPY-OIS")
    .discount_curve_domestic("USD-OIS")
    .build()?;

let pv = quanto_cms.value(&market, as_of)?;
```

```python
from finstack import QuantoCmsOption, OptionType, Money

quanto_cms = QuantoCmsOption(
    id="QUANTO-CMS-001",
    cms_rate="USD-10Y-SWAP",
    strike=0.045,
    option_type=OptionType.CALL,
    expiry=expiry_date,
    notional_quanto=Money(1_000_000_000, "JPY"),
    quanto_currency="JPY",
    domestic_currency="USD",
    fx_vol=0.10,
    rate_fx_correlation=0.30,
)

result = quanto_cms.price(market, as_of)
print(f"PV: {result.pv}")  # In JPY
```

```plaintext
Quanto CMS Option QUANTO-CMS-001:
  Underlying: USD 10Y swap rate
  Payment currency: JPY
  Notional: ¥1,000,000,000
  
  Option Details:
    Type: Call
    Strike: 4.50%
    Expiry: 1 year
    
  Market Inputs:
    USD 10Y swap rate: 4.20%
    USDJPY spot: 150
    USDJPY vol: 10%
    Rate-FX correlation: 30%
    
  Pricing:
    CMS option value (USD): $12,500
    Quanto adjustment: 0.985 (from correlation)
    FX conversion: × 150
    PV (JPY): ¥1,846,875
    
  Sensitivities:
    USD rate vega: ¥85,000 per 1% vol
    FX vega: ¥42,000 per 1% FX vol
    Correlation01: ¥3,200 per 1% correlation
```

```python
# Notebook: quanto_cms_option_pricing.ipynb
# 1. Build quanto CMS call: USD 10Y swap rate, strike 4.5%, payoff in JPY
# 2. Price with Hull-White + quanto adjustment
# 3. Show quanto drift impact: 30% correlation vs. 0% correlation
# 4. Scenario: USDJPY weakens (150 → 140) → JPY value changes
# 5. Compare: plain CMS (USD payoff) vs. quanto CMS (JPY payoff)
```

```rust
pub struct EquityDdm {
    pub id: InstrumentId,
    pub ticker: String,
    pub current_dividend: f64,  // D0 (most recent annual dividend)
    pub model: DdmModel,
    pub cost_of_equity: f64,  // Required return (CAPM or other)
    pub currency: Currency,
    pub attributes: Attributes,
}

pub enum DdmModel {
    /// Gordon Growth: V = D1 / (r - g)
    GordonGrowth { growth_rate: f64 },
    
    /// H-Model (2-stage): High growth transitions linearly to stable
    HModel {
        initial_growth: f64,
        stable_growth: f64,
        transition_years: f64,
    },
    
    /// 3-Stage: High growth → Transition → Mature
    ThreeStage {
        stage1_growth: f64,
        stage1_years: u32,
        stage2_growth: f64,
        stage2_years: u32,
        terminal_growth: f64,
    },
}

impl EquityDdm {
    /// Calculate intrinsic value using DDM
    pub fn intrinsic_value(&self) -> Result<f64> {
        match &self.model {
            DdmModel::GordonGrowth { growth_rate } => {
                // V = D1 / (r - g)
                let d1 = self.current_dividend * (1.0 + growth_rate);
                if self.cost_of_equity <= *growth_rate {
                    return Err(Error::InvalidInput("Cost of equity must exceed growth rate"));
                }
                Ok(d1 / (self.cost_of_equity - growth_rate))
            }
            DdmModel::ThreeStage { stage1_growth, stage1_years, stage2_growth, stage2_years, terminal_growth } => {
                let mut pv = 0.0;
                let mut div = self.current_dividend;
                
                // Stage 1: High growth
                for year in 1..=*stage1_years {
                    div *= 1.0 + stage1_growth;
                    pv += div / (1.0 + self.cost_of_equity).powi(year as i32);
                }
                
                // Stage 2: Transition
                for year in 1..=*stage2_years {
                    div *= 1.0 + stage2_growth;
                    let t = *stage1_years + year;
                    pv += div / (1.0 + self.cost_of_equity).powi(t as i32);
                }
                
                // Stage 3: Terminal value (Gordon growth)
                let terminal_div = div * (1.0 + terminal_growth);
                let terminal_value = terminal_div / (self.cost_of_equity - terminal_growth);
                let t_terminal = stage1_years + stage2_years;
                pv += terminal_value / (1.0 + self.cost_of_equity).powi(t_terminal as i32);
                
                Ok(pv)
            }
            _ => unimplemented!("H-Model pending"),
        }
    }
}
```

```rust
use finstack_valuations::instruments::equity_ddm::{EquityDdm, DdmModel};

// Gordon growth model (simple perpetuity)
let ddm_gordon = EquityDdm::new("AAPL-DDM", "AAPL")
    .current_dividend(0.96)  // $0.96 annual dividend
    .model(DdmModel::GordonGrowth { growth_rate: 0.05 })  // 5% perpetual growth
    .cost_of_equity(0.10)  // 10% required return
    .build()?;

let intrinsic = ddm_gordon.intrinsic_value()?;
println!("Intrinsic value: ${:.2}", intrinsic);  // e.g., $20.16

// 3-stage model (high growth → transition → mature)
let ddm_3stage = EquityDdm::new("TSLA-DDM", "TSLA")
    .current_dividend(0.0)  // Non-dividend paying (use earnings as proxy)
    .model(DdmModel::ThreeStage {
        stage1_growth: 0.20,  // 20% growth for 5 years
        stage1_years: 5,
        stage2_growth: 0.10,  // 10% for 5 years
        stage2_years: 5,
        terminal_growth: 0.03,  // 3% perpetual
    })
    .cost_of_equity(0.12)
    .build()?;

let intrinsic_3stage = ddm_3stage.intrinsic_value()?;
```

```python
from finstack import EquityDdm, DdmModel

# Gordon growth
ddm = EquityDdm(
    ticker="AAPL",
    current_dividend=0.96,
    model=DdmModel.gordon_growth(growth_rate=0.05),
    cost_of_equity=0.10,
)
intrinsic = ddm.intrinsic_value()
print(f"Intrinsic value: ${intrinsic:.2f}")

# Compare with market price
market_price = 175.0
upside = (intrinsic - market_price) / market_price
print(f"Upside/(Downside): {upside:.1%}")

# 3-stage model
ddm_3stage = EquityDdm(
    ticker="GOOGL",
    current_dividend=0.0,  # Use free cash flow as proxy
    model=DdmModel.three_stage(
        stage1_growth=0.15, stage1_years=5,
        stage2_growth=0.08, stage2_years=5,
        terminal_growth=0.03,
    ),
    cost_of_equity=0.11,
)
intrinsic_3stage = ddm_3stage.intrinsic_value()
```

```plaintext
Dividend Discount Model (Gordon Growth) for AAPL:
  Current dividend (D0): $0.96
  Growth rate (g): 5.0%
  Cost of equity (r): 10.0%
  
  Valuation:
    Next dividend (D1): $0.96 × 1.05 = $1.008
    Intrinsic value: D1 / (r - g)
                   = $1.008 / (0.10 - 0.05)
                   = $1.008 / 0.05
                   = $20.16
                   
  Comparison:
    Intrinsic value: $20.16
    Market price: $175.00
    Implied market growth: 10.45% (back-solved from DDM)
    
  Sensitivity:
    Growth +1% (5% → 6%): Intrinsic = $25.20 (+25%)
    Cost of equity +1% (10% → 11%): Intrinsic = $16.80 (-17%)
```

```python
# Notebook: equity_ddm_valuation.ipynb
# 1. Fetch AAPL dividend history, compute D0 = $0.96
# 2. Estimate cost of equity via CAPM: r = Rf + β(Rm - Rf)
# 3. Apply Gordon growth: assume g = 5%, compute intrinsic = $20.16
# 4. Compare with market: $175 (implies market expects 10.45% growth!)
# 5. Sensitivity table: intrinsic value vs. (growth, cost of equity)
# 6. 3-stage model for high-growth stock (e.g., NVDA)
```

```rust
pub struct InflationSeasonality {
    pub monthly_adjustments: [f64; 12],  // Jan=0, Feb=1, ..., Dec=11
    pub is_additive: bool,  // true = add bp, false = multiply ratio
}

impl Default for InflationSeasonality {
    fn default() -> Self {
        // No seasonality (all zeros for additive, all 1.0 for multiplicative)
        Self {
            monthly_adjustments: [0.0; 12],
            is_additive: true,
        }
    }
}

// Extend InflationCurve
pub struct InflationCurve {
    // ... existing fields ...
    pub seasonality: Option<InflationSeasonality>,
}

impl InflationCurve {
    /// Compute inflation index with seasonal adjustment
    pub fn index_at(&self, t: f64) -> Result<f64> {
        let base_index = self.base_index_at(t)?;  // Existing logic
        
        if let Some(seasonal) = &self.seasonality {
            let date = self.base_date + Duration::days((t * 365.25) as i64);
            let month_idx = (date.month() as usize) - 1;  // 0-indexed
            
            let adjustment = seasonal.monthly_adjustments[month_idx];
            if seasonal.is_additive {
                // Additive: index += adjustment (in bp)
                Ok(base_index + adjustment / 10_000.0)
            } else {
                // Multiplicative: index *= (1 + adjustment)
                Ok(base_index * (1.0 + adjustment))
            }
        } else {
            Ok(base_index)
        }
    }
}

// US CPI historical seasonality (BLS 1990-2020 average)
pub fn us_cpi_seasonality() -> InflationSeasonality {
    InflationSeasonality {
        monthly_adjustments: [
            -0.0010,  // Jan: -10bp (post-holiday pullback)
             0.0005,  // Feb: +5bp
             0.0008,  // Mar: +8bp
             0.0003,  // Apr: +3bp
             0.0002,  // May: +2bp
            -0.0005,  // Jun: -5bp
             0.0000,  // Jul: neutral
             0.0003,  // Aug: +3bp
             0.0005,  // Sep: +5bp
             0.0010,  // Oct: +10bp
             0.0008,  // Nov: +8bp
             0.0020,  // Dec: +20bp (holiday spending)
        ],
        is_additive: true,
    }
}
```

```rust
use finstack_core::market_data::InflationCurve;
use finstack_valuations::helpers::us_cpi_seasonality;

// Build inflation curve with seasonality
let inflation_curve = InflationCurve::builder("USD-CPI")
    .base_date(as_of)
    .base_index(300.0)  // CPI = 300
    .knots([(0.0, 0.025), (10.0, 0.022)])  // 2.5% → 2.2% over 10Y
    .seasonality(us_cpi_seasonality())
    .build()?;

// Price inflation swap near year-end
let inflation_swap = InflationSwap::new("IIS-001", notional, fixed_rate, start, end)
    .inflation_curve("USD-CPI")
    .build()?;

// Seasonal adjustment automatically applied when pricing
let pv_with_seasonal = inflation_swap.value(&market, as_of)?;

// Compare: disable seasonality
let inflation_curve_flat = inflation_curve.clone().without_seasonality();
let market_no_seasonal = market.insert_inflation_curve(inflation_curve_flat)?;
let pv_no_seasonal = inflation_swap.value(&market_no_seasonal, as_of)?;

println!("PV (with seasonal): {}", pv_with_seasonal);
println!("PV (no seasonal): {}", pv_no_seasonal);
println!("Seasonal impact: ${:.0}", (pv_with_seasonal - pv_no_seasonal).amount());
```

```python
from finstack import InflationCurve, us_cpi_seasonality

# Apply US CPI seasonality
seasonality = us_cpi_seasonality()
inflation_curve = InflationCurve(
    id="USD-CPI",
    base_date=as_of,
    base_index=300.0,
    knots=[(0.0, 0.025), (10.0, 0.022)],
    seasonality=seasonality,
)

# Price inflation swap maturing in December (seasonal peak)
iis = InflationSwap(
    id="IIS-DEC",
    notional=10_000_000,
    fixed_rate=0.025,
    start=date(2025, 1, 1),
    end=date(2025, 12, 15),  # Dec maturity
)
pv = iis.price(market, as_of)
print(f"PV with seasonality: {pv}")

# Sensitivity: seasonal adjustment adds ~$20K value
```

```plaintext
Inflation Curve USD-CPI with Seasonality:
  Base CPI: 300.0 (Jan 2025)
  Trend: 2.5% → 2.2% over 10Y
  
  Seasonal Adjustments (US CPI Historical):
    Jan: -10bp   May: +2bp    Sep: +5bp
    Feb: +5bp    Jun: -5bp    Oct: +10bp
    Mar: +8bp    Jul:  0bp    Nov: +8bp
    Apr: +3bp    Aug: +3bp    Dec: +20bp ← Peak
    
  Forward CPI (1Y ahead, Dec 2025):
    Base forecast: 307.5 (2.5% YoY)
    Seasonal adj: +0.20 (20bp for December)
    Total: 307.7
    
  Impact on Inflation Swap (Dec 2025 maturity):
    Fixed leg: $250,000 (2.5% on $10M)
    Inflation leg: $256,700 (actual CPI ratio with seasonal)
    NPV: +$6,700 (favors inflation receiver due to Dec seasonal)
```

```python
# Notebook: inflation_seasonality_impact.ipynb
# 1. Load historical US CPI data (BLS 1990-2020)
# 2. Compute monthly seasonal factors (deviations from trend)
# 3. Build inflation curve with seasonality
# 4. Price 1Y inflation swap maturing in Dec vs. Jul
# 5. Show: Dec maturity pays ~15bp more due to seasonal peak
# 6. Backtest: historical forecast errors reduced by 40% with seasonality
```

```rust
use arrow::array::{StringArray, Float64Array, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

pub fn export_sensitivities_to_parquet(
    results: &[(InstrumentId, ValuationResult)],
    as_of: Date,
    output_path: &Path,
) -> Result<()> {
    // Define Arrow schema
    let schema = Schema::new(vec![
        Field::new("instrument_id", DataType::Utf8, false),
        Field::new("metric_id", DataType::Utf8, false),
        Field::new("bucket_label", DataType::Utf8, true),  // nullable for scalar metrics
        Field::new("value", DataType::Float64, false),
        Field::new("currency", DataType::Utf8, false),
        Field::new("as_of_date", DataType::TimestampMillisecond, false),
    ]);
    
    let mut inst_ids = vec![];
    let mut metric_ids = vec![];
    let mut bucket_labels = vec![];
    let mut values = vec![];
    let mut currencies = vec![];
    let mut as_of_dates = vec![];
    
    let timestamp_ms = as_of.to_unix_timestamp_ms();
    
    for (inst_id, result) in results {
        // Scalar metrics
        for (metric_id, value) in &result.measures {
            inst_ids.push(inst_id.as_str().to_string());
            metric_ids.push(metric_id.to_string());
            bucket_labels.push(None);  // No bucket for scalars
            values.push(*value);
            currencies.push(result.value.currency().to_string());
            as_of_dates.push(timestamp_ms);
        }
        
        // Bucketed series (e.g., bucketed_dv01::1y)
        for (metric_id, series) in &result.bucketed_series {
            for (bucket, value) in series {
                inst_ids.push(inst_id.as_str().to_string());
                metric_ids.push(metric_id.to_string());
                bucket_labels.push(Some(bucket.clone()));
                values.push(*value);
                currencies.push(result.value.currency().to_string());
                as_of_dates.push(timestamp_ms);
            }
        }
    }
    
    // Build Arrow arrays
    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(inst_ids)),
            Arc::new(StringArray::from(metric_ids)),
            Arc::new(StringArray::from(bucket_labels)),
            Arc::new(Float64Array::from(values)),
            Arc::new(StringArray::from(currencies)),
            Arc::new(TimestampMillisecondArray::from(as_of_dates)),
        ],
    )?;
    
    // Write to Parquet
    let file = File::create(output_path)?;
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    
    Ok(())
}
```

```rust
use finstack_valuations::export::export_sensitivities_to_parquet;

// Price portfolio with bucketed sensitivities
let metrics = vec![MetricId::BucketedDv01, MetricId::BucketedCs01, MetricId::BucketedVega];
let mut results = vec![];
for inst in portfolio.instruments() {
    let result = inst.price_with_metrics(&market, as_of, &metrics)?;
    results.push((inst.id().clone(), result));
}

// Export to Parquet
export_sensitivities_to_parquet(
    &results,
    as_of,
    Path::new("/data/risk/sensitivities_2025-01-15.parquet"),
)?;
```

```python
from finstack import export_sensitivities_to_parquet
import polars as pl

# Price portfolio
results = []
for inst in portfolio.instruments():
    result = inst.price_with_metrics(market, as_of, ["bucketed_dv01", "bucketed_cs01"])
    results.append((inst.id, result))

# Export to Parquet
export_sensitivities_to_parquet(
    results,
    as_of=as_of,
    output_path="s3://data-lake/risk/sensitivities_2025-01-15.parquet",
)

# Read back in Polars for verification
df = pl.read_parquet("s3://data-lake/risk/sensitivities_2025-01-15.parquet")
print(df.head())
# ┌──────────────┬────────────────┬──────────────┬─────────┬──────────┬─────────────┐
# │ instrument_id│ metric_id      │ bucket_label │ value   │ currency │ as_of_date  │
# ├──────────────┼────────────────┼──────────────┼─────────┼──────────┼─────────────┤
# │ IRS-001      │ bucketed_dv01  │ 1Y           │ -125.3  │ USD      │ 2025-01-15  │
# │ IRS-001      │ bucketed_dv01  │ 2Y           │ -230.8  │ USD      │ 2025-01-15  │
# └──────────────┴────────────────┴──────────────┴─────────┴──────────┴─────────────┘
```

```plaintext
Exporting sensitivities to Parquet:
  Instruments: 250
  Metrics per instrument: ~35 (5 scalar + 30 bucketed)
  Total rows: 8,750
  
  Schema:
    - instrument_id: string (e.g., "IRS-001")
    - metric_id: string (e.g., "bucketed_dv01")
    - bucket_label: string (e.g., "1Y", "3M", "5Y")
    - value: float64 (sensitivity value)
    - currency: string (e.g., "USD")
    - as_of_date: timestamp (e.g., 2025-01-15T00:00:00)
    
  Output: /data/risk/sensitivities_2025-01-15.parquet (compressed, 145 KB)
  Compression: Snappy
  Write time: 23ms
```

```python
# Notebook: parquet_export_risk_pipeline.ipynb
# 1. Price 100-instrument portfolio with bucketed DV01/CS01
# 2. Export to Parquet: sensitivities_2025-01-15.parquet
# 3. Load in Polars, aggregate by bucket: total_dv01_1y = sum(value where bucket='1Y')
# 4. Upload to S3, query from Databricks SQL
# 5. Show: Daily risk report automation (replaces manual CSV processing)
```

