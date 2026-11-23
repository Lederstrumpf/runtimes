import { ApiPromise, WsProvider } from "@polkadot/api";
import { blake2AsHex } from "@polkadot/util-crypto";
import {
    u8aToHex,
    hexToU8a,
    stringToHex,
    compactAddLength,
} from "@polkadot/util";
import { readFileSync, writeFileSync } from "node:fs";

// Assertion suite for validator set 700 -> 500 -> 300 proposal, with cores
// 140 -> 100 -> 60 and n_delay_tranches 299 -> 214 -> 128 on chopsticks xcm fork of
// Kusama Asset Hub, relay and Coretime.
//
// Every write goes to the local forks through chopsticks dev_* RPCs. Exits non-zero on any FAIL.
// The pinned bytes below are the output of validator-set-reduction-500-300-preimage.sh.

const PREIMAGE = {
    outerHex:
        "0x5e032802105909d1071f0005010005082f0000060200183313d60000001f0005010100b50f05082f0000060200a02802083212640032000a000000c0890100c0890100b01300000084d71701340080c3c901b013000007056b736d2d76616c696461746f722d7365742d726564756374696f6e2d6c656732c0b00700003f28020c5909b1041f0005010005082f0000060200183313800000001f0005010100b50f05082f00000602001032123c00",
    outerLen: 174,
    outerHash:
        "0xb4efb4f5228cfe09b6df3f8e4938a2e514febc7c8858f87751a65060f46f239c",
    innerLen: 172,
    innerHash:
        "0xc7817c613b1756c81bf7139de882f6ecaff4d1e8835db2ce701404ef709280e8",
    leg2Hex:
        "0x28020c5909b1041f0005010005082f0000060200183313800000001f0005010100b50f05082f00000602001032123c00",
    coretimeLeg1Hex:
        "0x2802083212640032000a000000c0890100c0890100b01300000084d71701340080c3c901b0130000",
    coretimeLeg2Hex: "0x32123c00",
    relayLeg1Hex: "0x3313d6000000",
    relayLeg2Hex: "0x331380000000",
    leg2Id: "ksm-validator-set-reduction-leg2",
    leg2After: 504000,
    priority: 63,
    coretimeParaId: 1005,
    assetHubParaId: 1000,
    leg1: { validators: 500, cores: 100, nDelayTranches: 214 },
    leg2: { validators: 300, cores: 60, nDelayTranches: 128 },
    saleCap: 52,
    // Live broker.configuration the preimage is built from.
    liveBrokerConfig: {
        advanceNotice: 10,
        interludeLength: 100800,
        leadinLength: 100800,
        regionLength: 5040,
        idealBulkProportion: 400000000,
        limitCoresOffered: null,
        renewalBump: 30000000,
        contributionTimeout: 5040,
    },
    // Live state the proposal assumes.
    liveValidatorCount: 700,
    liveNumCores: 140,
    liveMaxValidatorsPerCore: 5,
    liveNDelayTranches: 299,
};
const LEG2_ID_HEX = stringToHex(PREIMAGE.leg2Id);
const PROPOSED_BROKER_CONFIG = {
    ...PREIMAGE.liveBrokerConfig,
    limitCoresOffered: PREIMAGE.saleCap,
};
const RELAY_DEST = { v5: { parents: 1, interior: { here: null } } };
const CORETIME_DEST = {
    v5: {
        parents: 1,
        interior: { x1: [{ parachain: PREIMAGE.coretimeParaId }] },
    },
};
// Validators per delay tranche must not drop below the plan's lowest steady state, 500 / 214
// (2.336; 700 / 299 is 2.341). Cutting validators before tranches would give 500 / 299.
const MIN_PER_TRANCHE = Math.min(
    PREIMAGE.liveValidatorCount / PREIMAGE.liveNDelayTranches,
    PREIMAGE.leg1.validators / PREIMAGE.leg1.nDelayTranches,
    PREIMAGE.leg2.validators / PREIMAGE.leg2.nDelayTranches,
);

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

function usage() {
    console.error(`
Usage: dry-run/validator-set-reduction-500-300.sh [OPTIONS]
       node dry-run/validator-set-reduction-500-300.mjs [OPTIONS]   (attach to running forks)

Options:
  --asset-hub <ws-url>    Kusama Asset Hub fork            [default: ws://localhost:8010]
  --relay <ws-url>        Kusama relay fork                [default: ws://localhost:8011]
  --coretime <ws-url>     Kusama Coretime fork             [default: ws://localhost:8012]
  --full                  Also rotate relay sessions after leg 1 (100 groups of 5).
                          Adds two session-change blocks (several minutes each).
  --skip-sessions         Skip all relay session rotations (no group checks)
  --preimage-json <path>  Cross-check the pinned bytes against the --json output of
                          validator-set-reduction-500-300-preimage.sh
  --onchain-preimage      Enact the proposal from the preimage noted on the AH fork
                          instead of writing one (fails if it is not noted there)
  --out <path>            Write all checks as JSON to this file
  --help                  Show this help
`);
    process.exit(1);
}

