#!/usr/bin/env bash
# Builds the preimage for validator set 700 -> 500 -> 300 proposal, with cores
# 140 -> 100 -> 60 and n_delay_tranches 299 -> 214 -> 128, from live Kusama Asset Hub,
# relay and Coretime metadata.
#
# Prerequisites (from dry-run/):
#   npm install @polkadot/api @polkadot/util @polkadot/util-crypto
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

exec node "$SCRIPT_DIR/validator-set-reduction-500-300-preimage.mjs" "$@"
