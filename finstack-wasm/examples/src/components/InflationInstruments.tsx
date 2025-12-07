import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  InflationCurve,
  InflationLinkedBond,
  InflationSwap,
  MarketContext,
  Money,
  createStandardRegistry,
} from 'finstack-wasm';
import { InflationInstrumentsProps, DEFAULT_INFLATION_PROPS } from './data/inflation';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  presentValue: number;
  keyMetric?: { name: string; value: string };
};

export const InflationInstrumentsExample: React.FC<InflationInstrumentsProps> = (props) => {
  // Merge with defaults
  const {
    valuationDate = DEFAULT_INFLATION_PROPS.valuationDate!,
    discountCurve = DEFAULT_INFLATION_PROPS.discountCurve!,
    inflationCurve = DEFAULT_INFLATION_PROPS.inflationCurve!,
    bonds = DEFAULT_INFLATION_PROPS.bonds!,
    swaps = DEFAULT_INFLATION_PROPS.swaps!,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

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

        // Build inflation curve from props
        const infCurve = new InflationCurve(
          inflationCurve.id,
          inflationCurve.baseIndex,
          new Float64Array(inflationCurve.tenors),
          new Float64Array(inflationCurve.indexLevels),
          inflationCurve.interpolation
        );

        const market = new MarketContext();
        market.insertDiscount(curve);
        market.insertInflation(infCurve);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process inflation-linked bonds
        for (const bond of bonds) {
          const issueDate = new FsDate(bond.issueDate.year, bond.issueDate.month, bond.issueDate.day);
          const maturityDate = new FsDate(
            bond.maturityDate.year,
            bond.maturityDate.month,
            bond.maturityDate.day
          );
          const notional = Money.fromCode(bond.notional.amount, bond.notional.currency);

          const ilb = new InflationLinkedBond(
            bond.id,
            notional,
            bond.realCoupon,
            issueDate,
            maturityDate,
            bond.baseIndex,
            bond.discountCurveId,
            bond.inflationCurveId,
            bond.bondType,
            bond.frequency,
            null,
            null
          );
          const ilbResult = registry.priceInflationLinkedBond(ilb, 'discounting', market, asOf);
          results.push({
            name: `US TIPS ${bond.maturityDate.year}`,
            type: 'InflationLinkedBond',
            presentValue: ilbResult.presentValue.amount,
            keyMetric: { name: 'Real Coupon', value: `${(bond.realCoupon * 100).toFixed(2)}%` },
          });
        }

        // Process inflation swaps
        for (const swap of swaps) {
          const startDate = new FsDate(swap.startDate.year, swap.startDate.month, swap.startDate.day);
          const endDate = new FsDate(swap.endDate.year, swap.endDate.month, swap.endDate.day);
          const notional = Money.fromCode(swap.notional.amount, swap.notional.currency);

          const tenorYears = swap.endDate.year - swap.startDate.year;

          const infSwap = new InflationSwap(
            swap.id,
            notional,
            swap.fixedRate,
            startDate,
            endDate,
            swap.discountCurveId,
            swap.inflationCurveId,
            swap.direction,
            swap.dayCount
          );
          const swapResult = registry.priceInflationSwap(infSwap, 'discounting', market, asOf);
          results.push({
            name: `ZC Inflation Swap (${tenorYears}Y)`,
            type: 'InflationSwap',
            presentValue: swapResult.presentValue.amount,
            keyMetric: { name: 'Fixed Rate', value: `${(swap.fixedRate * 100).toFixed(2)}%` },
          });
        }

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Inflation instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurve, inflationCurve, bonds, swaps]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building inflation instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Inflation Instruments</h2>
      <p>
        Inflation-linked bonds (TIPS-style) and zero-coupon inflation swaps. These instruments use
        inflation curves to project CPI levels and adjust cashflows accordingly.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
