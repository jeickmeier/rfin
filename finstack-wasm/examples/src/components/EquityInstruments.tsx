import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  Equity,
  EquityOption,
  MarketContext,
  MarketScalar,
  Money,
  PricingRequest,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { EquityInstrumentsProps, DEFAULT_EQUITY_PROPS } from './data/equity';
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

type RequiredEquityInstrumentsProps = Required<EquityInstrumentsProps>;

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  ticker: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const EquityInstrumentsExample: React.FC<EquityInstrumentsProps> = (props) => {
  const defaults = DEFAULT_EQUITY_PROPS as RequiredEquityInstrumentsProps;
  const {
    valuationDate = defaults.valuationDate,
    discountCurve = defaults.discountCurve,
    volSurface = defaults.volSurface,
    marketData = defaults.marketData,
    positions = defaults.positions,
    options = defaults.options,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);
        const usd = new Currency('USD');

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

        const equityVol = new VolSurface(
          volSurface.id,
          new Float64Array(volSurface.expiries),
          new Float64Array(volSurface.strikes),
          new Float64Array(volSurface.vols)
        );

        const market = new MarketContext();
        market.insertDiscount(curve);
        market.insertSurface(equityVol);

        for (const data of marketData) {
          const spotPrice = Money.fromCode(data.spotPrice.amount, data.spotPrice.currency);
          market.insertPrice(data.ticker, MarketScalar.price(spotPrice));
          market.insertPrice(`${data.ticker}-SPOT`, MarketScalar.price(spotPrice));
          market.insertPrice('EQUITY-SPOT', MarketScalar.price(spotPrice));
          market.insertPrice(`${data.ticker}-DIVYIELD`, MarketScalar.unitless(data.dividendYield));
          market.insertPrice('EQUITY-DIVYIELD', MarketScalar.unitless(data.dividendYield));
        }

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const pos of positions) {
          const equity = new Equity(pos.id, pos.ticker, usd, pos.quantity, pos.costBasis);
          const equityResult = registry.priceInstrument(equity, 'discounting', market, asOf);
          results.push({
            name: `${pos.ticker} Stock (${pos.quantity} shares)`,
            type: 'Equity',
            ticker: pos.ticker,
            presentValue: equityResult.presentValue.amount,
          });
        }

        for (const opt of options) {
          const expiryDate = new FsDate(
            opt.expiryDate.year,
            opt.expiryDate.month,
            opt.expiryDate.day
          );
          const spotPrice = Money.fromCode(opt.spotPrice.amount, opt.spotPrice.currency);

          const option =
            opt.optionType === 'call'
              ? EquityOption.europeanCall(
                  opt.id,
                  opt.ticker,
                  opt.strike,
                  expiryDate,
                  spotPrice,
                  opt.quantity
                )
              : EquityOption.europeanPut(
                  opt.id,
                  opt.ticker,
                  opt.strike,
                  expiryDate,
                  spotPrice,
                  opt.quantity
                );

          const isCall = opt.optionType === 'call';
          const opts = isCall ? new PricingRequest().withMetrics(['delta', 'gamma']) : null;
          const optResult = registry.priceInstrument(option, 'discounting', market, asOf, opts);

          const tenorDesc =
            opt.expiryDate.month === 12 ? '1Y' : `${opt.expiryDate.month - valuationDate.month}M`;

          results.push({
            name: `${opt.ticker} ${opt.optionType === 'call' ? 'Call' : 'Put'} @ $${opt.strike} (${tenorDesc})`,
            type: 'EquityOption',
            ticker: opt.ticker,
            presentValue: optResult.presentValue.amount,
            keyMetric: isCall
              ? { name: 'Delta', value: optResult.metric('delta') ?? 0 }
              : undefined,
          });
        }

        if (!cancelled) setRows(results);
      } catch (err) {
        if (!cancelled) {
          console.error('Equity instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurve, volSurface, marketData, positions, options]);

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
        <span className="ml-3 text-muted-foreground">Building equity instruments…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Equity Instruments</CardTitle>
        <CardDescription>
          Equity spot positions and European-style equity options (calls and puts). Options are
          priced using market data for spot prices and dividend yields.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Instrument</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Ticker</TableHead>
                <TableHead className="text-right">Present Value</TableHead>
                <TableHead className="text-right">Key Metric</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map(({ name, type, ticker, presentValue, keyMetric }) => (
                <TableRow key={name}>
                  <TableCell className="font-medium">{name}</TableCell>
                  <TableCell>
                    <Badge variant={type === 'Equity' ? 'default' : 'secondary'}>{type}</Badge>
                  </TableCell>
                  <TableCell className="font-mono">{ticker}</TableCell>
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
