import React, { useEffect, useState } from 'react';
import {
  Date as FsDate,
  DayCount,
  Deposit,
  DiscountCurve,
  MarketContext,
  Money,
  createStandardRegistry,
} from 'finstack-wasm';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

const percentFormatter = new Intl.NumberFormat('en-US', {
  style: 'percent',
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const bpsFormatter = new Intl.NumberFormat('en-US', {
  minimumFractionDigits: 0,
  maximumFractionDigits: 0,
});

type DepositMetrics = {
  presentValue: number;
  quoteRate: number;
  cleanPv: number;
  accrued: number;
  impliedRate: number;
  spreadBps: number;
  accrualFraction: number;
  tenorDescription: string;
};

export const DepositValuationExample: React.FC = () => {
  const [metrics, setMetrics] = useState<DepositMetrics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(2024, 1, 15);
        const end = new FsDate(2024, 4, 15);
        const valuationDate = new FsDate(2024, 2, 15);
        const quoteRate = 0.0525;

        // Set curve base date to start to avoid date range errors
        const discountCurve = new DiscountCurve(
          'USD-OIS',
          start,
          new Float64Array([0.0, 0.25, 0.5, 1.0]),
          new Float64Array([1.0, 0.998, 0.9945, 0.9875]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);

        const notional = Money.fromCode(5_000_000, 'USD');

        const deposit = new Deposit(
          'usd_deposit_3m',
          notional,
          start,
          end,
          DayCount.act360(),
          'USD-OIS',
          quoteRate
        );

        const registry = createStandardRegistry();
        const result = registry.priceDeposit(deposit, 'discounting', market);

        // Extract primitives immediately
        const presentValue = result.presentValue.amount;
        
        // Calculate metrics manually
        const dayCount = DayCount.act360();
        const accrualFraction = dayCount.yearFraction(start, end, undefined);
        const elapsed = dayCount.yearFraction(start, valuationDate, undefined);
        const accrued = notional.amount * quoteRate * Math.max(Math.min(elapsed, accrualFraction), 0);
        const cleanPv = presentValue - accrued;
        
        const dfEnd = discountCurve.dfOnDate(end, undefined);
        const dfStart = discountCurve.dfOnDate(start, undefined);
        const impliedRate = accrualFraction > 0 ? (dfStart / dfEnd - 1) / accrualFraction : 0;
        const spreadBps = (quoteRate - impliedRate) * 10_000;

        const tenorDescription = `${start.toString()} → ${end.toString()}`;

        if (!cancelled) {
          setMetrics({
            presentValue,
            quoteRate,
            cleanPv,
            accrued,
            impliedRate,
            spreadBps,
            accrualFraction,
            tenorDescription,
          });
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Deposit error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (!metrics) {
    return <p>Valuing deposit…</p>;
  }

  const { presentValue, quoteRate, cleanPv, accrued, impliedRate, spreadBps, accrualFraction, tenorDescription } = metrics;

  return (
    <section className="example-section">
      <h2>Money-Market Deposit Valuation</h2>
      <p>
        The deposit present value, accrued interest, and clean price are sourced directly from the
        Rust pricing registry. The example mirrors the Python walkthrough by comparing the quoted
        simple rate to the curve-implied forward rate.
      </p>

      <div className="inline-cards">
        <div className="card">
          <strong>Instrument</strong>
          <span>3M USD Cash Deposit</span>
        </div>
        <div className="card">
          <strong>Tenor</strong>
          <span>{tenorDescription}</span>
        </div>
        <div className="card">
          <strong>Accrual (Act/360)</strong>
          <span>{percentFormatter.format(accrualFraction)}</span>
        </div>
      </div>

      <table>
        <thead>
          <tr>
            <th>Quoted Simple Rate</th>
            <th>PV (Dirty)</th>
            <th>Accrued Interest</th>
            <th>Clean PV</th>
            <th>Implied Curve Rate</th>
            <th>Spread</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>{percentFormatter.format(quoteRate)}</td>
            <td>{currencyFormatter.format(presentValue)}</td>
            <td>{currencyFormatter.format(accrued)}</td>
            <td>{currencyFormatter.format(cleanPv)}</td>
            <td>{percentFormatter.format(impliedRate)}</td>
            <td>{bpsFormatter.format(spreadBps)} bps</td>
          </tr>
        </tbody>
      </table>
    </section>
  );
};
