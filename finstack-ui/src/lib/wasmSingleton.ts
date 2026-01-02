// WASM initialization singleton for the main thread
import * as wasmModule from "finstack-wasm";

let initPromise: Promise<void> | null = null;

export function canInitWasm(): boolean {
  return typeof window !== "undefined";
}

export function ensureWasmInit(): Promise<void> {
  if (!canInitWasm()) {
    // No-op in SSR / non-browser environments
    return Promise.resolve();
  }

  if (!initPromise) {
    // Handle both default export and named 'init' export patterns
    const mod = wasmModule as unknown as {
      default?: () => Promise<unknown>;
      init?: () => Promise<unknown>;
    };
    const initFn = mod.default ?? mod.init;

    if (typeof initFn !== "function") {
      initPromise = Promise.reject(
        new Error("finstack-wasm init function not found"),
      );
    } else {
      initPromise = initFn().then(() => undefined);
    }
  }
  return initPromise;
}

// For testing purposes only
export function __resetWasmInitForTests() {
  initPromise = null;
}