function parseArgs() {
    const args = process.argv.slice(2);
    const opts = {
        assetHub: "ws://localhost:8010",
        relay: "ws://localhost:8011",
        coretime: "ws://localhost:8012",
        full: false,
        skipSessions: false,
        preimageJson: null,
        onchainPreimage: false,
        out: null,
    };
    for (let i = 0; i < args.length; i++) {
        switch (args[i]) {
            case "--asset-hub":
                opts.assetHub = args[++i];
                break;
            case "--relay":
                opts.relay = args[++i];
                break;
            case "--coretime":
                opts.coretime = args[++i];
                break;
            case "--full":
                opts.full = true;
                break;
            case "--skip-sessions":
                opts.skipSessions = true;
                break;
            case "--preimage-json":
                opts.preimageJson = args[++i];
                break;
            case "--onchain-preimage":
                opts.onchainPreimage = true;
                break;
            case "--out":
                opts.out = args[++i];
                break;
            case "--help":
                usage();
                break;
            default:
                console.error(`Unknown option: ${args[i]}`);
                usage();
        }
    }
    if (opts.full && opts.skipSessions) {
        console.error(
            "Error: --full and --skip-sessions are mutually exclusive",
        );
        process.exit(1);
    }
    return opts;
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

const results = [];
let failures = 0;
const t0 = Date.now();

function stable(v) {
    return JSON.stringify(v, (_, x) =>
        typeof x === "bigint" ? x.toString() : x,
    );
}
function check(name, ok, detail) {
    results.push({
        name,
        ok,
        detail,
        atSec: Math.round((Date.now() - t0) / 1000),
    });
    const d = detail === undefined ? "" : `  ${stable(detail)}`;
    console.log(`${ok ? "PASS" : "FAIL"}  ${name}${ok ? "" : d}`);
    if (!ok) failures++;
    return ok;
}
function eq(name, actual, expected) {
    return check(name, stable(actual) === stable(expected), {
        actual,
        expected,
    });
}
function section(title) {
    console.log(`\n=== ${title} ===`);
}

// ---------------------------------------------------------------------------
// Chain helpers
// ---------------------------------------------------------------------------

const LABELS = new WeakMap(); // api -> 'relay' | 'Asset Hub' | 'Coretime', for error messages
async function connect(url, label) {
    // Session-change blocks lazily fetch a lot of remote state: allow long RPC calls.
    const api = await ApiPromise.create({
        provider: new WsProvider(url, 1000, {}, 3_600_000),
        noInitWarn: true,
    });
    LABELS.set(api, label);
    return api;
}
async function head(api) {
    return (await api.rpc.chain.getHeader()).number.toNumber();
}
/** Attempts after the first, and the wait before each, when dev_newBlock leaves the head unmoved. */
const BUILD_RETRY_BACKOFF_MS = [2000, 4000, 8000];
/**
 * Build `count` blocks and check the head moved. When chopsticks loses its upstream RPC, its txpool
 * logs "build block failed" and dev_newBlock still returns the old head, so callers would loop on a
 * fork that no longer builds. Retries briefly, then throws naming the chain.
 */
async function newBlocks(api, count = 1) {
    const label = LABELS.get(api) ?? "unknown";
    const before = await head(api);
    let lastErr = null;
    for (let attempt = 0; ; attempt++) {
        try {
            await api.rpc("dev_newBlock", { count });
            lastErr = null;
        } catch (err) {
            lastErr = err; // the build itself raised (chopsticks wraps it as an RPC error)
        }
        const now = await head(api);
        if (now > before) return now;
        if (attempt >= BUILD_RETRY_BACKOFF_MS.length) {
            const why = lastErr
                ? ` (last dev_newBlock error: ${lastErr.message ?? lastErr})`
                : "";
            throw new Error(
                `${label} fork stopped building blocks (head stuck at #${now}): upstream RPC likely lost; see chopsticks.log${why}`,
            );
        }
        const ms = BUILD_RETRY_BACKOFF_MS[attempt];
        console.log(
            `  ${label}: dev_newBlock left the head at #${now}${lastErr ? ` (${lastErr.message ?? lastErr})` : ""}; retry ${attempt + 1}/${BUILD_RETRY_BACKOFF_MS.length} in ${ms / 1000}s`,
        );
        await new Promise((r) => setTimeout(r, ms));
    }
}
/** All events from block `from` up to the head, so blocks chopsticks built on its own count. */
async function eventsFrom(api, from) {
    const to = await head(api);
    const out = [];
    for (let n = from; n <= to; n++) {
        const hash = await api.rpc.chain.getBlockHash(n);
        const records = await (await api.at(hash)).query.system.events();
        for (const { event } of records) {
            out.push({
                block: n,
                name: `${event.section}.${event.method}`,
                event,
            });
        }
    }
    return out;
}
function find(events, name, pred = () => true) {
    return events.filter((e) => e.name === name && pred(e.event));
}
function names(events, re) {
    return events
        .filter((e) => re.test(e.name))
        .map((e) => `#${e.block} ${e.name} ${stable(e.event.data.toHuman())}`);
}
/** 'Ok', 'BadOrigin', 'pallet.Error' or the JSON of the error, for a DispatchResult. */
function outcome(api, result) {
    if (result.isOk) return "Ok";
    const err = result.asErr.error ?? result.asErr; // DispatchResultWithPostInfo or DispatchResult
    if (err.isBadOrigin) return "BadOrigin";
    if (err.isModule) {
        const meta = api.registry.findMetaError(err.asModule);
        return `${meta.section}.${meta.name}`;
    }
    return stable(err.toJSON());
}
async function relayNowOnAh(ah) {
    return (
        await ah.query.parachainSystem.lastRelayChainBlockNumber()
    ).toNumber();
}
/**
 * chopsticks xcm builds parachain blocks on its own when messages arrive (e.g. the relay's session
 * report to AH). A dev_setStorage that lands during such a build is lost when that block becomes
 * the head, so wait until the head has been "still" for a few seconds before writing storage.
 */
async function settle(api, quietMs = 4000) {
    // A still head can have a block in flight (AH blocks take minutes during an election).
    // dev_newBlock queues behind it, so building one block first drains it.
    await newBlocks(api, 1);
    let last = await head(api);
    let since = Date.now();
    for (;;) {
        await new Promise((r) => setTimeout(r, 1000));
        const h = await head(api);
        if (h !== last) {
            last = h;
            since = Date.now();
        } else if (Date.now() - since >= quietMs) return h;
    }
}

// ---------------------------------------------------------------------------
// Simulated referendum enactment on AH (what pallet-referenda's schedule_enactment does)
// ---------------------------------------------------------------------------

async function whitelist(ah, present) {
    const key = ah.query.whitelist.whitelistedCall.key(PREIMAGE.innerHash);
    // WhitelistedCall maps to `()`: an empty value means present, null deletes.
    await ah.rpc("dev_setStorage", [[key, present ? "0x" : null]]);
}

let onchainPreimage = false; // --onchain-preimage

/**
 * With --onchain-preimage: check the proposal preimage noted on the fork, then request it the way the
 * scheduler does when referenda schedules the enactment (pallet-preimage do_request_preimage):
 * Unrequested { ticket, len } becomes Requested { maybe_ticket: Some(ticket), count: 1, maybe_len:
 * Some(len) }, an existing Requested gets count + 1. Only the status changes, the bytes stay as noted.
 */
async function requestOnchainPreimage(ah, callHex, hash, len) {
    const bytes = await ah.query.preimage.preimageFor([hash, len]);
    const status = await ah.query.preimage.requestStatusFor(hash);
    const noted = check(
        `on-chain preimage: PreimageFor(${hash}, ${len}) and RequestStatusFor exist on the AH fork`,
        bytes.isSome && status.isSome,
        { preimageFor: bytes.isSome, requestStatusFor: status.toJSON() },
    );
    if (!noted)
        throw new Error(
            `--onchain-preimage: preimage ${hash} (len ${len}) is not noted on the Asset Hub fork; fork a block after it was noted, or run without --onchain-preimage`,
        );
    const same = eq(
        "on-chain preimage: PreimageFor bytes equal the proposal",
        bytes.unwrap().toHex(),
        callHex,
    );
    if (!same)
        throw new Error(
            "--onchain-preimage: the noted preimage differs from the proposal bytes",
        );
    const s = status.unwrap();
    const requested = s.isUnrequested
        ? {
              maybeTicket: s.asUnrequested.ticket,
              count: 1,
              maybeLen: s.asUnrequested.len,
          }
        : {
              ...s.asRequested.toJSON(),
              count: s.asRequested.count.toNumber() + 1,
          };
    const value = ah.registry.createType(s.toRawType(), {
        Requested: requested,
    });
    await ah.rpc("dev_setStorage", [
        [ah.query.preimage.requestStatusFor.key(hash), value.toHex()],
    ]);
    eq(
        `on-chain preimage: RequestStatusFor ${s.type} -> Requested, as the scheduler's request() leaves it`,
        (await ah.query.preimage.requestStatusFor(hash)).unwrap().toJSON(),
        value.toJSON(),
    );
}

/**
 * Put `callHex` into the AH scheduler as a referendum enactment with `origin`, build one block, and
 * return the scheduler "now", the block and its events. AH's scheduler clock is the relay block
 * number from `LastRelayChainBlockNumber` (the previous AH block's relay parent), so an agenda entry
 * at that number, with `IncompleteSince` there, runs in the next block.
 */
async function enact(ah, callHex, origin) {
    const len = (callHex.length - 2) / 2;
    const hash = blake2AsHex(callHex, 256);
    await settle(ah);
    const at = await relayNowOnAh(ah);
    let call;
    if (len <= 128) {
        call = { Inline: callHex };
    } else if (onchainPreimage && callHex === PREIMAGE.outerHex) {
        call = { Lookup: { hash, len } };
        await requestOnchainPreimage(ah, callHex, hash, len);
    } else {
        call = { Lookup: { hash, len } };
        // PreimageFor holds a length-prefixed BoundedVec<u8>: write the raw value.
        await ah.rpc("dev_setStorage", [
            [
                ah.query.preimage.preimageFor.key([hash, len]),
                u8aToHex(compactAddLength(hexToU8a(callHex))),
            ],
        ]);
        await ah.rpc("dev_setStorage", {
            Preimage: {
                RequestStatusFor: [
                    [
                        [hash],
                        {
                            Requested: {
                                maybeTicket: null,
                                count: 1,
                                maybeLen: len,
                            },
                        },
                    ],
                ],
            },
        });
    }
    const existing = (await ah.query.scheduler.agenda(at)).toJSON() || [];
    await ah.rpc("dev_setStorage", {
        Scheduler: {
            Agenda: [
                [
                    [at],
                    [
                        ...existing,
                        {
                            maybeId: null,
                            priority: PREIMAGE.priority,
                            call,
                            maybePeriodic: null,
                            origin,
                        },
                    ],
                ],
            ],
            IncompleteSince: at,
        },
    });
    const written = (await ah.query.scheduler.agenda(at)).toJSON() || [];
    if (written.length !== existing.length + 1)
        throw new Error(
            `agenda write at ${at} did not stick (a block was built concurrently)`,
        );
    const from = (await head(ah)) + 1;
    await newBlocks(ah, 1);
    const events = await eventsFrom(ah, from);
    return { at, block: from, events };
}

function dispatched(events, pred = () => true) {
    return find(events, "scheduler.Dispatched", pred);
}

// ---------------------------------------------------------------------------
// Coretime and relay views
// ---------------------------------------------------------------------------

async function brokerConfig(ct) {
    return (await ct.query.broker.configuration()).toJSON();
}

async function relayConfigView(relay) {
    const active = (await relay.query.configuration.activeConfig()).toJSON();
    const pending = (await relay.query.configuration.pendingConfigs()).toJSON();
    return {
        session: (await relay.query.session.currentIndex()).toNumber(),
        sharedSession: (
            await relay.query.parasShared.currentSessionIndex()
        ).toNumber(),
        active,
        pending,
    };
}

async function groupsView(relay) {
    const cfg = (await relay.query.configuration.activeConfig()).toJSON();
    const groups = await relay.query.paraScheduler.validatorGroups();
    const sizes = {};
    for (const g of groups) sizes[g.length] = (sizes[g.length] || 0) + 1;
    return {
        relayBlock: await head(relay),
        session: (await relay.query.session.currentIndex()).toNumber(),
        sessionValidators: (await relay.query.session.validators()).length,
        numCores: cfg.schedulerParams.numCores,
        maxValidatorsPerCore: cfg.schedulerParams.maxValidatorsPerCore,
        nDelayTranches: cfg.nDelayTranches,
        groups: groups.length,
        groupSizes: sizes,
        pending: (await relay.query.configuration.pendingConfigs())
            .toJSON()
            .map(([s, c]) => ({
                session: s,
                numCores: c.schedulerParams.numCores,
                nDelayTranches: c.nDelayTranches,
            })),
    };
}

/** Config fields the proposal changes on the relay. */
function withoutChanged(cfg) {
    return {
        ...cfg,
        nDelayTranches: undefined,
        schedulerParams: { ...cfg.schedulerParams, numCores: undefined },
    };
}

/**
 * Build blocks and check each hop for one leg: AH -> relay (UMP) for n_delay_tranches, and
 * AH -> Coretime (HRMP) -> relay (UMP) for num_cores. The two may land in different relay blocks;
 * pending configs for the same session merge.
 */
async function checkPropagation(
    tag,
    { ct, relay },
    { cores, nDelayTranches },
    expectConfig,
    marks = {},
) {
    section(`${tag}: Coretime`);
    const ctFrom = marks.ct ?? (await head(ct)) + 1;
    let ctEvents = [];
    for (let i = 0; i < 3; i++) {
        await newBlocks(ct, 1);
        ctEvents = await eventsFrom(ct, ctFrom);
        if (find(ctEvents, "broker.CoreCountRequested").length) break;
    }
    check(
        `${tag}: Coretime processed the XCM from AH (messageQueue.Processed, success, from sibling ${PREIMAGE.assetHubParaId})`,
        find(
            ctEvents,
            "messageQueue.Processed",
            (e) =>
                e.data.success.isTrue &&
                e.data.origin.toJSON()?.sibling === PREIMAGE.assetHubParaId,
        ).length === 1,
        names(
            ctEvents,
            /^(messageQueue|broker|utility|polkadotXcm|xcmpQueue)\./,
        ),
    );
    const ccr = find(ctEvents, "broker.CoreCountRequested").map((e) =>
        e.event.data.coreCount.toNumber(),
    );
    eq(`${tag}: broker.CoreCountRequested(${cores})`, ccr, [cores]);
    check(
        `${tag}: Coretime sent the request upward (parachainSystem.UpwardMessageSent)`,
        find(ctEvents, "parachainSystem.UpwardMessageSent").length >= 1,
    );
    check(
        `${tag}: no utility.BatchInterrupted on Coretime`,
        find(ctEvents, "utility.BatchInterrupted").length === 0,
    );
    eq(`${tag}: broker.configuration`, await brokerConfig(ct), expectConfig);

    section(`${tag}: relay`);
    // The AH -> relay message may already be in a relay block built before this point.
    const before = marks.relayConfig ?? (await relayConfigView(relay));
    const rFrom = marks.relay ?? (await head(relay)) + 1;
    const umpFrom = (events, para) =>
        find(
            events,
            "messageQueue.Processed",
            (e) => e.data.origin.toJSON()?.ump?.para === para,
        );
    let rEvents = [];
    for (let i = 0; i < 4; i++) {
        await newBlocks(relay, 1);
        rEvents = await eventsFrom(relay, rFrom);
        if (
            umpFrom(rEvents, PREIMAGE.assetHubParaId).length &&
            umpFrom(rEvents, PREIMAGE.coretimeParaId).length
        )
            break;
    }
    // AH and Coretime may send other UMP in the same blocks (staking and broker traffic), so
    // require at least one from each, all successful. PendingConfigs below checks that ours landed.
    for (const [who, para] of [
        ["AH's", PREIMAGE.assetHubParaId],
        ["the broker's", PREIMAGE.coretimeParaId],
    ]) {
        const ump = umpFrom(rEvents, para);
        check(
            `${tag}: relay processed ${who} UMP (messageQueue.Processed from para ${para}, all success)`,
            ump.length >= 1 && ump.every((e) => e.event.data.success.isTrue),
            names(rEvents, /^(messageQueue|coretime|configuration)\./),
        );
    }
    const after = await relayConfigView(relay);
    const last = after.pending[after.pending.length - 1];
    check(`${tag}: relay has a pending config`, !!last, after.pending);
    if (last) {
        eq(
            `${tag}: pending num_cores / n_delay_tranches`,
            [last[1].schedulerParams.numCores, last[1].nDelayTranches],
            [cores, nDelayTranches],
        );
        eq(
            `${tag}: pending config applies at session + 2`,
            last[0],
            after.sharedSession + 2,
        );
        // If the two messages straddled a session change there are two new entries; each may
        // carry either change, so compare every entry with the rest masked.
        const base = before.pending.length
            ? before.pending[before.pending.length - 1][1]
            : before.active;
        const oldCores = base.schedulerParams.numCores;
        const oldTranches = base.nDelayTranches;
        check(
            `${tag}: pending configs differ from the previous one only in num_cores and n_delay_tranches`,
            after.pending.every(
                ([, c]) =>
                    stable(withoutChanged(c)) ===
                        stable(withoutChanged(base)) &&
                    [oldCores, cores].includes(c.schedulerParams.numCores) &&
                    [oldTranches, nDelayTranches].includes(c.nDelayTranches),
            ),
            { base, pending: after.pending },
        );
    }
    check(
        `${tag}: active num_cores and n_delay_tranches unchanged until the session change`,
        after.active.schedulerParams.numCores ===
            before.active.schedulerParams.numCores &&
            after.active.nDelayTranches === before.active.nDelayTranches,
        {
            before: [
                before.active.schedulerParams.numCores,
                before.active.nDelayTranches,
            ],
            after: [
                after.active.schedulerParams.numCores,
                after.active.nDelayTranches,
            ],
        },
    );
    return { pendingAt: last?.[0] };
}

// ---------------------------------------------------------------------------
// Relay sessions
// ---------------------------------------------------------------------------

/**
 * Advance the relay fork to the next session. BABE ends a session at the first block whose slot is
 * at least one epoch past the current epoch's start (genesis_slot + epoch_index * duration, with no
 * skipped epochs). Building an epoch of blocks takes ~20 min on a fork, so shift `Babe.GenesisSlot`
 * back just enough that the next block (slot + 1) starts the next epoch. BABE then computes
 * epoch_index + 1 as on a normal boundary, pallet-session rotates in that block, and later epochs
 * stay consistent with the shifted genesis slot. dev_timeTravel isn't used: on Kusama it pushed
 * BABE's epoch index to ~3e9 and the fork never rotated. Falls back to building blocks if the shift
 * doesn't rotate.
 */
async function rotateSession(relay) {
    const s0 = (await relay.query.session.currentIndex()).toNumber();
    const skipped = (await relay.query.babe.skippedEpochs()).toJSON();
    const epoch = await relay.call.babeApi.currentEpoch();
    const startSlot = epoch.startSlot.toBigInt();
    const duration = epoch.duration.toBigInt();
    const genesisSlot = (await relay.query.babe.genesisSlot()).toBigInt();
    const epochIndex = (await relay.query.babe.epochIndex()).toBigInt();
    const currentSlot = (await relay.query.babe.currentSlot()).toBigInt();
    const started = Date.now();
    if (
        skipped.length === 0 &&
        startSlot === genesisSlot + epochIndex * duration
    ) {
        const shift = startSlot + duration - (currentSlot + 1n);
        if (shift > 0n) {
            await relay.rpc("dev_setStorage", {
                Babe: { GenesisSlot: (genesisSlot - shift).toString() },
            });
        }
    } else {
        console.log(
            `  BABE has skipped epochs or an unexpected start slot; building blocks instead`,
        );
    }
    let built = 0;
    for (;;) {
        await newBlocks(relay, 1);
        built++;
        if ((await relay.query.session.currentIndex()).toNumber() > s0) break;
        if (built > Number(duration) + 10)
            throw new Error(`session did not rotate after ${built} blocks`);
    }
    const newIndex = (await relay.query.babe.epochIndex()).toBigInt();
    check(
        `relay session ${s0} -> ${s0 + 1}: BABE epoch index advanced by exactly 1`,
        newIndex === epochIndex + 1n,
        { before: epochIndex, after: newIndex, blocks: built },
    );
    const secs = Math.round((Date.now() - started) / 1000);
    console.log(
        `  rotated relay session ${s0} -> ${s0 + 1} (${built} block(s), ${secs}s)`,
    );
    return { from: s0, built, secs };
}

/**
 * Stand-in for the AH election export: queue a validator set of size `n` (the first `n` current
 * validators, which have session keys) in StakingAhClient.ValidatorSet. The ah-client hands it to
 * pallet-session at the next rotation, so it becomes active one session after that.
 */
async function queueValidatorSet(relay, n) {
    const vals = (await relay.query.session.validators())
        .map((v) => v.toString())
        .slice(0, n);
    const current = (await relay.query.stakingAhClient.validatorSet()).toJSON();
    const id = Array.isArray(current)
        ? current[0] + 1
        : (await relay.query.session.currentIndex()).toNumber();
    await relay.rpc("dev_setStorage", {
        StakingAhClient: { ValidatorSet: [id, vals] },
    });
    check(
        `queued a ${n}-validator set in StakingAhClient.ValidatorSet`,
        (await relay.query.stakingAhClient.validatorSet()).isSome &&
            vals.length === n,
        { n: vals.length },
    );
}

async function checkBrokerNotified(ct, cores, tag) {
    const from = (await head(ct)) + 1;
    let inbox = null;
    let status = null;
    for (let i = 0; i < 2; i++) {
        await newBlocks(ct, 1);
        inbox = (await ct.query.broker.coreCountInbox()).toJSON();
        status = (await ct.query.broker.status()).toJSON();
        if (inbox === cores || status?.coreCount === cores) break;
    }
    const evs = await eventsFrom(ct, from);
    check(
        `${tag}: relay notified the broker of ${cores} cores (coreCountInbox or status.coreCount)`,
        inbox === cores || status?.coreCount === cores,
        {
            inbox,
            statusCoreCount: status?.coreCount,
            events: names(evs, /^broker\./),
        },
    );
}

/**
 * Rotate until the pending config and the queued validator set are both active, checking the
 * groups and n_delay_tranches after every rotation. While the old (larger) set is active, the
 * groups stay at `oldValidators / 5` groups of 5 whatever num_cores is (cores are cut first, so
 * no capacity is lost early). Validators per delay tranche must never drop below MIN_PER_TRANCHE.
 */
async function sessionsAfterLeg(
    tag,
    { relay, ct },
    { pendingAt, oldValidators, newValidators, cores, nDelayTranches },
) {
    section(`${tag}: relay sessions`);
    const sQueue = (await relay.query.session.currentIndex()).toNumber();
    const oldTranches = (await relay.query.configuration.activeConfig())
        .nDelayTranches.toNumber();
    await queueValidatorSet(relay, newValidators);
    const target = Math.max(pendingAt ?? sQueue, sQueue + 2);
    let notified = false;
    for (;;) {
        const r = await rotateSession(relay);
        const v = await groupsView(relay);
        console.log(`  ${stable(v)}`);
        results.push({
            name: `${tag}: session view`,
            ok: true,
            detail: { ...v, rotation: r },
        });
        const perTranche = v.sessionValidators / v.nDelayTranches;
        check(
            `${tag}: session ${v.session}: validators per delay tranche ${v.sessionValidators}/${v.nDelayTranches} >= ${MIN_PER_TRANCHE.toFixed(3)}`,
            perTranche >= MIN_PER_TRANCHE,
            { perTranche, min: MIN_PER_TRANCHE },
        );
        eq(
            `${tag}: session ${v.session}: n_delay_tranches ${v.session >= pendingAt ? nDelayTranches : oldTranches}`,
            v.nDelayTranches,
            v.session >= pendingAt ? nDelayTranches : oldTranches,
        );
        if (v.numCores === cores && !notified) {
            await checkBrokerNotified(ct, cores, tag);
            notified = true;
        }
        if (v.session < target) {
            eq(
                `${tag}: session ${v.session}: ${oldValidators} validators still active`,
                v.sessionValidators,
                oldValidators,
            );
            eq(
                `${tag}: session ${v.session}: ${oldValidators / 5} groups of 5 (num_cores ${v.numCores})`,
                v.groupSizes,
                { 5: oldValidators / 5 },
            );
            continue;
        }
        eq(
            `${tag}: session ${v.session}: num_cores ${cores} and n_delay_tranches ${nDelayTranches} active`,
            [v.numCores, v.nDelayTranches],
            [cores, nDelayTranches],
        );
        eq(
            `${tag}: session ${v.session}: ${newValidators} validators active`,
            v.sessionValidators,
            newValidators,
        );
        eq(`${tag}: session ${v.session}: ${cores} groups of 5`, v.groupSizes, {
            5: cores,
        });
        break;
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
    const opts = parseArgs();
    onchainPreimage = opts.onchainPreimage;
    console.log("--- Connecting ---");
    const [ah, relay, ct] = await Promise.all([
        connect(opts.assetHub, "Asset Hub"),
        connect(opts.relay, "relay"),
        connect(opts.coretime, "Coretime"),
    ]);
    const chains = {};
    for (const [k, api] of Object.entries({ ah, relay, ct })) {
        chains[k] = {
            chain: (await api.rpc.system.chain()).toString(),
            spec: api.runtimeVersion.specVersion.toNumber(),
            block: await head(api),
        };
    }
    console.log(stable(chains));
    results.push({ name: "forks", ok: true, detail: chains });

    // ---- Static: the pinned bytes, decoded with the forks' own metadata ----
    section("Encoding");
    if (opts.preimageJson) {
        const pj = JSON.parse(readFileSync(opts.preimageJson, "utf8"));
        for (const [field, actual, expected] of [
            ["proposal.hex", pj.proposal.hex, PREIMAGE.outerHex],
            ["proposal.len", pj.proposal.len, PREIMAGE.outerLen],
            ["proposal.blake2_256", pj.proposal.blake2_256, PREIMAGE.outerHash],
            ["inner.len", pj.inner.len, PREIMAGE.innerLen],
            ["inner.blake2_256", pj.inner.blake2_256, PREIMAGE.innerHash],
            ["leg2.hex", pj.leg2.hex, PREIMAGE.leg2Hex],
            ["coretimeLeg1.hex", pj.coretimeLeg1.hex, PREIMAGE.coretimeLeg1Hex],
            ["coretimeLeg2.hex", pj.coretimeLeg2.hex, PREIMAGE.coretimeLeg2Hex],
            ["relayLeg1.hex", pj.relayLeg1.hex, PREIMAGE.relayLeg1Hex],
            ["relayLeg2.hex", pj.relayLeg2.hex, PREIMAGE.relayLeg2Hex],
        ])
            eq(
                `--preimage-json ${field} equals the pinned value`,
                actual,
                expected,
            );
    }
    eq("outer length", (PREIMAGE.outerHex.length - 2) / 2, PREIMAGE.outerLen);
    eq(
        "outer blake2-256 (preimage hash)",
        blake2AsHex(PREIMAGE.outerHex, 256),
        PREIMAGE.outerHash,
    );
    const outer = ah.registry.createType("Call", PREIMAGE.outerHex);
    eq(
        "outer is whitelist.dispatchWhitelistedCallWithPreimage",
        `${outer.section}.${outer.method}`,
        "whitelist.dispatchWhitelistedCallWithPreimage",
    );
    const inner = outer.args[0];
    eq("inner length", inner.encodedLength, PREIMAGE.innerLen);
    eq(
        "inner blake2-256 (whitelist hash)",
        blake2AsHex(inner.toU8a(), 256),
        PREIMAGE.innerHash,
    );
    eq("outer re-encodes byte for byte", outer.toHex(), PREIMAGE.outerHex);
    const batch = inner.args[0];
    eq(
        "inner is utility.batchAll of 4",
        [`${inner.section}.${inner.method}`, batch.length],
        ["utility.batchAll", 4],
    );
    eq(
        "call 1: staking.setValidatorCount(500)",
        [`${batch[0].section}.${batch[0].method}`, batch[0].args[0].toNumber()],
        ["staking.setValidatorCount", PREIMAGE.leg1.validators],
    );
    // An XCM send in the proposal's shape: V5 dest, UnpaidExecution{Unlimited,None},
    // Transact{Superuser,None,call}. Returns the transported call's hex.
    const checkSend = (name, send, dest) => {
        eq(
            `${name}: polkadotXcm.send to ${stable(dest)}`,
            [`${send.section}.${send.method}`, send.args[0].toJSON()],
            ["polkadotXcm.send", dest],
        );
        const xcm = send.args[1].asV5;
        eq(
            `${name}: XCM is UnpaidExecution{Unlimited,None}, Transact{Superuser,None}`,
            [
                xcm.length,
                xcm[0].toJSON(),
                xcm[1].asTransact.originKind.toString(),
                xcm[1].asTransact.fallbackMaxWeight.toJSON(),
            ],
            [
                2,
                {
                    unpaidExecution: {
                        weightLimit: { unlimited: null },
                        checkOrigin: null,
                    },
                },
                "Superuser",
                null,
            ],
        );
        return u8aToHex(xcm[1].asTransact.call.encoded.toU8a(true));
    };
    eq(
        "call 2: Transact carries the relay leg-1 call",
        checkSend("call 2", batch[1], RELAY_DEST),
        PREIMAGE.relayLeg1Hex,
    );
    eq(
        "call 3: Transact carries the Coretime leg-1 call",
        checkSend("call 3", batch[2], CORETIME_DEST),
        PREIMAGE.coretimeLeg1Hex,
    );
    const sched = batch[3];
    eq(
        "call 4: scheduler.scheduleNamedAfter(id, 504000, None, 63, leg2)",
        [
            `${sched.section}.${sched.method}`,
            sched.args[0].toHex(),
            sched.args[1].toNumber(),
            sched.args[2].toJSON(),
            sched.args[3].toNumber(),
            sched.args[4].toHex(),
        ],
        [
            "scheduler.scheduleNamedAfter",
            LEG2_ID_HEX,
            PREIMAGE.leg2After,
            null,
            PREIMAGE.priority,
            PREIMAGE.leg2Hex,
        ],
    );
    const leg2Batch = sched.args[4].args[0];
    eq(
        "leg 2: utility.batchAll[staking.setValidatorCount(300), send relay, send Coretime]",
        [
            `${sched.args[4].section}.${sched.args[4].method}`,
            leg2Batch.length,
            `${leg2Batch[0].section}.${leg2Batch[0].method}`,
            leg2Batch[0].args[0].toNumber(),
        ],
        [
            "utility.batchAll",
            3,
            "staking.setValidatorCount",
            PREIMAGE.leg2.validators,
        ],
    );
    eq(
        "leg 2 call 2: Transact carries the relay leg-2 call",
        checkSend("leg 2 call 2", leg2Batch[1], RELAY_DEST),
        PREIMAGE.relayLeg2Hex,
    );
    eq(
        "leg 2 call 3: Transact carries the Coretime leg-2 call",
        checkSend("leg 2 call 3", leg2Batch[2], CORETIME_DEST),
        PREIMAGE.coretimeLeg2Hex,
    );
    for (const [leg, hex, n] of [
        [1, PREIMAGE.relayLeg1Hex, PREIMAGE.leg1.nDelayTranches],
        [2, PREIMAGE.relayLeg2Hex, PREIMAGE.leg2.nDelayTranches],
    ]) {
        const c = relay.registry.createType("Call", hex);
        eq(
            `relay leg ${leg}: configuration.setNDelayTranches(${n})`,
            [`${c.section}.${c.method}`, c.args[0].toNumber(), c.toHex()],
            ["configuration.setNDelayTranches", n, hex],
        );
    }
    const ctLeg1 = ct.registry.createType("Call", PREIMAGE.coretimeLeg1Hex);
    const ctCalls = ctLeg1.args[0];
    eq(
        "Coretime leg 1: utility.batchAll[broker.requestCoreCount(100), broker.configure]",
        [
            `${ctLeg1.section}.${ctLeg1.method}`,
            ctCalls.map((c) => `${c.section}.${c.method}`),
            ctCalls[0].args[0].toNumber(),
        ],
        [
            "utility.batchAll",
            ["broker.requestCoreCount", "broker.configure"],
            PREIMAGE.leg1.cores,
        ],
    );
    eq(
        "Coretime leg 1: configure = live snapshot with limitCoresOffered 52",
        ctCalls[1].args[0].toJSON(),
        PROPOSED_BROKER_CONFIG,
    );
    const ctLeg2 = ct.registry.createType("Call", PREIMAGE.coretimeLeg2Hex);
    eq(
        "Coretime leg 2: broker.requestCoreCount(60)",
        [`${ctLeg2.section}.${ctLeg2.method}`, ctLeg2.args[0].toNumber()],
        ["broker.requestCoreCount", PREIMAGE.leg2.cores],
    );

    // ---- Preconditions: the live state the preimage was built for ----
    section("Preconditions (live state the proposal assumes)");
    eq(
        "AH staking.validatorCount",
        (await ah.query.staking.validatorCount()).toNumber(),
        PREIMAGE.liveValidatorCount,
    );
    eq(
        "Coretime broker.configuration has not drifted from the preimage's snapshot",
        await brokerConfig(ct),
        PREIMAGE.liveBrokerConfig,
    );
    const rc0 = await relayConfigView(relay);
    eq(
        "relay active num_cores / max_validators_per_core",
        [
            rc0.active.schedulerParams.numCores,
            rc0.active.schedulerParams.maxValidatorsPerCore,
        ],
        [PREIMAGE.liveNumCores, PREIMAGE.liveMaxValidatorsPerCore],
    );
    eq(
        "relay active n_delay_tranches",
        rc0.active.nDelayTranches,
        PREIMAGE.liveNDelayTranches,
    );
    eq("relay has no pending config", rc0.pending, []);
    eq(
        "AH Scheduler.Lookup[leg2] is free",
        (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON(),
        null,
    );

    // ---- Negative cases (state is unchanged afterwards, so leg 1 runs on the same fork) ----
    section(
        "Negative: StakingAdmin enacting the inner batch (staking-admin track, no whitelist)",
    );
    {
        await whitelist(ah, false);
        const innerHex = inner.toHex();
        const r = await enact(ah, innerHex, { Origins: "StakingAdmin" });
        const d = dispatched(r.events);
        eq(
            "scheduler.Dispatched result",
            d.map((e) => outcome(ah, e.event.data.result)),
            ["BadOrigin"],
        );
        eq(
            "validatorCount unchanged",
            (await ah.query.staking.validatorCount()).toNumber(),
            PREIMAGE.liveValidatorCount,
        );
        eq(
            "nothing staged",
            (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON(),
            null,
        );
        check("no XCM sent", find(r.events, "polkadotXcm.Sent").length === 0);
    }
    section(
        "Negative: StakingAdmin dispatching the whitelisted wrapper (call is whitelisted)",
    );
    {
        await whitelist(ah, true);
        const r = await enact(ah, PREIMAGE.outerHex, {
            Origins: "StakingAdmin",
        });
        eq(
            "scheduler.Dispatched result",
            dispatched(r.events).map((e) => outcome(ah, e.event.data.result)),
            ["BadOrigin"],
        );
        eq(
            "validatorCount unchanged",
            (await ah.query.staking.validatorCount()).toNumber(),
            PREIMAGE.liveValidatorCount,
        );
        check(
            "whitelist entry not consumed",
            (await ah.query.whitelist.whitelistedCall(PREIMAGE.innerHash))
                .isSome,
        );
    }
    section("Negative: WhitelistedCaller without the Fellowship whitelist");
    {
        await whitelist(ah, false);
        const r = await enact(ah, PREIMAGE.outerHex, {
            Origins: "WhitelistedCaller",
        });
        eq(
            "scheduler.Dispatched result",
            dispatched(r.events).map((e) => outcome(ah, e.event.data.result)),
            ["whitelist.CallIsNotWhitelisted"],
        );
        eq(
            "validatorCount unchanged",
            (await ah.query.staking.validatorCount()).toNumber(),
            PREIMAGE.liveValidatorCount,
        );
        eq(
            "nothing staged",
            (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON(),
            null,
        );
    }

    // ---- Leg 1 ----
    section("Leg 1: WhitelistedCaller enacts the proposal");
    await whitelist(ah, true);
    const leg1Marks = {
        ct: (await head(ct)) + 1,
        relay: (await head(relay)) + 1,
        relayConfig: await relayConfigView(relay),
    };
    const leg1 = await enact(ah, PREIMAGE.outerHex, {
        Origins: "WhitelistedCaller",
    });
    const when = leg1.at + PREIMAGE.leg2After + 1;
    console.log(
        `  enacted at AH #${leg1.block}, scheduler now (relay) ${leg1.at}, expected leg-2 slot ${when}`,
    );
    eq(
        "scheduler.Dispatched (referendum enactment) Ok",
        dispatched(leg1.events).map((e) => outcome(ah, e.event.data.result)),
        ["Ok"],
    );
    eq(
        "whitelist.WhitelistedCallDispatched(inner hash) Ok",
        find(leg1.events, "whitelist.WhitelistedCallDispatched").map((e) => [
            e.event.data.callHash.toHex(),
            outcome(ah, e.event.data.result),
        ]),
        [[PREIMAGE.innerHash, "Ok"]],
    );
    check(
        "utility.BatchCompleted, no BatchInterrupted",
        find(leg1.events, "utility.BatchCompleted").length === 1 &&
            find(leg1.events, "utility.BatchInterrupted").length === 0,
        names(leg1.events, /^utility\./),
    );
    eq(
        "polkadotXcm.Sent to the relay, then to Coretime",
        find(leg1.events, "polkadotXcm.Sent").map((e) =>
            e.event.data.destination.toJSON(),
        ),
        [RELAY_DEST.v5, CORETIME_DEST.v5],
    );
    const scheduled = find(leg1.events, "scheduler.Scheduled").map((e) =>
        e.event.data.when.toNumber(),
    );
    eq("scheduler.Scheduled at now + 504000 + 1", scheduled, [when]);
    eq(
        "AH validatorCount = 500",
        (await ah.query.staking.validatorCount()).toNumber(),
        PREIMAGE.leg1.validators,
    );
    check(
        "whitelist entry consumed",
        (await ah.query.whitelist.whitelistedCall(PREIMAGE.innerHash)).isNone,
    );
    const lookup = (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON();
    check(
        "Scheduler.Lookup[leg2] = (when, i)",
        Array.isArray(lookup) && lookup[0] === when,
        lookup,
    );
    const idx = Array.isArray(lookup) ? lookup[1] : 0;
    const agenda = (await ah.query.scheduler.agenda(when)).toJSON();
    const task = agenda?.[idx];
    eq("Scheduler.Agenda[when][i]", task, {
        maybeId: LEG2_ID_HEX,
        priority: PREIMAGE.priority,
        call: { inline: PREIMAGE.leg2Hex },
        maybePeriodic: null,
        origin: { system: { root: null } },
    });

    const leg1Prop = await checkPropagation(
        "leg 1",
        { ct, relay },
        PREIMAGE.leg1,
        PROPOSED_BROKER_CONFIG,
        leg1Marks,
    );

    if (opts.full) {
        await sessionsAfterLeg(
            "leg 1",
            { relay, ct },
            {
                pendingAt: leg1Prop.pendingAt,
                oldValidators: PREIMAGE.liveValidatorCount,
                newValidators: PREIMAGE.leg1.validators,
                cores: PREIMAGE.leg1.cores,
                nDelayTranches: PREIMAGE.leg1.nDelayTranches,
            },
        );
    }

    // ---- Leg 2: move the staged task to the current slot (stand-in for ~504k relay blocks) ----
    section("Leg 2: the staged task runs");
    let leg2Events = [];
    await settle(ct);
    const leg2Marks = {
        ct: (await head(ct)) + 1,
        relay: (await head(relay)) + 1,
        relayConfig: await relayConfigView(relay),
    };
    for (let attempt = 1; attempt <= 3; attempt++) {
        await settle(ah);
        const [from] = (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON();
        const now = await relayNowOnAh(ah);
        const moved = task; // the entry as staged by leg 1, checked above
        await ah.rpc("dev_setStorage", {
            Scheduler: {
                Agenda:
                    from === now
                        ? [[[now], [moved]]]
                        : [
                              [[from], null],
                              [[now], [moved]],
                          ],
                Lookup: [[[LEG2_ID_HEX], [now, 0]]],
                IncompleteSince: now,
            },
        });
        const stuck = (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON();
        const stuckTask = (await ah.query.scheduler.agenda(now)).toJSON()?.[0];
        console.log(
            `  attempt ${attempt}: moved the task unchanged from relay slot ${from} to ${now} (read back ${stable(stuck)})`,
        );
        if (
            stable(stuck) !== stable([now, 0]) ||
            stable(stuckTask) !== stable(task)
        ) {
            console.log(`  attempt ${attempt}: write did not stick, retrying`);
            continue;
        }
        if (attempt === 1)
            results.push({
                name: "leg 2 task moved",
                ok: true,
                detail: { from: when, to: now },
            });
        const ahFrom = (await head(ah)) + 1;
        await newBlocks(ah, 1);
        leg2Events = await eventsFrom(ah, ahFrom);
        if (
            dispatched(leg2Events).length ||
            (await ah.query.scheduler.lookup(LEG2_ID_HEX)).isNone
        )
            break;
        console.log(
            `  attempt ${attempt}: not dispatched (concurrent block?), retrying`,
        );
    }
    eq(
        "scheduler.Dispatched(leg2 id) Ok, origin Root",
        dispatched(leg2Events).map((e) => [
            e.event.data.id.isSome ? e.event.data.id.unwrap().toHex() : null,
            outcome(ah, e.event.data.result),
        ]),
        [[LEG2_ID_HEX, "Ok"]],
    );
    check(
        "utility.BatchCompleted, no BatchInterrupted",
        find(leg2Events, "utility.BatchCompleted").length === 1 &&
            find(leg2Events, "utility.BatchInterrupted").length === 0,
        names(leg2Events, /^utility\./),
    );
    eq(
        "polkadotXcm.Sent to the relay, then to Coretime",
        find(leg2Events, "polkadotXcm.Sent").map((e) =>
            e.event.data.destination.toJSON(),
        ),
        [RELAY_DEST.v5, CORETIME_DEST.v5],
    );
    eq(
        "AH validatorCount = 300",
        (await ah.query.staking.validatorCount()).toNumber(),
        PREIMAGE.leg2.validators,
    );
    eq(
        "Scheduler.Lookup[leg2] cleared",
        (await ah.query.scheduler.lookup(LEG2_ID_HEX)).toJSON(),
        null,
    );

    const leg2Prop = await checkPropagation(
        "leg 2",
        { ct, relay },
        PREIMAGE.leg2,
        PROPOSED_BROKER_CONFIG,
        leg2Marks,
    );

    if (!opts.skipSessions) {
        const activeValidators = (await relay.query.session.validators())
            .length;
        await sessionsAfterLeg(
            "leg 2",
            { relay, ct },
            {
                pendingAt: leg2Prop.pendingAt,
                oldValidators: activeValidators,
                newValidators: PREIMAGE.leg2.validators,
                cores: PREIMAGE.leg2.cores,
                nDelayTranches: PREIMAGE.leg2.nDelayTranches,
            },
        );
    }

    await Promise.all([ah.disconnect(), relay.disconnect(), ct.disconnect()]);
}

main()
    .catch((err) => {
        console.log(`\nERROR: ${err?.message ?? err}`);
        // chopsticks answers -32603 with its upstream provider's error when a fork cannot fetch state.
        if (
            /-32603: .*(WebSocket is not connected|No response received from RPC endpoint)/.test(
                String(err?.message),
            )
        ) {
            console.log(
                "ERROR: a fork lost its upstream RPC while reading state; see chopsticks.log",
            );
        }
        check("suite ran to completion", false, String(err?.stack || err));
    })
    .finally(() => {
        const opts = parseArgs();
        const passed = results.filter(
            (r) => r.ok && !/session view|forks/.test(r.name),
        ).length;
        console.log(
            `\n--- ${failures === 0 ? "ALL PASSED" : `${failures} FAILED`} (${passed} passed, ${Math.round((Date.now() - t0) / 1000)}s) ---`,
        );
        if (opts.out)
            writeFileSync(opts.out, stable(results.map((r) => r)) + "\n");
        process.exit(failures === 0 ? 0 : 1);
    });
