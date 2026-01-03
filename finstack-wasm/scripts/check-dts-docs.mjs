import fs from 'node:fs';
import path from 'node:path';

function fail(message) {
  console.error(message);
  process.exit(1);
}

const dtsPath = path.join(process.cwd(), 'pkg', 'finstack_wasm.d.ts');
if (!fs.existsSync(dtsPath)) {
  fail(
    `Missing generated typings at ${dtsPath}. Run 'npm run build' (or 'wasm-pack build --target web') first.`
  );
}

const dts = fs.readFileSync(dtsPath, 'utf8');

/** @type {{name: string, patterns: (string|RegExp)[]}[]} */
const checks = [
  {
    name: 'DayCount.yearFraction docs',
    patterns: [
      /Compute the year fraction between two dates using this day count convention\./,
      /@param start -/,
    ],
  },
  {
    name: 'Bond constructor docs',
    patterns: [
      /Construct a bond with full control over schedule and coupon conventions\./,
      /@param instrument_id - Unique identifier/,
    ],
  },
  {
    name: 'Deposit constructor docs',
    patterns: [/Create a money-market deposit accruing simple interest over a date range\./],
  },
  {
    name: 'IRS constructor docs',
    patterns: [/Create a plain-vanilla fixed-for-floating interest rate swap\./],
  },
  {
    name: 'CDS buyProtection docs',
    patterns: [/Create a CDS position that \*\*buys protection\*\*/],
  },
  {
    name: 'InflationLinkedBond constructor docs',
    patterns: [/Create an inflation-linked bond \(TIPS-style by default\)\./],
  },
  {
    name: 'Swaption payer docs',
    patterns: [/Create a payer swaption \(option to enter a payer swap\)\./],
  },
  {
    name: 'EquityOption europeanCall docs',
    patterns: [/Create a European equity call option\./],
  },
];

const missing = [];
for (const check of checks) {
  for (const pattern of check.patterns) {
    const ok = typeof pattern === 'string' ? dts.includes(pattern) : pattern.test(dts);
    if (!ok) {
      missing.push(`${check.name}: missing ${pattern.toString()}`);
      break;
    }
  }
}

if (missing.length) {
  fail(
    [
      'Generated TypeScript declarations appear to be missing expected documentation blocks.',
      '',
      ...missing.map((m) => `- ${m}`),
    ].join('\n')
  );
}

console.log('OK: generated TypeScript declarations include required documentation.');
