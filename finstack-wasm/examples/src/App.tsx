import React, { useEffect, useState } from 'react';
import init from 'finstack-wasm';
import { MarketDataExample } from './examples/DatesAndMarketData';
import { CashflowBasicsExample } from './examples/CashflowBasics';
import { MathShowcaseExample } from './examples/MathShowcase';
import {
  DateConstructionExample,
  DateUtilitiesExample,
  CalendarExample,
  DayCountExample,
  ScheduleBuilderExample,
  PeriodPlansExample,
  IMMDatesExample,
  FrequencyExample,
} from './examples/DatesShowcase';
import './App.css';

const App: React.FC = () => {
  const [wasmReady, setWasmReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Initialize WASM once at app level
    init()
      .then(() => {
        setWasmReady(true);
      })
      .catch((err) => {
        setError(`Failed to initialize WASM: ${err.message}`);
      });
  }, []);

  if (error) {
    return (
      <main className="container">
        <h1>finstack-wasm Examples</h1>
        <p className="error">{error}</p>
      </main>
    );
  }

  if (!wasmReady) {
    return (
      <main className="container">
        <h1>finstack-wasm Examples</h1>
        <p>Loading WASM module...</p>
      </main>
    );
  }

  return (
    <main className="container">
      <h1>finstack-wasm TypeScript Examples</h1>
      <p className="intro">
        Comprehensive examples demonstrating the usage of finstack-wasm in a React + TypeScript environment.
        These examples mirror the Python bindings and showcase date handling, calendars, schedules, and market data.
      </p>

      <div style={{ borderTop: '2px solid #646cff', paddingTop: '2rem', marginTop: '2rem' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1rem', color: '#646cff' }}>
          Date & Calendar Functionality
        </h2>
        <DateConstructionExample />
        <DateUtilitiesExample />
        <CalendarExample />
        <DayCountExample />
        <ScheduleBuilderExample />
        <PeriodPlansExample />
        <IMMDatesExample />
        <FrequencyExample />
      </div>

      <div style={{ borderTop: '2px solid #646cff', paddingTop: '2rem', marginTop: '2rem' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1rem', color: '#646cff' }}>
          Market Data
        </h2>
        <MarketDataExample />
      </div>

      <div style={{ borderTop: '2px solid #646cff', paddingTop: '2rem', marginTop: '2rem' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1rem', color: '#646cff' }}>
          Cashflow Primitives
        </h2>
        <CashflowBasicsExample />
      </div>

      <div style={{ borderTop: '2px solid #646cff', paddingTop: '2rem', marginTop: '2rem' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1rem', color: '#646cff' }}>
          Math Utilities
        </h2>
        <MathShowcaseExample />
      </div>
    </main>
  );
};

export default App;
