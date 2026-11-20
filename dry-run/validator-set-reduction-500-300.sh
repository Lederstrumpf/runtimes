#!/usr/bin/env bash
# Assertion suite for validator set 700 -> 500 -> 300 proposal, with cores
# 140 -> 100 -> 60 on chopsticks xcm fork of Kusama Asset Hub, relay and Coretime.
#
# Prerequisites (from dry-run/):
#   npm install @polkadot/api @polkadot/util @polkadot/util-crypto
#   npm install --save-dev @acala-network/chopsticks@1.6.1
#   npm install --no-save better-sqlite3@^13.1.1   # optional, for the storage cache (see below)
#
# Options:
#   --attach               Use already running forks instead of starting chopsticks
#                          (AH ws://localhost:8010, relay :8011, Coretime :8012, or pass
#                          --asset-hub/--relay/--coretime)
#   --keep-running         Leave the started chopsticks running afterwards
#   --no-report-checks     Skip the validator-set-report.mjs runs
#   --relay-block <n>      Fork the relay at block n         (default: latest)
#   --asset-hub-block <n>  Fork Asset Hub at block n         (default: latest)
#   --coretime-block <n>   Fork Coretime at block n          (default: latest)
#   --fresh-cache          Move the chopsticks storage caches to the trash before starting
#   --full / --skip-sessions / --preimage-json / --out: see validator-set-reduction-500-300.mjs --help
#
# Storage cache: each fork gets a chopsticks `db:` sqlite file in $VSR_CACHE_DIR (default
# ${TMPDIR:-/tmp}/vsr-cache), named <chain>-<block>.sqlite, or <chain>-latest.sqlite when the block
# is not pinned. chopsticks keys cached upstream storage by fork block hash, reruns only hit the cache
# for the same fork block: pin --*-block to reuse it. `save-blocks: false` keeps locally built blocks
# out of the db (with a shared db, chopsticks would load a previous run's block by number).
# chopsticks opens the db with better-sqlite3, which npm may not have installed; without it
# (in node_modules or NODE_PATH) the cache is disabled with a warning.
#
# Upstream endpoints can be overridden with space-separated lists in VSR_RELAY_ENDPOINTS,
# VSR_AH_ENDPOINTS, VSR_CT_ENDPOINTS.
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

ATTACH=0
KEEP=0
REPORTS=1
FRESH_CACHE=0
RELAY_BLOCK=""
AH_BLOCK=""
CT_BLOCK=""
AH_WS="ws://localhost:8010"
RELAY_WS="ws://localhost:8011"
CT_WS="ws://localhost:8012"
PASS=()
while [ $# -gt 0 ]; do
    case "$1" in
    --attach) ATTACH=1 ;;
    --keep-running) KEEP=1 ;;
    --no-report-checks) REPORTS=0 ;;
    --fresh-cache) FRESH_CACHE=1 ;;
    --relay-block)
        RELAY_BLOCK="$2"
        shift
        ;;
    --asset-hub-block)
        AH_BLOCK="$2"
        shift
        ;;
    --coretime-block)
        CT_BLOCK="$2"
        shift
        ;;
    --asset-hub)
        AH_WS="$2"
        shift
        ;;
    --relay)
        RELAY_WS="$2"
        shift
        ;;
    --coretime)
        CT_WS="$2"
        shift
        ;;
    --help) exec node "$SCRIPT_DIR/validator-set-reduction-500-300.mjs" --help ;;
    *) PASS+=("$1") ;;
    esac
    shift
done

# Archive endpoints
RELAY_ENDPOINTS="${VSR_RELAY_ENDPOINTS:-wss://kusama-rpc.polkadot.io wss://kusama-rpc.n.dwellir.com wss://rpc-kusama.luckyfriday.io wss://rpc.ibp.network/kusama}"
AH_ENDPOINTS="${VSR_AH_ENDPOINTS:-wss://kusama-asset-hub-rpc.polkadot.io wss://asset-hub-kusama-rpc.n.dwellir.com wss://rpc-asset-hub-kusama.luckyfriday.io wss://sys.ibp.network/asset-hub-kusama}"
CT_ENDPOINTS="${VSR_CT_ENDPOINTS:-wss://kusama-coretime-rpc.polkadot.io wss://coretime-kusama-rpc.n.dwellir.com wss://rpc-coretime-kusama.luckyfriday.io wss://sys.ibp.network/coretime-kusama}"
CACHE_DIR="${VSR_CACHE_DIR:-${TMPDIR:-/tmp}/vsr-cache}"

