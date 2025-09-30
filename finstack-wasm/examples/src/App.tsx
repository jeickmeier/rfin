import React from 'react';
import { PeriodPlanExample, MarketDataExample } from './examples/DatesAndMarketData';
import './App.css';

const App: React.FC = () => {
  return (
    <main className="container">
      <h1>finstack-wasm TypeScript Examples</h1>
      <p className="intro">
        These examples demonstrate the usage of finstack-wasm in a React + TypeScript environment.
      </p>
      <PeriodPlanExample />
      <MarketDataExample />
    </main>
  );
};

export default App;
