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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';

type RequiredInflationInstrumentsProps = Required<InflationInstrumentsProps>;

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
  const defaults = DEFAULT_INFLATION_PROPS as RequiredInflationInstrumentsProps;
  const {
    valuationDate = defaults.valuationDate,
    discountCurve = defaults.discountCurve,
    inflationCurve = defaults.inflationCurve,
    bonds = defaults.bonds,
    swaps = defaults.swaps,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

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

        for (const bond of bonds) {
          const issueDate = new FsDate(
            bond.issueDate.year,
            bond.issueDate.month,
            bond.issueDate.day
          );
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
          const ilbResult = registry.priceInstrument(ilb, 'discounting', market, asOf);
          results.push({
            name: `US TIPS ${bond.maturityDate.year}`,
            type: 'InflationLinkedBond',
            presentValue: ilbResult.presentValue.amount,
            keyMetric: { name: 'Real Coupon', value: `${(bond.realCoupon * 100).toFixed(2)}%` },
          });
        }

        for (const swap of swaps) {
          const startDate = new FsDate(
            swap.startDate.year,
            swap.startDate.month,
            swap.startDate.day
          );
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
          const swapResult = registry.priceInstrument(infSwap, 'discounting', market, asOf);
          results.push({
            name: `ZC Inflation Swap (${tenorYears}Y)`,
            type: 'InflationSwap',
            presentValue: swapResult.presentValue.amount,
            keyMetric: { name: 'Fixed Rate', value: `${(swap.fixedRate * 100).toFixed(2)}%` },
          });
        }

        if (!cancelled) setRows(results);
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
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Building inflation instruments…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Inflation Instruments</CardTitle>
        <CardDescription>
          Inflation-linked bonds (TIPS-style) and zero-coupon inflation swaps. These instruments use
          inflation curves to project CPI levels and adjust cashflows accordingly.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Instrument</TableHead>
                <TableHead>Type</TableHead>
                <TableHead className="text-right">Present Value</TableHead>
                <TableHead className="text-right">Key Metric</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map(({ name, type, presentValue, keyMetric }) => (
                <TableRow key={name}>
                  <TableCell className="font-medium">{name}</TableCell>
                  <TableCell>
                    <Badge variant={type === 'InflationLinkedBond' ? 'default' : 'secondary'}>
                      {type}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right font-mono">
                    {currencyFormatter.format(presentValue)}
                  </TableCell>
                  <TableCell className="text-right font-mono text-muted-foreground">
                    {keyMetric ? `${keyMetric.name}: ${keyMetric.value}` : '—'}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
};
