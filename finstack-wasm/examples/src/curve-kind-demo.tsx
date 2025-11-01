import React from 'react';
import {
  MarketContext,
  CurveKind,
  DiscountCurve,
  FsDate,
} from 'finstack-wasm';

/**
 * Demonstrates the unified curve getter API with CurveKind enum
 * 
 * Before: Had to know exact curve type at compile time
 * After: Can dynamically dispatch based on CurveKind
 */
export const CurveKindDemo: React.FC = () => {
  const [output, setOutput] = React.useState<string>('');

  const runDemo = () => {
    const lines: string[] = [];

    // Create market context
    const ctx = new MarketContext();
    const asOf = new FsDate(2025, 1, 1);

    // Build a flat discount curve manually
    const rate = 0.045;
    const times = new Float64Array([0.0, 30.0]);
    const dfs = new Float64Array(times.length);
    for (let i = 0; i < times.length; i++) {
      dfs[i] = Math.exp(-rate * times[i]);
    }
    const curve = new DiscountCurve("USD-SOFR", asOf, times, dfs, "act_365f", "linear", "flat_forward", true);
    ctx.insertDiscount(curve);

    lines.push('=== Unified Curve Getter Demo ===\n');

    // Method 1: Generic getCurve with dynamic dispatch
    lines.push('1. Using getCurve() with CurveKind enum:');
    const genericCurve = ctx.getCurve("USD-SOFR", CurveKind.Discount) as any;
    const df1 = genericCurve.df(1.0); // 1 year
    lines.push(`   Discount Factor: ${df1.toFixed(6)}\n`);

    // Method 2: Type-safe convenience method (still available!)
    lines.push('2. Using typed convenience method discount():');
    const typedCurve = ctx.discount("USD-SOFR");
    const df2 = typedCurve.df(1.0); // 1 year
    lines.push(`   Discount Factor: ${df2.toFixed(6)}\n`);

    // Use case: Iterate over all curve types dynamically
    lines.push('3. Dynamic iteration over curve types:');
    const curveTypes = [
      { name: 'Discount', kind: CurveKind.Discount, id: 'USD-SOFR' },
      { name: 'Forward', kind: CurveKind.Forward, id: 'USD-LIBOR-3M' },
      { name: 'Hazard', kind: CurveKind.Hazard, id: 'CORP-AA' },
    ];

    for (const { name, kind, id } of curveTypes) {
      try {
        ctx.getCurve(id, kind);
        lines.push(`   ✓ Found ${name} curve: ${id}`);
      } catch (e) {
        lines.push(`   ✗ Missing ${name} curve: ${id}`);
      }
    }

    lines.push('\n4. Benefits:');
    lines.push('   • Single generic method replaces 5 duplicate methods');
    lines.push('   • Enables dynamic curve type selection');
    lines.push('   • Supports generic "all curves" enumeration');
    lines.push('   • Maintains type-safe convenience methods');
    lines.push('   • Reduces ~80 LOC of duplication');

    setOutput(lines.join('\n'));
  };

  return (
    <div className="demo-section">
      <h2>Unified Curve Getter with CurveKind</h2>
      <p>
        Demonstrates the new unified <code>getCurve()</code> method with dynamic
        dispatch via <code>CurveKind</code> enum.
      </p>
      <button onClick={runDemo}>Run Demo</button>
      {output && (
        <pre className="output-box">{output}</pre>
      )}
    </div>
  );
};

