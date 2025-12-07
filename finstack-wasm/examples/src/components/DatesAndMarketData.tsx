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

type RequiredMarketDataExampleProps = Required<MarketDataExampleProps>;

type MarketSnapshot = {
  discountFactor: number;
  fxRate: number;
  cpiLevel: number;
  equitySpot: number;
};

export const MarketDataExample: React.FC<MarketDataExampleProps> = (props) => {
  // Merge with defaults - DEFAULT_MARKET_DATA_EXAMPLE_PROPS always has these values defined
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
        // Create currencies and base date
        const baseCurrency = new Currency(fxQuote.baseCurrency);
        const quoteCurrency = new Currency(fxQuote.quoteCurrency);
        const baseDateObj = new FsDate(baseDate.year, baseDate.month, baseDate.day);

        // Create discount curve
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

        // Create CPI time series
        const cpiDates = cpiSeries.dates.map((d) => new FsDate(d.year, d.month, d.day));
        const cpiCurrency = new Currency(cpiSeries.currency);
        const series = new ScalarTimeSeries(
          cpiSeries.id,
          cpiDates,
          new Float64Array(cpiSeries.values),
          cpiCurrency,
          SeriesInterpolation.Linear()
        );

        // Create FX matrix and set quote
        const fx = new FxMatrix();
        fx.setQuote(baseCurrency, quoteCurrency, fxQuote.rate);

        // Query FX rate
        const policy = FxConversionPolicy.CashflowDate();
        const fxQuoteResult = fx.rate(baseCurrency, quoteCurrency, baseDateObj, policy);
        const fxRate = fxQuoteResult.rate;

        // Create market context and insert data
        const context = new MarketContext();
        context.insertDiscount(curve);
        context.insertFx(fx);
        context.insertSeries(series);

        // Add equity price
        const priceMoney = Money.fromCode(equityPrice.price.amount, equityPrice.price.currency);
        const equitySpot = MarketScalar.price(priceMoney);
        context.insertPrice(equityPrice.symbol, equitySpot);

        // Query data from context
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
    return <p className="error">{error}</p>;
  }

  if (!snapshot) {
    return <p>Loading market data snapshot…</p>;
  }

  return (
    <section className="example-section">
      <h2>Market Data Snapshot</h2>
      <dl className="data-list">
        <dt>1Y Discount Factor</dt>
        <dd>{snapshot.discountFactor.toFixed(6)}</dd>
        <dt>
          {fxQuote.baseCurrency}/{fxQuote.quoteCurrency} Spot
        </dt>
        <dd>{snapshot.fxRate.toFixed(4)}</dd>
        <dt>CPI Interpolated</dt>
        <dd>{snapshot.cpiLevel.toFixed(2)}</dd>
        <dt>Equity Spot ({equityPrice.price.currency})</dt>
        <dd>{snapshot.equitySpot.toFixed(2)}</dd>
      </dl>
    </section>
  );
};
