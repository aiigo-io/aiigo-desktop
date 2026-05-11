#!/usr/bin/env node
/**
 * check-compute-boundary.cjs
 *
 * Enforces the compute marketplace backend boundary:
 * All direct-chain operations inside ComputingPower/ must flow through
 * the Rust/Tauri compute module — no viem, no ABI encoding, no env vars.
 *
 * Usage:  node scripts/check-compute-boundary.cjs
 *         npm run test:compute-boundary
 *
 * Exit 0 on pass, exit 1 on violation.
 */

'use strict';

const fs = require('fs');
const path = require('path');

const TARGET_DIR = path.join(__dirname, '../src/pages/Projects/ComputingPower');

// Patterns that are forbidden inside the ComputingPower/ boundary
const FORBIDDEN_PATTERNS = [
  // viem import lines
  { pattern: /from ['"]viem['"]/, label: "viem import" },
  { pattern: /from ['"]viem\//, label: "viem sub-package import" },

  // Low-level viem utilities that indicate ABI-layer leaks
  { pattern: /\bparseAbi\b/, label: "parseAbi (viem ABI encoding)" },
  { pattern: /\bencodeFunctionData\b/, label: "encodeFunctionData (viem ABI encoding)" },
  { pattern: /\bdecodeEventLog\b/, label: "decodeEventLog (viem event decoding)" },
  { pattern: /\bcreatePublicClient\b/, label: "createPublicClient (direct RPC client)" },
  { pattern: /\bcreateWalletClient\b/, label: "createWalletClient (direct wallet client)" },

  // Direct raw-transaction command (must go through compute module, not generic send)
  { pattern: /\bevm_send_transaction\b/, label: "evm_send_transaction (bypass compute module)" },

  // Direct contract reads that bypass the Rust compute module read layer
  { pattern: /\breadContract\b/, label: "readContract (direct contract read — use Rust compute module)" },

  // Env vars that belong exclusively on the Rust side
  { pattern: /VITE_AIIGO_COMPUTE_/, label: "VITE_AIIGO_COMPUTE_* env var (Rust-side only)" },
];

// Files/dirs that are allowed to be excluded from the check
const EXCLUDE_NAMES = new Set(['node_modules', '.git', 'dist', 'build']);

/** Recursively collect .ts/.tsx files under a directory */
function collectFiles(dir) {
  if (!fs.existsSync(dir)) {
    console.error(`[compute-boundary] Target directory does not exist: ${dir}`);
    process.exit(1);
  }

  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (EXCLUDE_NAMES.has(entry.name)) continue;
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectFiles(fullPath));
    } else if (/\.(ts|tsx)$/.test(entry.name)) {
      results.push(fullPath);
    }
  }
  return results;
}

const files = collectFiles(TARGET_DIR);
const violations = [];

for (const filePath of files) {
  const relPath = path.relative(process.cwd(), filePath);
  const lines = fs.readFileSync(filePath, 'utf8').split('\n');

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const line = lines[lineIdx];
    for (const { pattern, label } of FORBIDDEN_PATTERNS) {
      if (pattern.test(line)) {
        violations.push({ file: relPath, lineNumber: lineIdx + 1, line: line.trim(), label });
      }
    }
  }
}

if (violations.length === 0) {
  console.log('[compute-boundary] PASS — no forbidden patterns found in ComputingPower/');
  process.exit(0);
} else {
  console.error('[compute-boundary] FAIL — boundary violations found:\n');
  for (const v of violations) {
    console.error(`  ${v.file}:${v.lineNumber}  [${v.label}]`);
    console.error(`    ${v.line}\n`);
  }
  console.error(`${violations.length} violation(s) detected. Move all chain interactions to the Rust compute module.`);
  process.exit(1);
}
