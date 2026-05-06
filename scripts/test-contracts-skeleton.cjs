#!/usr/bin/env node

const { spawnSync } = require('node:child_process');

function run(command, args) {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    env: process.env,
  });

  if (result.error) {
    throw result.error;
  }

  return result.status ?? 1;
}

const check = spawnSync('forge', ['--version'], { stdio: 'ignore' });
if (check.status !== 0) {
  console.error('forge is required for test:contracts:skeleton but was not found in PATH.');
  process.exit(1);
}

const status = run('forge', ['test', '--match-path', 'test/contracts.skeleton.t.sol', '-vv']);
process.exit(status);
