// Type declarations for the finstack-wasm namespaced facade.
//
// Each namespace corresponds to a Rust crate domain.

export { default } from "./pkg/finstack_wasm";

export declare const core: Record<string, unknown>;
export declare const analytics: Record<string, unknown>;
export declare const correlation: Record<string, unknown>;
export declare const monte_carlo: Record<string, unknown>;
export declare const margin: Record<string, unknown>;
export declare const valuations: Record<string, unknown>;
export declare const statements: Record<string, unknown>;
export declare const statements_analytics: Record<string, unknown>;
export declare const portfolio: Record<string, unknown>;
export declare const scenarios: Record<string, unknown>;
