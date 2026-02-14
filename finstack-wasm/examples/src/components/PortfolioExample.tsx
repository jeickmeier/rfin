import React, { useState } from 'react';
import * as finstack from 'finstack-wasm';
import { PortfolioExampleProps, DEFAULT_PORTFOLIO_PROPS } from './data/portfolio';

type RequiredPortfolioExampleProps = Required<PortfolioExampleProps>;

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
export const PortfolioExample: React.FC<PortfolioExampleProps> = (props) => {
  const defaults = DEFAULT_PORTFOLIO_PROPS as RequiredPortfolioExampleProps;
  const {
    valuationDate = defaults.valuationDate,
    entities = defaults.entities,
    bonds = defaults.bonds,
    deposits = defaults.deposits,
    positions = defaults.positions,
    portfolio = defaults.portfolio,
    discountCurve = defaults.discountCurve,
  } = props;

  const [output, setOutput] = useState<string>('Click "Run Example" to execute');

  const runExample = () => {
    try {
      let log = '';
      const addLog = (msg: string) => {
        log += `${msg}\n`;
        console.log(msg);
      };

      addLog('='.repeat(80));
      addLog('Portfolio Example: Creating and Valuing a Multi-Asset Portfolio');
      addLog('='.repeat(80));

      // 1. Create entities from props
      addLog('\n1. Creating Entities');
      const entityMap = new Map<string, finstack.JsEntity>();
      for (const entityData of entities) {
        let entity = new finstack.JsEntity(entityData.id).withName(entityData.name);
        for (const [key, value] of Object.entries(entityData.tags)) {
          entity = entity.withTag(key, value);
        }
        entityMap.set(entityData.id, entity);
        addLog(`  Created entity: ${entityData.id} - ${entityData.name}`);
      }

      // 2. Create instruments from props
      addLog('\n2. Creating Instruments');
      const asOf = new finstack.FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
      const usd = new finstack.Currency(portfolio.baseCurrency);

      // Create bonds
      const bondMap = new Map<string, finstack.Bond>();
      for (const bondData of bonds) {
        const bond = new finstack.Bond(
          bondData.id,
          new finstack.Money(
            bondData.notional.amount,
            new finstack.Currency(bondData.notional.currency)
          ),
          new finstack.FsDate(
            bondData.issueDate.year,
            bondData.issueDate.month,
            bondData.issueDate.day
          ),
          new finstack.FsDate(
            bondData.maturityDate.year,
            bondData.maturityDate.month,
            bondData.maturityDate.day
          ),
          bondData.discountCurveId,
          bondData.couponRate,
          finstack.Frequency.semiAnnual(),
          finstack.DayCount.thirty360(),
          finstack.BusinessDayConvention.ModifiedFollowing,
          undefined,
          finstack.StubKind.none(),
          undefined,
          undefined,
          undefined,
          undefined
        );
        bondMap.set(bondData.id, bond);
        addLog(`  Created bond: ${bondData.id}`);
      }

      // Create deposits
      const depositMap = new Map<string, finstack.Deposit>();
      for (const depositData of deposits) {
        const deposit = new finstack.Deposit(
          depositData.id,
          new finstack.Money(
            depositData.notional.amount,
            new finstack.Currency(depositData.notional.currency)
          ),
          new finstack.FsDate(
            depositData.startDate.year,
            depositData.startDate.month,
            depositData.startDate.day
          ),
          new finstack.FsDate(
            depositData.maturity.year,
            depositData.maturity.month,
            depositData.maturity.day
          ),
          finstack.DayCount.act360(),
          depositData.discountCurveId,
          depositData.quoteRate
        );
        depositMap.set(depositData.id, deposit);
        addLog(`  Created deposit: ${depositData.id}`);
      }

      // 3. Create positions from props
      addLog('\n3. Creating Positions');
      const positionList: finstack.JsPosition[] = [];

      for (const posData of positions) {
        let position: finstack.JsPosition;

        if (posData.instrumentType === 'bond') {
          const bond = bondMap.get(posData.instrumentRef);
          if (!bond) {
            throw new Error(`Bond ${posData.instrumentRef} not found`);
          }
          position = finstack.createPositionFromBond(
            posData.positionId,
            posData.entityId,
            bond,
            posData.quantity,
            posData.unit === 'units'
              ? finstack.JsPositionUnit.UNITS
              : finstack.JsPositionUnit.notional()
          );
        } else {
          const deposit = depositMap.get(posData.instrumentRef);
          if (!deposit) {
            throw new Error(`Deposit ${posData.instrumentRef} not found`);
          }
          position = finstack.createPositionFromDeposit(
            posData.positionId,
            posData.entityId,
            deposit,
            posData.quantity,
            posData.unit === 'units'
              ? finstack.JsPositionUnit.UNITS
              : finstack.JsPositionUnit.notional()
          );
        }

        positionList.push(position);
        addLog(`  Created position: ${posData.positionId} (${posData.instrumentRef})`);
      }

      // 4. Build market data from props
      addLog('\n4. Building Market Data');
      const market = new finstack.MarketContext();

      const curveBaseDate = new finstack.FsDate(
        discountCurve.baseDate.year,
        discountCurve.baseDate.month,
        discountCurve.baseDate.day
      );
      const discCurve = new finstack.DiscountCurve(
        discountCurve.id,
        curveBaseDate,
        new Float64Array(discountCurve.tenors),
        new Float64Array(discountCurve.discountFactors),
        discountCurve.dayCount,
        discountCurve.interpolation,
        discountCurve.extrapolation,
        discountCurve.continuous
      );
      market.insertDiscount(discCurve);

      addLog(`Created market data with ${discountCurve.id} discount curve`);

      // 5. Build portfolio with positions
      addLog('\n5. Building Portfolio');
      let portfolioBuilder = new finstack.JsPortfolioBuilder(portfolio.id)
        .name(portfolio.name)
        .baseCcy(usd)
        .asOf(asOf);

      // Add entities
      for (const entity of entityMap.values()) {
        portfolioBuilder = portfolioBuilder.entity(entity);
      }

      // Add positions
      for (const position of positionList) {
        portfolioBuilder = portfolioBuilder.position(position);
      }

      // Add tags
      for (const [key, value] of Object.entries(portfolio.tags)) {
        portfolioBuilder = portfolioBuilder.tag(key, value);
      }

      const builtPortfolio = portfolioBuilder.build();

      addLog(`  Portfolio built successfully:`);
      addLog(`    ID: ${builtPortfolio.id}`);
      addLog(`    Name: ${builtPortfolio.name}`);
      addLog(`    Base Currency: ${builtPortfolio.baseCcy.code}`);
      addLog(`    Positions: ${positions.length}`);

      // Validate portfolio
      builtPortfolio.validate();
      addLog('    ✓ Portfolio validation passed');

      // 6. Value the portfolio
      addLog('\n6. Valuing Portfolio');
      const config = new finstack.FinstackConfig();
      const valuation = finstack.valuePortfolio(builtPortfolio, market, config);

      const totalValue = valuation.totalBaseCcy.amount;
      addLog(
        `  Total Portfolio Value: ${totalValue.toFixed(2)} ${valuation.totalBaseCcy.currency.code}`
      );

      // Show position-level values
      addLog('\n  Position-Level Values:');
      for (const posData of positions) {
        const posValue = valuation.getPositionValue(posData.positionId);
        if (posValue) {
          addLog(`    ${posValue.positionId}:`);
          addLog(`      Entity: ${posValue.entityId}`);
          addLog(
            `      Native: ${posValue.valueNative.amount.toFixed(2)} ${posValue.valueNative.currency.code}`
          );
          addLog(
            `      Base:   ${posValue.valueBase.amount.toFixed(2)} ${posValue.valueBase.currency.code}`
          );
        }
      }

      // Show entity-level aggregation
      addLog('\n  Entity-Level Aggregation:');
      for (const entityData of entities) {
        const entityValue = valuation.getEntityValue(entityData.id);
        if (entityValue) {
          addLog(
            `    ${entityData.name}: ${entityValue.amount.toFixed(2)} ${entityValue.currency.code}`
          );
        }
      }

      // 7. Aggregate metrics
      addLog('\n7. Aggregating Metrics');
      const metrics = finstack.aggregateMetrics(valuation);

      const dv01 = metrics.getTotal('dv01');
      const cs01 = metrics.getTotal('cs01');
      const theta = metrics.getTotal('theta');

      if (dv01 !== undefined && dv01 !== null) {
        addLog(`  Portfolio DV01: ${dv01.toFixed(4)}`);
      }
      if (cs01 !== undefined && cs01 !== null) {
        addLog(`  Portfolio CS01: ${cs01.toFixed(4)}`);
      }
      if (theta !== undefined && theta !== null) {
        addLog(`  Portfolio Theta: ${theta.toFixed(4)}`);
      }

      addLog(`\n${'='.repeat(80)}`);
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
        entity management, position creation from instruments, portfolio construction, valuation
        with market data, and metrics aggregation.
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
