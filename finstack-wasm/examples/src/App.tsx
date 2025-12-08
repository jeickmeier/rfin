import React, { useEffect, useState } from 'react';
import { Route, Routes } from 'react-router-dom';
import init from 'finstack-wasm';
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
      // Use setTimeout to avoid synchronous setState in effect
      setTimeout(() => setWasmReady(true), 0);
      return;
    }

    init()
      .then(() => {
        wasmInitialized = true;
        setWasmReady(true);
      })
      .catch((err) => {
        console.error('Failed to initialize WASM module:', err);
        setError(`Failed to initialize WASM: ${err.message}`);
      });
  }, []);

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {error ? (
          <div className="text-center">
            <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
              finstack-wasm Examples
            </h1>
            <p className="mt-4 rounded-lg border border-destructive bg-destructive/10 p-4 text-destructive">
              {error}
            </p>
          </div>
        ) : !wasmReady ? (
          <div className="flex flex-col items-center justify-center py-20">
            <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
              finstack-wasm Examples
            </h1>
            <p className="mt-4 text-muted-foreground">Loading WASM module...</p>
            <div className="mt-6 h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
          </div>
        ) : (
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/examples/:slug" element={<ExamplePage />} />
            <Route path="*" element={<NotFound />} />
          </Routes>
        )}
      </div>
    </main>
  );
};

export default App;
