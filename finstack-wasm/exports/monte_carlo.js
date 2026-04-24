import * as wasm from '../pkg/finstack_wasm.js';

export const monte_carlo = {
  // European pricing — MC
  priceEuropeanCall: wasm.priceEuropeanCall,
  priceEuropeanPut: wasm.priceEuropeanPut,
  // Path-dependent pricing — MC
  priceAsianCall: wasm.priceAsianCall,
  priceAsianPut: wasm.priceAsianPut,
  // LSMC pricing
  priceAmericanPut: wasm.priceAmericanPut,
  priceAmericanCall: wasm.priceAmericanCall,
  // Analytical
  blackScholesCall: wasm.blackScholesCall,
  blackScholesPut: wasm.blackScholesPut,
};
