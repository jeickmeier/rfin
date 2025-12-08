#!/usr/bin/env node
/**
 * Generate TypeScript types from Finstack JSON schemas.
 * 
 * This script generates TypeScript interfaces from the JSON schemas
 * defined in finstack/valuations/schemas/, eliminating the need to
 * manually maintain TypeScript type definitions.
 * 
 * Usage:
 *   node scripts/generate-types.js
 * 
 * Or add to package.json:
 *   "scripts": { "generate:types": "node scripts/generate-types.js" }
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '../../../');
const SCHEMAS_DIR = path.join(ROOT, 'finstack/valuations/schemas');
const OUTPUT_DIR = path.join(__dirname, '../src/types/generated');

// Ensure output directory exists
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

// Schema categories to generate
const SCHEMA_CATEGORIES = [
  { name: 'calibration', path: 'calibration/1' },
  { name: 'instruments', path: 'instruments/1' },
  // Add more as needed
];

console.log('🔧 Generating TypeScript types from JSON schemas...\n');

// Check if json-schema-to-typescript is installed
try {
  execSync('npx json-schema-to-typescript --help', { stdio: 'ignore' });
} catch {
  console.log('📦 Installing json-schema-to-typescript...');
  execSync('npm install -D json-schema-to-typescript', { stdio: 'inherit' });
}

for (const category of SCHEMA_CATEGORIES) {
  const schemaDir = path.join(SCHEMAS_DIR, category.path);
  
  if (!fs.existsSync(schemaDir)) {
    console.log(`⚠️  Schema directory not found: ${schemaDir}`);
    continue;
  }
  
  const files = fs.readdirSync(schemaDir).filter(f => f.endsWith('.schema.json'));
  
  if (files.length === 0) {
    console.log(`⚠️  No schema files found in: ${schemaDir}`);
    continue;
  }
  
  console.log(`📁 Processing ${category.name} (${files.length} schemas)...`);
  
  const outputFile = path.join(OUTPUT_DIR, `${category.name}.ts`);
  
  // Generate types for each schema file
  for (const file of files) {
    const schemaPath = path.join(schemaDir, file);
    const baseName = file.replace('.schema.json', '');
    
    try {
      const cmd = `npx json-schema-to-typescript "${schemaPath}" --no-additionalProperties`;
      const output = execSync(cmd, { encoding: 'utf-8' });
      
      // Append to output file
      fs.appendFileSync(
        outputFile,
        `// Generated from ${file}\n${output}\n\n`,
        { encoding: 'utf-8' }
      );
      
      console.log(`  ✅ ${baseName}`);
    } catch (error) {
      console.log(`  ❌ ${baseName}: ${error.message}`);
    }
  }
}

// Generate index file
const indexContent = `/**
 * Auto-generated TypeScript types from Finstack JSON schemas.
 * 
 * DO NOT EDIT MANUALLY - run \`npm run generate:types\` to regenerate.
 * 
 * These types are derived from the Rust structs in finstack/valuations
 * via their JSON schema definitions.
 */

export * from './calibration';
// export * from './instruments';  // Uncomment when needed
`;

fs.writeFileSync(path.join(OUTPUT_DIR, 'index.ts'), indexContent);

console.log('\n✨ Type generation complete!');
console.log(`   Output: ${OUTPUT_DIR}`);

