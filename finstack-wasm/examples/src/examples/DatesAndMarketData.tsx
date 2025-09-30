import React, { useEffect, useState } from "react";
import init, {
  Date as FsDate,
  Money,
  Period,
  buildPeriods,
  DiscountCurve,
  MarketContext,
  MarketScalar,
  ScalarTimeSeries,
  SeriesInterpolation,
  Currency,
  FxConfig,
  FxConversionPolicy,
  FxMatrix,
  DayCount,
  InterpStyle,
  ExtrapolationPolicy,
} from "finstack-wasm";

type PeriodRow = {
  id: string;
  start: string;
  end: string;
  isActual: boolean;
};

type MarketSnapshot = {
  discountFactor: number;
  fxRate: number;
  cpiLevel: number;
  equitySpot: number;
};

const toIso = (date: FsDate) => {
  const month = String(date.month).padStart(2, "0");
  const day = String(date.day).padStart(2, "0");
  return `${date.year}-${month}-${day}`;
};

export const PeriodPlanExample: React.FC = () => {
  const [periods, setPeriods] = useState<PeriodRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await init();
        const plan = buildPeriods("2024Q1..Q4", "2024Q2");
        const raw = plan.toArray();
        const rows: PeriodRow[] = raw.map((period: Period) => ({
          id: period.id.code,
          start: toIso(period.start),
          end: toIso(period.end),
          isActual: period.isActual,
        }));
        
        if (!cancelled) {
          setPeriods(rows);
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
  }, []);

  if (error) {
    return <p className="error">{error}</p>;
  }

  return (
    <section className="example-section">
      <h2>Fiscal Quarter Plan</h2>
      <table>
        <thead>
          <tr>
            <th>Period</th>
            <th>Start</th>
            <th>End</th>
            <th>Actual?</th>
          </tr>
        </thead>
        <tbody>
          {periods.map((row) => (
            <tr key={row.id}>
              <td>{row.id}</td>
              <td>{row.start}</td>
              <td>{row.end}</td>
              <td>{row.isActual ? "yes" : "no"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};

export const MarketDataExample: React.FC = () => {
  const [snapshot, setSnapshot] = useState<MarketSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await init();
        
        // Create currencies and base date
        const usd = new Currency("USD");
        const eur = new Currency("EUR");
        const baseDate = new FsDate(2024, 1, 2);
        
        // Create discount curve
        const curve = new DiscountCurve(
          "USD-OIS",
          baseDate,
          new Float64Array([0.0, 0.5, 1.0, 2.0]),
          new Float64Array([1.0, 0.9905, 0.979, 0.955]),
          "act_365f",
          "monotone_convex",
          "flat_forward",
          true
        );

        // Create CPI time series
        const cpiDates = [new FsDate(2023, 12, 31), new FsDate(2024, 3, 31)];
        const series = new ScalarTimeSeries(
          "US-CPI",
          cpiDates,
          new Float64Array([300.1, 302.8]),
          usd,
          SeriesInterpolation.Linear()
        );

        // Create FX matrix and set quote
        const fx = new FxMatrix();
        fx.setQuote(usd, eur, 0.92);

        // Query FX rate
        const policy = FxConversionPolicy.CashflowDate();
        const fxQuote = fx.rate(usd, eur, baseDate, policy);
        const fxRate = fxQuote.rate;

        // Create market context and insert data
        const context = new MarketContext();
        context.insertDiscount(curve);
        context.insertFx(fx);
        context.insertSeries(series);

        // Add equity price
        const priceMoney = Money.fromCode(102.45, "USD");
        const equitySpot = MarketScalar.price(priceMoney);
        context.insertPrice("AAPL", equitySpot);

        // Query data from context
        const fetchedCurve = context.discount("USD-OIS");
        const discountFactor = fetchedCurve.df(1.0);

        const fetchedSeries = context.series("US-CPI");
        const lookThroughDate = new FsDate(2024, 2, 15);
        const cpiLevel = fetchedSeries.valueOn(lookThroughDate);

        const storedSpot = context.price("AAPL");
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
  }, []);

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
        <dt>USD/EUR Spot</dt>
        <dd>{snapshot.fxRate.toFixed(4)}</dd>
        <dt>CPI Interpolated</dt>
        <dd>{snapshot.cpiLevel.toFixed(2)}</dd>
        <dt>Equity Spot (USD)</dt>
        <dd>{snapshot.equitySpot.toFixed(2)}</dd>
      </dl>
    </section>
  );
};