ws_ready() {
    node -e 'const w = new WebSocket(process.argv[1]); w.onopen = () => process.exit(0); w.onerror = () => process.exit(1); setTimeout(() => process.exit(1), 5000);' "$1" 2>/dev/null
}

# A fork is usable when it answers chain_getHeader (local) and state_getStorage of a random key
# (not cached anywhere, so chopsticks must fetch it from upstream) within 30s.
fork_alive() {
    node -e '
    const w = new WebSocket(process.argv[1]);
    const key = "0x" + [...crypto.getRandomValues(new Uint8Array(32))].map((b) => b.toString(16).padStart(2, "0")).join("");
    const fail = (why) => { console.error(`  ${process.argv[1]}: ${why}`); process.exit(1); };
    setTimeout(() => fail("no answer in 30s"), 30000);
    w.onerror = () => fail("cannot connect");
    w.onopen = () => {
      w.send(JSON.stringify({ jsonrpc: "2.0", id: 1, method: "chain_getHeader", params: [] }));
      w.send(JSON.stringify({ jsonrpc: "2.0", id: 2, method: "state_getStorage", params: [key] }));
    };
    const got = new Set();
    w.onmessage = (m) => {
      const d = JSON.parse(m.data);
      if (d.error) fail(`${d.id === 1 ? "chain_getHeader" : "state_getStorage (upstream)"}: ${JSON.stringify(d.error)}`);
      got.add(d.id);
      if (got.size === 2) process.exit(0);
    };' "$1"
}

CHOPSTICKS_PID=""
WORKDIR=""
cleanup() {
    if [ -n "$CHOPSTICKS_PID" ] && [ "$KEEP" -eq 0 ]; then
        kill -- "-$CHOPSTICKS_PID" 2>/dev/null || true
        for _ in $(seq 1 20); do
            kill -0 -- "-$CHOPSTICKS_PID" 2>/dev/null || break
            sleep 0.5
        done
        kill -KILL -- "-$CHOPSTICKS_PID" 2>/dev/null || true
        CHOPSTICKS_PID=""
    fi
}
trap cleanup EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap 'cleanup; exit 129' HUP

