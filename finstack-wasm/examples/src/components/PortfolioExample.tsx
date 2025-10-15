import React, { useState } from 'react';
import * as finstack from 'finstack-wasm';

/**
 * Portfolio Example Component
 * 
 * Demonstrates the portfolio WASM bindings including:
 * - Creating entities and positions
 * - Building portfolios with the fluent builder API
 * - Valuing portfolios with market data
 * - Aggregating metrics across positions
 * - Grouping by attributes
 */
export const PortfolioExample: React.FC = () => {
  const [output, setOutput] = useState<string>('Click "Run Example" to execute');

  const runExample = () => {
    try {
      let log = '';
      const addLog = (msg: string) => {
        log += msg + '\n';
        console.log(msg);
      };

      addLog('='.repeat(80));
      addLog('Portfolio Example: Creating and Valuing a Multi-Asset Portfolio');
      addLog('='.repeat(80));

      // 1. Create entities
      addLog('\n1. Creating Entities');
      const entityCorp = new finstack.JsEntity("CORP_A")
        .withName("Corporate A")
        .withTag("sector", "Finance");
      
      const entityFund = new finstack.JsEntity("FUND_B")
        .withName("Fund B")
        .withTag("sector", "Technology");

      addLog(`  Created entity: ${entityCorp.id} - ${entityCorp.name}`);
      addLog(`  Created entity: ${entityFund.id} - ${entityFund.name}`);

      // 2. Create instruments
      addLog('\n2. Creating Instruments');
      const asOf = new finstack.FsDate(2024, 1, 2);
      const usd = new finstack.Currency("USD");

      // Corporate bond
      const bond = finstack.Bond.fixedSemiannual(
        "BOND_CORP_A",
        new finstack.Money(5_000_000, usd),
        0.045,  // 4.5% coupon
        new finstack.FsDate(2024, 1, 15),
        new finstack.FsDate(2029, 1, 15),
        "USD-OIS"
      );

      // Money market deposit
      const deposit = new finstack.Deposit(
        "DEPOSIT_MM",
        new finstack.Money(2_000_000, usd),
        asOf,
        new finstack.FsDate(2024, 7, 2),
        finstack.DayCount.act360(),
        "USD-OIS",
        0.0525  // quote rate
      );

      addLog(`  Created bond: ${bond.instrumentId}`);
      addLog(`  Created deposit: ${deposit.instrumentId}`);

      // 3. Create positions
      addLog('\n3. Creating Positions');
      
      const posBond = finstack.createPositionFromBond(
        "POS_BOND_001",
        entityCorp.id,
        bond,
        1.0,  // quantity
        finstack.JsPositionUnit.UNITS
      );
      
      const posDeposit = finstack.createPositionFromDeposit(
        "POS_DEP_001",
        entityFund.id,
        deposit,
        1.0,  // quantity
        finstack.JsPositionUnit.UNITS
      );
      
      addLog(`  Created position: ${posBond.positionId} (${posBond.instrumentId})`);
      addLog(`  Created position: ${posDeposit.positionId} (${posDeposit.instrumentId})`);

      // 4. Build market data
      addLog('\n4. Building Market Data');
      const market = new finstack.MarketContext();
      
      // Add discount curve
      const discCurve = new finstack.DiscountCurve(
        "USD-OIS",
        asOf,
        new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0, 10.0]),  // times in years
        new Float64Array([1.0, 0.9975, 0.9950, 0.9750, 0.9500, 0.9000]),  // discount factors
        "act_365f",  // day count
        "linear",  // interpolation
        "flat_forward",  // extrapolation
        true  // require monotonic
      );
      market.insertDiscount(discCurve);

      addLog('Created market data with USD discount curve');

      // 5. Build portfolio with positions
      addLog('\n5. Building Portfolio');
      const portfolio = new finstack.JsPortfolioBuilder("MULTI_ASSET_FUND")
        .name("Multi-Asset Investment Fund")
        .baseCcy(usd)
        .asOf(asOf)
        .entity(entityCorp)
        .entity(entityFund)
        .position(posBond)
        .position(posDeposit)
        .tag("strategy", "balanced")
        .tag("risk_profile", "moderate")
        .build();

      addLog(`  Portfolio built successfully:`);
      addLog(`    ID: ${portfolio.id}`);
      addLog(`    Name: ${portfolio.name}`);
      addLog(`    Base Currency: ${portfolio.baseCcy.code}`);
      addLog(`    Positions: 2`);

      // Validate portfolio
      portfolio.validate();
      addLog('    ✓ Portfolio validation passed');

      // 6. Value the portfolio
      addLog('\n6. Valuing Portfolio');
      const config = new finstack.FinstackConfig();
      const valuation = finstack.valuePortfolio(portfolio, market, config);
      
      const totalValue = valuation.totalBaseCcy.amount;
      addLog(`  Total Portfolio Value: ${totalValue.toFixed(2)} ${valuation.totalBaseCcy.currency.code}`);
      
      // Show position-level values
      addLog('\n  Position-Level Values:');
      const bondValue = valuation.getPositionValue("POS_BOND_001");
      if (bondValue) {
        addLog(`    ${bondValue.positionId}:`);
        addLog(`      Entity: ${bondValue.entityId}`);
        addLog(`      Native: ${bondValue.valueNative.amount.toFixed(2)} ${bondValue.valueNative.currency.code}`);
        addLog(`      Base:   ${bondValue.valueBase.amount.toFixed(2)} ${bondValue.valueBase.currency.code}`);
      }
      
      const depositValue = valuation.getPositionValue("POS_DEP_001");
      if (depositValue) {
        addLog(`    ${depositValue.positionId}:`);
        addLog(`      Entity: ${depositValue.entityId}`);
        addLog(`      Native: ${depositValue.valueNative.amount.toFixed(2)} ${depositValue.valueNative.currency.code}`);
        addLog(`      Base:   ${depositValue.valueBase.amount.toFixed(2)} ${depositValue.valueBase.currency.code}`);
      }
      
      // Show entity-level aggregation
      addLog('\n  Entity-Level Aggregation:');
      const corpValue = valuation.getEntityValue(entityCorp.id);
      if (corpValue) {
        addLog(`    ${entityCorp.name}: ${corpValue.amount.toFixed(2)} ${corpValue.currency.code}`);
      }
      const fundValue = valuation.getEntityValue(entityFund.id);
      if (fundValue) {
        addLog(`    ${entityFund.name}: ${fundValue.amount.toFixed(2)} ${fundValue.currency.code}`);
      }

      // 7. Aggregate metrics
      addLog('\n7. Aggregating Metrics');
      const metrics = finstack.aggregateMetrics(valuation);
      
      const dv01 = metrics.getTotal("dv01");
      const cs01 = metrics.getTotal("cs01");
      const theta = metrics.getTotal("theta");
      
      if (dv01 !== undefined && dv01 !== null) {
        addLog(`  Portfolio DV01: ${dv01.toFixed(4)}`);
      }
      if (cs01 !== undefined && cs01 !== null) {
        addLog(`  Portfolio CS01: ${cs01.toFixed(4)}`);
      }
      if (theta !== undefined && theta !== null) {
        addLog(`  Portfolio Theta: ${theta.toFixed(4)}`);
      }

      addLog('\n' + '='.repeat(80));
      addLog('Example completed successfully!');
      addLog('✓ Full parity achieved: Entity → Instrument → Position → Portfolio → Valuation');
      addLog('='.repeat(80));

      console.log('About to setOutput with', log.length, 'characters');
      setOutput(log);
      console.log('setOutput called');
    } catch (error) {
      const errorMsg = `Error: ${error instanceof Error ? error.message : String(error)}`;
      console.error(errorMsg);
      setOutput(errorMsg);
    }
  };

  return (
    <div style={{ padding: '20px' }}>
      <h2>Portfolio Management Example</h2>
      <p>
        This example demonstrates the complete portfolio WASM bindings with full parity to Rust:
        entity management, position creation from instruments, portfolio construction,
        valuation with market data, and metrics aggregation.
      </p>
      <button
        onClick={runExample}
        style={{
          padding: '10px 20px',
          fontSize: '16px',
          cursor: 'pointer',
          marginBottom: '20px',
        }}
      >
        Run Example
      </button>
      <pre
        style={{
          background: '#f5f5f5',
          color: '#333',
          padding: '15px',
          borderRadius: '5px',
          overflowX: 'auto',
          maxHeight: '600px',
          overflowY: 'auto',
          fontFamily: 'monospace',
          fontSize: '14px',
          lineHeight: '1.5',
        }}
      >
        {output}
      </pre>
    </div>
  );
};

export default PortfolioExample;

