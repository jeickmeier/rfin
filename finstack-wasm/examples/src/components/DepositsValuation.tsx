import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DayCount,
  DepositBuilder,
  DiscountCurve,
  MarketContext,
  Money,
  standardRegistry,
} from 'finstack-wasm';
import { DepositValuationProps, DEFAULT_DEPOSIT_PROPS } from './data/deposits';
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
        const maturity = new FsDate(
          deposit.maturity.year,
          deposit.maturity.month,
          deposit.maturity.day
        );
        const valDate = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
        const quoteRate = deposit.quoteRate;

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

        const depositInst = new DepositBuilder(deposit.id)
          .money(notional)
          .start(start)
          .maturity(maturity)
          .dayCount(DayCount.act360())
          .discountCurve(deposit.discountCurveId)
          .quoteRate(quoteRate)
          .build();

        const registry = standardRegistry();
        const result = registry.priceInstrument(depositInst, 'discounting', market, valDate, null);

        const presentValue = result.presentValue.amount;

        const dayCount = DayCount.act360();
        const accrualFraction = dayCount.yearFraction(start, maturity);
        const elapsed = dayCount.yearFraction(start, valDate);
        const accrued =
          notional.amount * quoteRate * Math.max(Math.min(elapsed, accrualFraction), 0);
        const cleanPv = presentValue - accrued;

        const dfEnd = curve.dfOnDate(maturity, null);
        const dfStart = curve.dfOnDate(start, null);
        const impliedRate = accrualFraction > 0 ? (dfStart / dfEnd - 1) / accrualFraction : 0;
        const spreadBps = (quoteRate - impliedRate) * 10_000;

        const tenorDescription = `${start.toString()} → ${maturity.toString()}`;

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
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!metrics) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Valuing deposit…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Money-Market Deposit Valuation</CardTitle>
        <CardDescription>
          The deposit present value, accrued interest, and clean price are sourced directly from the
          Rust pricing registry. The example mirrors the Python walkthrough by comparing the quoted
          simple rate to the curve-implied forward rate.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="flex flex-wrap gap-3">
          <Badge variant="secondary" className="px-3 py-1.5 text-sm">
            <span className="font-medium">Instrument:</span>
            <span className="ml-1.5 font-mono">3M USD Cash Deposit</span>
          </Badge>
          <Badge variant="secondary" className="px-3 py-1.5 text-sm">
            <span className="font-medium">Tenor:</span>
            <span className="ml-1.5 font-mono">{metrics.tenorDescription}</span>
          </Badge>
          <Badge variant="secondary" className="px-3 py-1.5 text-sm">
            <span className="font-medium">Accrual (Act/360):</span>
            <span className="ml-1.5 font-mono">
              {percentFormatter.format(metrics.accrualFraction)}
            </span>
          </Badge>
        </div>

        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Quoted Simple Rate</TableHead>
                <TableHead className="text-right">PV (Dirty)</TableHead>
                <TableHead className="text-right">Accrued Interest</TableHead>
                <TableHead className="text-right">Clean PV</TableHead>
                <TableHead className="text-right">Implied Curve Rate</TableHead>
                <TableHead className="text-right">Spread</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell className="font-mono">
                  {percentFormatter.format(metrics.quoteRate)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {currencyFormatter.format(metrics.presentValue)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {currencyFormatter.format(metrics.accrued)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {currencyFormatter.format(metrics.cleanPv)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {percentFormatter.format(metrics.impliedRate)}
                </TableCell>
                <TableCell className="text-right font-mono">
                  {bpsFormatter.format(metrics.spreadBps)} bps
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
};
