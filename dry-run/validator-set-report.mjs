import { ApiPromise, WsProvider } from '@polkadot/api';
import { blake2AsHex } from '@polkadot/util-crypto';
import { u8aToHex } from '@polkadot/util';

const MIN_VALIDATOR_SET_SIZE_KEY = '0xf3bc143b8d000789783c731662bcb641';

function usage() {
  console.error(`
Usage: dry-run/validator-set-report.sh [OPTIONS]

Options:
  --relay <ws-url>          Relay chain WS endpoint       [default: ws://localhost:8005]
  --validator-count <n>     Number of validators in report [default: 700]
  --report-id <n>           Validator set report id        [default: 1]
  --expect <result>         accepted, dropped, or any      [default: accepted]
  --help                    Show this help
`);
  process.exit(1);
}

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    relay: 'ws://localhost:8005',
    validatorCount: 700,
    reportId: 1,
    expect: 'accepted',
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--relay': opts.relay = args[++i]; break;
      case '--validator-count': opts.validatorCount = parseInt(args[++i], 10); break;
      case '--report-id': opts.reportId = parseInt(args[++i], 10); break;
      case '--expect': opts.expect = args[++i]; break;
      case '--help': usage(); break;
      default:
        console.error(`Unknown option: ${args[i]}`);
        usage();
    }
  }

  if (!Number.isInteger(opts.validatorCount) || opts.validatorCount <= 0) {
    console.error('Error: --validator-count must be a positive integer');
    process.exit(1);
  }
  if (!Number.isInteger(opts.reportId) || opts.reportId < 0) {
    console.error('Error: --report-id must be a non-negative integer');
    process.exit(1);
  }
  if (!['accepted', 'dropped', 'any'].includes(opts.expect)) {
    console.error('Error: --expect must be accepted, dropped, or any');
    process.exit(1);
  }

  return opts;
}

async function isParachain(api) {
  return !!api.query.parachainSystem;
}

async function getSchedulerBlockAndCleanup(api) {
  const incompleteSince = (await api.query.scheduler.incompleteSince()).unwrap().toNumber();
  let blockNumber = incompleteSince;
  if (await isParachain(api)) {
    const lrcbn = (await api.query.parachainSystem.lastRelayChainBlockNumber()).toNumber();
    blockNumber = Math.min(incompleteSince, lrcbn);
  }

  const entries = await api.query.scheduler.agenda.entries();
  const clearEntries = [];
  for (const [key] of entries) {
    const blockNum = key.args[0].toNumber();
    if (blockNum !== blockNumber) {
      clearEntries.push([[blockNum], []]);
    }
  }

  console.log(`Scheduler block: ${blockNumber} (incompleteSince=${incompleteSince})`);
  if (clearEntries.length > 0) {
    console.log(`Clearing:       ${clearEntries.length} existing agenda entries`);
  }

  return { blockNumber, clearEntries };
}

function accountId32(index) {
  const bytes = new Uint8Array(32);
  new DataView(bytes.buffer).setUint32(0, index, true);
  return u8aToHex(bytes);
}

function validatorSetReport(validatorCount, reportId) {
  return {
    newValidatorSet: Array.from({ length: validatorCount }, (_, index) => accountId32(index)),
    id: reportId,
    pruneUpTo: null,
    leftover: false,
  };
}

function schedulerCallFor(api, callHex) {
  const byteLen = (callHex.length - 2) / 2;
  if (byteLen <= 128) {
    return { schedulerCall: { Inline: callHex }, preimageStorage: null, byteLen };
  }

  const hash = blake2AsHex(callHex, 256);
  return {
    schedulerCall: { Lookup: { hash, len: byteLen } },
    preimageStorage: {
      requestStatusFor: [
        [[hash], { Requested: { maybeTicket: null, count: 1, maybeLen: byteLen } }],
      ],
      preimageFor: [
        [[[hash, byteLen]], u8aToHex(api.createType('Bytes', callHex).toU8a())],
      ],
    },
    byteLen,
    hash,
  };
}

function decodeLittleEndianU32(hex) {
  if (!hex || hex === '0x') return null;
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (clean.length < 8) return null;
  return Number.parseInt(
    clean.slice(6, 8) + clean.slice(4, 6) + clean.slice(2, 4) + clean.slice(0, 2),
    16,
  );
}

