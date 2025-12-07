import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DayCount,
  Deposit,
  DiscountCurve,
  MarketContext,
  Money,
  createStandardRegistry,
} from 'finstack-wasm';
import { DepositValuationProps, DEFAULT_DEPOSIT_PROPS } from './data/deposits';

type RequiredDepositValuationProps = Required<DepositValuationProps>;

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

export const DepositValuationExample: React.FC<DepositValuationProps> = (props) => {
  // Merge with defaults - DEFAULT_DEPOSIT_PROPS always has these values defined
  const defaults = DEFAULT_DEPOSIT_PROPS as RequiredDepositValuationProps;
  const {
    valuationDate = defaults.valuationDate,
    deposit = defaults.deposit,
    discountCurve = defaults.discountCurve,
  } = props;

  const [metrics, setMetrics] = useState<DepositMetrics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const start = new FsDate(
          deposit.startDate.year,
          deposit.startDate.month,
          deposit.startDate.day
        );
        const end = new FsDate(deposit.endDate.year, deposit.endDate.month, deposit.endDate.day);
        const valDate = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
        const quoteRate = deposit.quoteRate;

        // Debug type checks to ensure FsDate instances are from the same WASM module
        console.debug(
          'FsDate checks (deposit)',
          start instanceof FsDate,
          end instanceof FsDate,
          valDate instanceof FsDate
        );

        // Build discount curve from props
        const curveBaseDate = new FsDate(
          discountCurve.baseDate.year,
          discountCurve.baseDate.month,
          discountCurve.baseDate.day
        );
        const curve = new DiscountCurve(
          discountCurve.id,
          curveBaseDate,
          new Float64Array(discountCurve.tenors),
          new Float64Array(discountCurve.discountFactors),
          discountCurve.dayCount,
          discountCurve.interpolation,
          discountCurve.extrapolation,
          discountCurve.continuous
        );

        const market = new MarketContext();
        market.insertDiscount(curve);

        const notional = Money.fromCode(deposit.notional.amount, deposit.notional.currency);

        const depositInst = new Deposit(
          deposit.id,
          notional,
          start,
          end,
          DayCount.act360(),
          deposit.discountCurveId,
          quoteRate
        );

        const registry = createStandardRegistry();
        let result;
        try {
          result = registry.priceDeposit(depositInst, 'discounting', market, valDate, null);
        } catch (err) {
          console.error('priceDeposit failed', err);
          throw err;
        }

        // Extract primitives immediately
        const presentValue = result.presentValue.amount;

        // Calculate metrics manually
        const dayCount = DayCount.act360();
        let accrualFraction = 0;
        let elapsed = 0;
        try {
          accrualFraction = dayCount.yearFraction(start, end);
          elapsed = dayCount.yearFraction(start, valDate);
        } catch (err) {
          console.error('yearFraction failed', err);
          throw err;
        }
        const accrued =
          notional.amount * quoteRate * Math.max(Math.min(elapsed, accrualFraction), 0);
        const cleanPv = presentValue - accrued;

        let dfEnd = 0;
        let dfStart = 0;
        try {
          dfEnd = curve.dfOnDate(end, null);
          dfStart = curve.dfOnDate(start, null);
        } catch (err) {
          console.error('dfOnDate failed', err);
          throw err;
        }
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
  }, [valuationDate, deposit, discountCurve]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (!metrics) {
    return <p>Valuing deposit…</p>;
  }

  const {
    presentValue,
    quoteRate,
    cleanPv,
    accrued,
    impliedRate,
    spreadBps,
    accrualFraction,
    tenorDescription,
  } = metrics;

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
