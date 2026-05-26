#!/usr/bin/env bash
# Dry-run a synthetic stakingAhClient.validatorSet report on a chopsticks-forked relay chain.
#
# Prerequisites:
#   - Start chopsticks in xcm mode before running this script.
#   - npm install @polkadot/api @polkadot/util @polkadot/util-crypto (from dry-run/ directory)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if ! command -v node &>/dev/null; then
  echo "Error: node is required. Install Node.js or run inside nix-shell -p nodejs" >&2
  exit 1
fi

if [ ! -d "$SCRIPT_DIR/node_modules/@polkadot/api" ]; then
  echo "Error: @polkadot/api not found. From dry-run/, run: npm install @polkadot/api @polkadot/util @polkadot/util-crypto" >&2
  exit 1
fi

exec node "$SCRIPT_DIR/validator-set-report.mjs" "$@"