async function printPreState(api) {
  const mode = api.query.stakingAhClient?.mode
    ? (await api.query.stakingAhClient.mode()).toString()
    : '<stakingAhClient.mode unavailable>';
  console.log(`StakingAhClient mode: ${mode}`);

  if (api.query.staking?.minimumValidatorCount) {
    const minimumValidatorCount = await api.query.staking.minimumValidatorCount();
    console.log(`Staking.minimumValidatorCount: ${minimumValidatorCount.toString()}`);
  }

  const rawMin = await api.rpc.state.getStorage(MIN_VALIDATOR_SET_SIZE_KEY);
  const rawHex = rawMin?.toHex?.() ?? null;
  const rawValue = decodeLittleEndianU32(rawHex);
  console.log(
    `MinimumValidatorSetSize override: ${rawHex ?? '<none>'}` +
      (rawValue === null ? '' : ` (${rawValue})`),
  );
}

function printEvents(events, label) {
  console.log(`\n=== ${label} Events ===`);
  for (const record of events) {
    const { event, phase } = record;
    const important = new Set([
      'scheduler',
      'preimage',
      'stakingAhClient',
      'system',
    ]).has(event.section);
    const prefix = important ? '>>>' : '   ';
    console.log(`  ${prefix} [${phase.toString()}] ${event.section}.${event.method}`);
    if (important) {
      console.log(`        ${JSON.stringify(event.data.toHuman())}`);
    }
  }
}

async function printValidatorSet(api) {
  if (!api.query.stakingAhClient?.validatorSet) {
    console.log('ValidatorSet storage: <stakingAhClient.validatorSet unavailable>');
    return;
  }

  const maybeValidatorSet = await api.query.stakingAhClient.validatorSet();
  if (maybeValidatorSet.isNone) {
    console.log('ValidatorSet storage: None');
    return;
  }

  const [id, validators] = maybeValidatorSet.unwrap();
  console.log(`ValidatorSet storage: Some(id=${id.toString()}, validators=${validators.length})`);
}

function classify(events) {
  let accepted = false;
  let dropped = false;

  for (const { event } of events) {
    if (event.section !== 'stakingAhClient') continue;
    if (event.method === 'ValidatorSetReceived') accepted = true;
    if (event.method === 'SetTooSmallAndDropped') dropped = true;
  }

  if (accepted) return 'accepted';
  if (dropped) return 'dropped';
  return 'unknown';
}

async function main() {
  const opts = parseArgs();

  console.log('--- Connecting ---');
  const api = await ApiPromise.create({ provider: new WsProvider(opts.relay) });
  const chain = (await api.rpc.system.chain()).toString();
  console.log(`Relay:          ${opts.relay} (${chain})`);
  console.log(`Validator count: ${opts.validatorCount}`);
  console.log(`Report id:      ${opts.reportId}`);
  console.log(`Expect:         ${opts.expect}`);

  if (!api.tx.stakingAhClient?.validatorSet) {
    throw new Error('stakingAhClient.validatorSet call not found in relay metadata');
  }

  console.log('\n--- Current state ---');
  await printPreState(api);

  console.log('\n--- Preparing call ---');
  const report = validatorSetReport(opts.validatorCount, opts.reportId);
  const call = api.tx.stakingAhClient.validatorSet(report).method;
  const callHex = call.toHex();
  const { schedulerCall, preimageStorage, byteLen, hash } = schedulerCallFor(api, callHex);
  console.log(`Call:           ${call.section}.${call.method} (${byteLen} bytes)`);
  if (hash) {
    console.log(`Encoding:       Lookup`);
    console.log(`Preimage hash:  ${hash}`);
  } else {
    console.log(`Encoding:       Inline`);
  }

  console.log('\n--- Scheduling on relay ---');
  const { blockNumber, clearEntries } = await getSchedulerBlockAndCleanup(api);
  const storageOverride = {
    scheduler: {
      agenda: [
        [[blockNumber], [{ call: schedulerCall, origin: { system: 'Root' } }]],
        ...clearEntries,
      ],
      incompleteSince: blockNumber,
    },
  };
  if (preimageStorage) storageOverride.preimage = preimageStorage;

  await api.rpc('dev_setStorage', storageOverride);
  console.log(`Agenda set at:  block ${blockNumber}`);

  console.log('\n--- Producing relay block ---');
  await api.rpc('dev_newBlock');
  const header = await api.rpc.chain.getHeader();
  console.log(`New block:      #${header.number.toNumber()}`);

  const events = await api.query.system.events();
  printEvents(events, chain);

  console.log('\n--- Result ---');
  const result = classify(events);
  console.log(`Validator report: ${result}`);
  await printValidatorSet(api);

  await api.disconnect();

  if (opts.expect !== 'any' && result !== opts.expect) {
    console.error(`Expected ${opts.expect}, got ${result}`);
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('Error:', err.message || err);
  process.exit(1);
});
