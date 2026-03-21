import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  FxMatrix,
  FxOption,
  FxSpot,
  FxSwap,
  MarketContext,
  Money,
  PricingRequest,
  VolSurface,
  standardRegistry,
} from 'finstack-wasm';
import { FxInstrumentsProps, DEFAULT_FX_PROPS } from './data/fx';
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

type RequiredFxInstrumentsProps = Required<FxInstrumentsProps>;

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  pair: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const FxInstrumentsExample: React.FC<FxInstrumentsProps> = (props) => {
  const defaults = DEFAULT_FX_PROPS as RequiredFxInstrumentsProps;
  const {
    valuationDate = defaults.valuationDate,
    discountCurves = defaults.discountCurves,
    volSurface = defaults.volSurface,
    fxQuotes = defaults.fxQuotes,
    spots = defaults.spots,
    options = defaults.options,
    swaps = defaults.swaps,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
        const market = new MarketContext();

        for (const curveData of discountCurves) {
          const curveBaseDate = new FsDate(
            curveData.baseDate.year,
            curveData.baseDate.month,
            curveData.baseDate.day
          );
          const curve = new DiscountCurve(
            curveData.id,
            curveBaseDate,
            new Float64Array(curveData.tenors),
            new Float64Array(curveData.discountFactors),
            curveData.dayCount,
            curveData.interpolation,
            curveData.extrapolation,
            curveData.continuous
          );
          market.insertDiscount(curve);
        }

        const fx = new FxMatrix();
        for (const quote of fxQuotes) {
          const base = new Currency(quote.base);
          const quoteCcy = new Currency(quote.quote);
          fx.setQuote(base, quoteCcy, quote.rate);
        }
        market.insertFx(fx);

        const fxVol = new VolSurface(
          volSurface.id,
          new Float64Array(volSurface.expiries),
          new Float64Array(volSurface.strikes),
          new Float64Array(volSurface.vols)
        );
        market.insertSurface(fxVol);

        const registry = standardRegistry();
        const results: InstrumentRow[] = [];

        for (const spot of spots) {
          const baseCcy = new Currency(spot.baseCurrency);
          const quoteCcy = new Currency(spot.quoteCurrency);
          const settlementDate = new FsDate(
            spot.settlementDate.year,
            spot.settlementDate.month,
            spot.settlementDate.day
          );
          const notional = Money.fromCode(spot.notional.amount, spot.notional.currency);

          const fxSpot = new FxSpot(
            spot.id,
            baseCcy,
            quoteCcy,
            settlementDate,
            spot.rate,
            notional
          );
          const spotResult = registry.priceInstrument(fxSpot, 'discounting', market, asOf);
          results.push({
            name: `${spot.baseCurrency}/${spot.quoteCurrency} Spot`,
            type: 'FxSpot',
            pair: `${spot.baseCurrency}${spot.quoteCurrency}`,
            presentValue: spotResult.presentValue.amount,
          });
        }

        for (const opt of options) {
          const baseCcy = new Currency(opt.baseCurrency);
          const quoteCcy = new Currency(opt.quoteCurrency);
          const expiryDate = new FsDate(
            opt.expiryDate.year,
            opt.expiryDate.month,
            opt.expiryDate.day
          );
          const notional = Money.fromCode(opt.notional.amount, opt.notional.currency);

          const option =
            opt.optionType === 'call'
              ? new FxOption(
                  opt.id,
                  baseCcy,
                  quoteCcy,
                  opt.strike,
                  'call',
                  expiryDate,
                  notional,
                  opt.domesticCurveId,
                  opt.foreignCurveId,
                  opt.volSurfaceId
                )
              : new FxOption(
                  opt.id,
                  baseCcy,
                  quoteCcy,
                  opt.strike,
                  'put',
                  expiryDate,
                  notional,
                  opt.domesticCurveId,
                  opt.foreignCurveId,
                  opt.volSurfaceId
                );

          const isCall = opt.optionType === 'call';
          const optReq = isCall ? new PricingRequest().withMetrics(['delta']) : null;
          const optResult = registry.priceInstrument(option, 'discounting', market, asOf, optReq);

          const tenorMonths =
            (opt.expiryDate.year - valuationDate.year) * 12 +
            (opt.expiryDate.month - valuationDate.month);
          const tenorDesc = tenorMonths >= 12 ? `${tenorMonths / 12}Y` : `${tenorMonths}M`;

          results.push({
            name: `${tenorDesc} ${opt.optionType === 'call' ? 'Call' : 'Put'} @ ${opt.strike.toFixed(2)}`,
            type: 'FxOption',
            pair: `${opt.baseCurrency}${opt.quoteCurrency}`,
            presentValue: optResult.presentValue.amount,
            keyMetric: isCall
              ? {
                  name: 'Delta',
                  value:
                    Math.abs(optResult.metric('delta') ?? 0) > 100
                      ? (optResult.metric('delta') ?? 0) / opt.notional.amount
                      : (optResult.metric('delta') ?? 0),
                }
              : undefined,
          });
        }

        for (const swap of swaps) {
          const baseCcy = new Currency(swap.baseCurrency);
          const quoteCcy = new Currency(swap.quoteCurrency);
          const notional = Money.fromCode(swap.notional.amount, swap.notional.currency);
          const nearDate = new FsDate(swap.nearDate.year, swap.nearDate.month, swap.nearDate.day);
          const farDate = new FsDate(swap.farDate.year, swap.farDate.month, swap.farDate.day);

          const fxSwap = new FxSwap(
            swap.id,
            baseCcy,
            quoteCcy,
            notional,
            nearDate,
            farDate,
            swap.domesticCurveId,
            swap.foreignCurveId,
            swap.nearRate,
            swap.farRate
          );
          const swapResult = registry.priceInstrument(fxSwap, 'discounting', market, asOf);

          const tenorMonths =
            (swap.farDate.year - swap.nearDate.year) * 12 +
            (swap.farDate.month - swap.nearDate.month);

          results.push({
            name: `${tenorMonths}M FX Swap`,
            type: 'FxSwap',
            pair: `${swap.baseCurrency}${swap.quoteCurrency}`,
            presentValue: swapResult.presentValue.amount,
          });
        }

        if (!cancelled) setRows(results);
      } catch (err) {
        if (!cancelled) {
          console.error('FX instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurves, volSurface, fxQuotes, spots, options, swaps]);

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
        <span className="ml-3 text-muted-foreground">Building FX instruments…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>FX Instruments</CardTitle>
        <CardDescription>
          Foreign exchange instruments including spot transactions, European options (calls/puts),
          and FX swaps with near and far legs.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Instrument</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Pair</TableHead>
                <TableHead className="text-right">Present Value</TableHead>
                <TableHead className="text-right">Key Metric</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map(({ name, type, pair, presentValue, keyMetric }) => (
                <TableRow key={name}>
                  <TableCell className="font-medium">{name}</TableCell>
                  <TableCell>
                    <Badge
                      variant={
                        type === 'FxSpot'
                          ? 'default'
                          : type === 'FxOption'
                            ? 'secondary'
                            : 'outline'
                      }
                    >
                      {type}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono">{pair}</TableCell>
                  <TableCell className="text-right font-mono">
                    {currencyFormatter.format(presentValue)}
                  </TableCell>
                  <TableCell className="text-right font-mono text-muted-foreground">
                    {keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}
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
