import "@testing-library/jest-dom/vitest";

// Recharts and virtualization rely on ResizeObserver; provide a minimal stub for jsdom.
class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

// @ts-expect-error jsdom global assignment for tests
if (typeof globalThis.ResizeObserver === "undefined") {
  // @ts-expect-error jsdom global assignment for tests
  globalThis.ResizeObserver = ResizeObserver;
}
