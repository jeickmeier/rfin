/**
 * P&L Attribution Example for WASM
 * 
 * Demonstrates how to use the attribution types and methodology selectors.
 * 
 * Note: Full attribution functions require instrument-specific implementations.
 * For complete attribution workflows, use the Python or Rust APIs which support
 * generic instrument types.
 */

import * as finstack from '../pkg/finstack_wasm';

/**
 * Example 1: Attribution Method Creation
 */
function exampleAttributionMethods() {
  console.log('='.repeat(70));
  console.log('Example 1: Attribution Methods');
  console.log('='.repeat(70));
  
  // Parallel attribution (default)
  const parallel = new finstack.AttributionMethod();
  console.log(`Parallel method: ${parallel.toString()}`);
  
  // Waterfall attribution with custom order
  const waterfall = finstack.AttributionMethod.waterfall([
    "carry",
    "rates_curves",
    "credit_curves",
    "fx",
    "volatility"
  ]);
  console.log(`Waterfall method: ${waterfall.toString()}`);
  
  // Metrics-based attribution
  const metricsBased = finstack.AttributionMethod.metricsBased();
  console.log(`Metrics-based method: ${metricsBased.toString()}`);
}

/**
 * Example 2: Working with Attribution Results
 * 
 * In practice, PnlAttribution would be returned from an attribution function.
 * Here we demonstrate how to access the data once you have it.
 */
function exampleAttributionResults() {
  console.log('\n' + '='.repeat(70));
  console.log('Example 2: Attribution Results Structure');
  console.log('='.repeat(70));
  
  console.log(`
Attribution results provide:

1. 10 P&L Factors:
   - totalPnl       : Total P&L (val_t1 - val_t0)
   - carry          : Time decay (theta + accruals)
   - ratesCurvesPnl : Discount & forward curve shifts
   - creditCurvesPnl: Hazard curve shifts
   - inflationCurvesPnl: Inflation curve shifts
   - correlationsPnl: Base correlation changes
   - fxPnl          : FX rate changes
   - volPnl         : Implied volatility changes
   - modelParamsPnl : Model parameter changes
   - marketScalarsPnl: Market scalar changes
   - residual       : Unexplained P&L

2. Metadata (via .meta property):
   - instrumentId   : Instrument being attributed
   - numRepricings  : Number of repricings performed
   - residualPct    : Residual as % of total
   - tolerance      : Tolerance for validation
   - method         : Attribution methodology used
   - t0, t1         : Date range

3. Detail Structures (optional):
   - ratesDetail       : Per-curve breakdown for rates
   - modelParamsDetail : Prepayment, default, recovery details

4. Export Methods:
   - toCsv()            : Export as CSV string
   - toJson()           : Export as JSON string
   - explain()          : Structured tree output
   - residualWithinTolerance(pct, abs): Validate residual
  `);
}

/**
 * Example 3: Hypothetical Usage (Structure Demonstration)
 */
function exampleUsagePattern() {
  console.log('\n' + '='.repeat(70));
  console.log('Example 3: Usage Pattern (Conceptual)');
  console.log('='.repeat(70));
  
  console.log(`
Conceptual TypeScript/JavaScript Usage:

\`\`\`typescript
import * as finstack from './finstack_wasm';

// In a full implementation, you would:

// 1. Create instrument
const bond = finstack.Bond.fixedSemiannual(
  "CORP-001",
  notional,
  0.05,
  issueDate,
  maturityDate,
  "USD-OIS"
);

// 2. Create markets at T₀ and T₁
const marketT0 = createMarketWithCurve(date_t0, rate: 0.04);
const marketT1 = createMarketWithCurve(date_t1, rate: 0.045);

// 3. Run attribution (hypothetical - requires implementation)
const attr = attributePnl(
  bond,
  marketT0,
  marketT1,
  "2025-01-15",
  "2025-01-16",
  finstack.AttributionMethod.parallel()
);

// 4. Access results
console.log(\`Total P&L: \${attr.totalPnl}\`);
console.log(\`Carry: \${attr.carry}\`);
console.log(\`Rates: \${attr.ratesCurvesPnl}\`);
console.log(\`Residual: \${attr.residual} (\${attr.meta.residualPct}%)\`);

// 5. Check validation
if (attr.residualWithinTolerance(0.1, 100.0)) {
  console.log("✓ Attribution validated");
}

// 6. Export
const csv = attr.toCsv();
const json = attr.toJson();
const tree = attr.explain();

// 7. Access details
if (attr.ratesDetail) {
  const curves = JSON.parse(attr.ratesDetail.byCurveToJson());
  console.log("Discount total:", attr.ratesDetail.discountTotal);
}

if (attr.modelParamsDetail) {
  if (attr.modelParamsDetail.prepayment) {
    console.log("Prepayment P&L:", attr.modelParamsDetail.prepayment);
  }
}
\`\`\`

For Production Use:
- Use Python/Rust bindings for full attribution workflows
- WASM provides types and data structures for result display
- Server-side attribution → client-side visualization
  `);
}

/**
 * Example 4: Portfolio Attribution
 */
function examplePortfolioAttribution() {
  console.log('\n' + '='.repeat(70));
  console.log('Example 4: Portfolio Attribution Structure');
  console.log('='.repeat(70));
  
  console.log(`
Portfolio Attribution provides:

\`\`\`typescript
const portfolioAttr: PortfolioAttribution = ...;

// Aggregate factors (same 10 as instrument-level)
console.log(portfolioAttr.totalPnl);
console.log(portfolioAttr.carry);
console.log(portfolioAttr.ratesCurvesPnl);
// ... all 10 factors

// Position breakdown
const positions = JSON.parse(portfolioAttr.byPositionToJson());
for (const [positionId, pnl] of Object.entries(positions)) {
  console.log(\`\${positionId}: \${pnl}\`);
}

// Export
const summary = portfolioAttr.toCsv();
const detail = portfolioAttr.positionDetailToCsv();
const tree = portfolioAttr.explain();
\`\`\`
  `);
}

/**
 * Main function
 */
function main() {
  console.log('');
  console.log('█'.repeat(70));
  console.log('  '.repeat(10) + 'FINSTACK WASM - P&L ATTRIBUTION');
  console.log('█'.repeat(70));
  
  exampleAttributionMethods();
  exampleAttributionResults();
  exampleUsagePattern();
  examplePortfolioAttribution();
  
  console.log('\n' + '='.repeat(70));
  console.log('Attribution types ready for use!');
  console.log('For full attribution workflows, use Python or Rust bindings.');
  console.log('='.repeat(70));
  console.log('');
}

// Run if executed directly
if (require.main === module) {
  main();
}

export { main };

