import React, { useEffect, useState } from 'react';
import { Route, Routes } from 'react-router-dom';
import init from 'finstack-wasm';
import './App.css';
import ExamplePage from './pages/ExamplePage';
import Home from './pages/Home';
import NotFound from './pages/NotFound';

// Global flag to ensure WASM is only initialized once across all hot reloads
let wasmInitialized = false;

const App: React.FC = () => {
  const [wasmReady, setWasmReady] = useState(wasmInitialized);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Only initialize WASM once, even across hot module reloads
    if (wasmInitialized) {
      setWasmReady(true);
      return;
    }

    init()
      .then(() => {
        wasmInitialized = true;
        setWasmReady(true);
      })
      .catch((err) => {
        setError(`Failed to initialize WASM: ${err.message}`);
      });
  }, []);

  return (
    <main className="container">
      {error ? (
        <>
          <h1>finstack-wasm Examples</h1>
          <p className="error">{error}</p>
        </>
      ) : !wasmReady ? (
        <>
          <h1>finstack-wasm Examples</h1>
          <p>Loading WASM module...</p>
        </>
      ) : (
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/examples/:slug" element={<ExamplePage />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      )}
    </main>
  );
};

export default App;