if [ "$ATTACH" -eq 0 ]; then
    CHOPSTICKS="$SCRIPT_DIR/node_modules/.bin/chopsticks"
    if [ ! -x "$CHOPSTICKS" ]; then
        echo "Error: chopsticks not found. From dry-run/, run: npm install --save-dev @acala-network/chopsticks@1.6.1" >&2
        exit 1
    fi
    for url in "$AH_WS" "$RELAY_WS" "$CT_WS"; do
        if ws_ready "$url"; then
            echo "Error: $url is already in use. Stop it, or pass --attach to use it." >&2
            exit 1
        fi
    done
    mkdir -p "$CACHE_DIR"
    if [ "$FRESH_CACHE" -eq 1 ]; then
        shopt -s nullglob
        old=("$CACHE_DIR"/*.sqlite*)
        shopt -u nullglob
        if [ ${#old[@]} -gt 0 ]; then
            echo "--- --fresh-cache: moving ${#old[@]} cache file(s) in $CACHE_DIR to the trash ---"
            trash-put "${old[@]}"
        fi
    fi
    WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/vsr-500-300.XXXXXX")"
    if ! (cd "$SCRIPT_DIR/node_modules/typeorm" && node -e 'require.resolve("better-sqlite3")') 2>/dev/null; then
        echo "Warning: storage cache disabled: better-sqlite3 not found (from dry-run/: npm install --no-save better-sqlite3@^13.1.1)" >&2
        CACHE_DIR=""
    fi
    write_config() { # file port block db endpoint...
        local file="$1" port="$2" block="$3" db="$4"
        shift 4
        {
            echo "endpoint:"
            for e in "$@"; do echo "  - $e"; done
            echo "mock-signature-host: true"
            echo "port: $port"
            echo "runtime-log-level: 0"
            if [ -n "$db" ]; then
                echo "db: $db"
                echo "save-blocks: false"
            fi
            if [ -n "$block" ]; then echo "block: $block"; fi
        } >"$file"
    }
    port_of() { echo "${1##*:}"; }
    cache_of() { [ -z "$CACHE_DIR" ] || echo "$CACHE_DIR/$1-${2:-latest}.sqlite"; } # chain block
    # shellcheck disable=SC2086 # endpoint lists are space-separated on purpose
    write_config "$WORKDIR/relay.yml" "$(port_of "$RELAY_WS")" "$RELAY_BLOCK" "$(cache_of kusama "$RELAY_BLOCK")" $RELAY_ENDPOINTS
    # shellcheck disable=SC2086
    write_config "$WORKDIR/asset-hub.yml" "$(port_of "$AH_WS")" "$AH_BLOCK" "$(cache_of asset-hub-kusama "$AH_BLOCK")" $AH_ENDPOINTS
    # shellcheck disable=SC2086
    write_config "$WORKDIR/coretime.yml" "$(port_of "$CT_WS")" "$CT_BLOCK" "$(cache_of coretime-kusama "$CT_BLOCK")" $CT_ENDPOINTS
    echo "--- Starting chopsticks xcm (configs and log in $WORKDIR${CACHE_DIR:+, storage cache in $CACHE_DIR}) ---"
    setsid "$CHOPSTICKS" xcm -r "$WORKDIR/relay.yml" -p "$WORKDIR/asset-hub.yml" -p "$WORKDIR/coretime.yml" \
        >"$WORKDIR/chopsticks.log" 2>&1 </dev/null &
    CHOPSTICKS_PID=$!
    for _ in $(seq 1 120); do
        if ws_ready "$AH_WS" && ws_ready "$RELAY_WS" && ws_ready "$CT_WS"; then break; fi
        if ! kill -0 "$CHOPSTICKS_PID" 2>/dev/null; then
            echo "Error: chopsticks exited; see $WORKDIR/chopsticks.log" >&2
            tail -20 "$WORKDIR/chopsticks.log" >&2
            exit 1
        fi
        sleep 5
    done
    if ! (ws_ready "$AH_WS" && ws_ready "$RELAY_WS" && ws_ready "$CT_WS"); then
        echo "Error: forks not reachable after 10 minutes; see $WORKDIR/chopsticks.log" >&2
        exit 1
    fi
    # chopsticks xcm opens the ports before it has connected the chains to each other.
    for _ in $(seq 1 60); do
        if grep -q "Connected parachains" "$WORKDIR/chopsticks.log" 2>/dev/null &&
            [ "$(grep -c "Connected relaychain" "$WORKDIR/chopsticks.log")" -ge 2 ]; then break; fi
        sleep 5
    done
fi

STATUS=0
node "$SCRIPT_DIR/validator-set-reduction-500-300.mjs" \
    --asset-hub "$AH_WS" --relay "$RELAY_WS" --coretime "$CT_WS" ${PASS[@]+"${PASS[@]}"} || STATUS=1

if [ "$REPORTS" -eq 1 ]; then
    # The report checks build relay blocks: do not run them on a dead fork or one whose upstream
    # RPC is gone (they'd fail with "WebSocket is not connected" or stall).
    alive=1
    if [ -n "$CHOPSTICKS_PID" ] && ! kill -0 "$CHOPSTICKS_PID" 2>/dev/null; then
        echo
        echo "chopsticks is no longer running" >&2
        alive=0
    else
        echo
        echo "--- Checking the forks still respond (incl. an upstream storage read) ---"
        for url in "$RELAY_WS" "$AH_WS" "$CT_WS"; do
            fork_alive "$url" || alive=0
        done
    fi
    if [ "$alive" -eq 1 ]; then
        for n in 500 300; do
            echo
            echo "=== validator-set-report.mjs --validator-count $n --expect accepted ==="
            node "$SCRIPT_DIR/validator-set-report.mjs" --relay "$RELAY_WS" --validator-count "$n" --expect accepted || STATUS=1
        done
    else
        echo "SKIPPED the validator-set-report.mjs checks: forks not responding or upstream RPC lost${WORKDIR:+ (see $WORKDIR/chopsticks.log)}" >&2
        STATUS=1
    fi
fi

echo
if [ "$STATUS" -eq 0 ]; then echo "SUITE PASSED"; else echo "SUITE FAILED"; fi
[ -n "$WORKDIR" ] && echo "chopsticks log: $WORKDIR/chopsticks.log"
exit "$STATUS"
