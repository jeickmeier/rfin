import React, { useEffect, useState } from 'react';
import {
  FsDate,
  Money,
  DiscountCurve,
  MarketContext,
  MarketScalar,
  ScalarTimeSeries,
  SeriesInterpolation,
  Currency,
  FxConversionPolicy,
  FxMatrix,
} from 'finstack-wasm';
import {
  MarketDataExampleProps,
  DEFAULT_MARKET_DATA_EXAMPLE_PROPS,
} from './data/market-data-example';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';

type RequiredMarketDataExampleProps = Required<MarketDataExampleProps>;

type MarketSnapshot = {
  discountFactor: number;
  fxRate: number;
  cpiLevel: number;
  equitySpot: number;
};

export const MarketDataExample: React.FC<MarketDataExampleProps> = (props) => {
  const defaults = DEFAULT_MARKET_DATA_EXAMPLE_PROPS as RequiredMarketDataExampleProps;
  const {
    baseDate = defaults.baseDate,
    discountCurve = defaults.discountCurve,
    cpiSeries = defaults.cpiSeries,
    fxQuote = defaults.fxQuote,
    equityPrice = defaults.equityPrice,
    cpiLookupDate = defaults.cpiLookupDate,
  } = props;

  const [snapshot, setSnapshot] = useState<MarketSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const baseCurrency = new Currency(fxQuote.baseCurrency);
        const quoteCurrency = new Currency(fxQuote.quoteCurrency);
        const baseDateObj = new FsDate(baseDate.year, baseDate.month, baseDate.day);

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

        const cpiDates = cpiSeries.dates.map((d) => new FsDate(d.year, d.month, d.day));
        const cpiCurrency = new Currency(cpiSeries.currency);
        const series = new ScalarTimeSeries(
          cpiSeries.id,
          cpiDates,
          new Float64Array(cpiSeries.values),
          cpiCurrency,
          SeriesInterpolation.Linear()
        );

        const fx = new FxMatrix();
        fx.setQuote(baseCurrency, quoteCurrency, fxQuote.rate);

        const policy = FxConversionPolicy.CashflowDate();
        const fxQuoteResult = fx.rate(baseCurrency, quoteCurrency, baseDateObj, policy);
        const fxRate = fxQuoteResult.rate;

        const context = new MarketContext();
        context.insertDiscount(curve);
        context.insertFx(fx);
        context.insertSeries(series);

        const priceMoney = Money.fromCode(equityPrice.price.amount, equityPrice.price.currency);
        const equitySpot = MarketScalar.price(priceMoney);
        context.insertPrice(equityPrice.symbol, equitySpot);

        const fetchedCurve = context.discount(discountCurve.id);
        const discountFactor = fetchedCurve.df(1.0);

        const fetchedSeries = context.series(cpiSeries.id);
        const lookThroughDate = new FsDate(
          cpiLookupDate.year,
          cpiLookupDate.month,
          cpiLookupDate.day
        );
        const cpiLevel = fetchedSeries.valueOn(lookThroughDate);

        const storedSpot = context.price(equityPrice.symbol);
        const moneyValue = storedSpot.value as Money;
        const equitySpotAmount = moneyValue.amount;

        if (!cancelled) {
          setSnapshot({
            discountFactor,
            fxRate,
            cpiLevel,
            equitySpot: equitySpotAmount,
          });
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [baseDate, discountCurve, cpiSeries, fxQuote, equityPrice, cpiLookupDate]);

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!snapshot) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Loading market data snapshot…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Market Data Snapshot</CardTitle>
        <CardDescription>
          Assembled discount curves, CPI series, FX matrices, and equity prices
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">1Y Discount Factor</div>
            <div className="mt-1 font-mono text-2xl font-semibold">
              {snapshot.discountFactor.toFixed(6)}
            </div>
          </div>
          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              {fxQuote.baseCurrency}/{fxQuote.quoteCurrency} Spot
            </div>
            <div className="mt-1 font-mono text-2xl font-semibold">
              {snapshot.fxRate.toFixed(4)}
            </div>
          </div>
          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">CPI Interpolated</div>
            <div className="mt-1 font-mono text-2xl font-semibold">
              {snapshot.cpiLevel.toFixed(2)}
            </div>
          </div>
          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              Equity Spot ({equityPrice.price.currency})
            </div>
            <div className="mt-1 font-mono text-2xl font-semibold">
              {snapshot.equitySpot.toFixed(2)}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
