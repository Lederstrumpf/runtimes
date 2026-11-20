import { ApiPromise, WsProvider } from "@polkadot/api";
import { blake2AsHex } from "@polkadot/util-crypto";
import { stringToU8a, u8aToHex } from "@polkadot/util";
import { writeFileSync } from "node:fs";

// Builds the preimage for validator set 700 -> 500 -> 300 proposal, with cores
// 140 -> 100 -> 60 and n_delay_tranches 299 -> 214 -> 128, from live Kusama Asset Hub,
// relay and Coretime metadata. Prints hex, length and blake2-256 of each call. Never signs
// or submits anything.
//
//   whitelist.dispatchWhitelistedCallWithPreimage(        <- proposal
//     utility.batchAll([                                  <- inner: the Fellowship whitelists its hash
//       staking.setValidatorCount(500),
//       polkadotXcm.send(relay, [UnpaidExecution, Transact(Superuser,
//         configuration.setNDelayTranches(214))]),
//       polkadotXcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//         utility.batchAll([broker.requestCoreCount(100),
//                           broker.configure(live config, limitCoresOffered: 52)]))]),
//       scheduler.scheduleNamedAfter(leg2Id, 504000, None, 63,
//         utility.batchAll([                              <- leg 2: staged call
//           staking.setValidatorCount(300),
//           polkadotXcm.send(relay, [UnpaidExecution, Transact(Superuser,
//             configuration.setNDelayTranches(128))]),
//           polkadotXcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//             broker.requestCoreCount(60))])])),
//     ]))
//
// broker.requestCoreCount makes the broker ask the relay for the new core count; the relay
// applies num_cores at session + 2 and notifies the broker back.
//
// Approval voting draws each validator's delay tranche uniformly from n_delay_tranches, so
// validators per delay tranche is validators / n_delay_tranches. Live Kusama has 700 / 299;
// the new values keep that ratio (as Polkadot #1969 did with 171 at 400). The relay applies
// n_delay_tranches at session + 2 too.

const PARAMS = {
    leg1: { validators: 500, cores: 100, nDelayTranches: 214 },
    leg2: { validators: 300, cores: 60, nDelayTranches: 128 },
    // n_delay_tranches the new values are scaled from. Refuse to build if live differs.
    liveValidators: 700,
    liveNDelayTranches: 299,
    // AH's scheduler counts relay blocks: 35 days of 6s blocks.
    leg2AfterRelayBlocks: 35 * 14_400,
    // The priority pallet-referenda uses for enactments.
    priority: 63,
    // 32-byte name, so the staged leg can be looked up or cancelled.
    leg2Id: "ksm-validator-set-reduction-leg2",
    coretimeParaId: 1005,
    // broker.configure is built from the live config. Refuse to build if it is not this.
    expectedBrokerConfig: {
        advanceNotice: 10,
        interludeLength: 100800,
        leadinLength: 100800,
        regionLength: 5040,
        idealBulkProportion: 400000000,
        limitCoresOffered: null,
        renewalBump: 30000000,
        contributionTimeout: 5040,
    },
};

function usage() {
    console.error(`
Usage: dry-run/validator-set-reduction-500-300-preimage.sh [OPTIONS]

Options:
  --asset-hub <ws-url>    Kusama Asset Hub   [default: wss://kusama-asset-hub-rpc.polkadot.io]
  --relay <ws-url>        Kusama relay       [default: wss://kusama-rpc.polkadot.io]
  --coretime <ws-url>     Kusama Coretime    [default: wss://kusama-coretime-rpc.polkadot.io]
  --json <path>           Also write the output as JSON to this file
  --help                  Show this help
`);
    process.exit(1);
}

function parseArgs() {
    const args = process.argv.slice(2);
    const opts = {
        assetHub: "wss://kusama-asset-hub-rpc.polkadot.io",
        relay: "wss://kusama-rpc.polkadot.io",
        coretime: "wss://kusama-coretime-rpc.polkadot.io",
        json: null,
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
            case "--json":
                opts.json = args[++i];
                break;
            case "--help":
                usage();
                break;
            default:
                console.error(`Unknown option: ${args[i]}`);
                usage();
        }
    }
    return opts;
}

function summarize(call) {
    const u8a = call.method.toU8a();
    return {
        hex: u8aToHex(u8a),
        len: u8a.length,
        blake2_256: blake2AsHex(u8a, 256),
    };
}

async function build(ah, relay, ct, p = PARAMS) {
    if (stringToU8a(p.leg2Id).length !== 32)
        throw new Error("leg2Id must be 32 bytes");
    for (const leg of [p.leg1, p.leg2]) {
        const scaled = Math.round(
            (p.liveNDelayTranches * leg.validators) / p.liveValidators,
        );
        if (scaled !== leg.nDelayTranches)
            throw new Error(
                `nDelayTranches ${leg.nDelayTranches} for ${leg.validators} validators is not ${scaled}`,
            );
    }

    // Relay calls, encoded with relay metadata. The tranche values are scaled from the live
    // n_delay_tranches: refuse to build if it, or any pending config, differs.
    const relayCfg = (await relay.query.configuration.activeConfig()).toJSON();
    if (relayCfg.nDelayTranches !== p.liveNDelayTranches)
        throw new Error(
            `relay n_delay_tranches differs: live ${relayCfg.nDelayTranches}, expected ${p.liveNDelayTranches}`,
        );
    for (const [s, c] of (
        await relay.query.configuration.pendingConfigs()
    ).toJSON()) {
        if (c.nDelayTranches !== p.liveNDelayTranches)
            throw new Error(
                `relay pending config for session ${s} has n_delay_tranches ${c.nDelayTranches}, expected ${p.liveNDelayTranches}`,
            );
    }
    const relayLeg1 = relay.tx.configuration.setNDelayTranches(
        p.leg1.nDelayTranches,
    );
    const relayLeg2 = relay.tx.configuration.setNDelayTranches(
        p.leg2.nDelayTranches,
    );

    // Coretime calls, encoded with Coretime metadata.
    const liveCfg = (await ct.query.broker.configuration()).unwrap().toJSON();
    for (const [k, v] of Object.entries(p.expectedBrokerConfig)) {
        if (JSON.stringify(liveCfg[k]) !== JSON.stringify(v))
            throw new Error(
                `broker.configuration.${k} differs: live ${JSON.stringify(liveCfg[k])}, expected ${JSON.stringify(v)}`,
            );
    }
    // Reserved and leased cores come first; the sale offers only the bulk cores below leg 2's count.
    const firstCore =
        (await ct.query.broker.reservations()).length +
        (await ct.query.broker.leases()).length;
    const saleCap = p.leg2.cores - firstCore;
    const capCfg = { ...liveCfg, limitCoresOffered: saleCap };
    const coretimeLeg1 = ct.tx.utility.batchAll([
        ct.tx.broker.requestCoreCount(p.leg1.cores),
        ct.tx.broker.configure(capCfg),
    ]);
    const coretimeLeg2 = ct.tx.broker.requestCoreCount(p.leg2.cores);

    // Asset Hub calls.
    const send = (dest, remote) =>
        ah.tx.polkadotXcm.send(
            { V5: dest },
            {
                V5: [
                    {
                        UnpaidExecution: {
                            weightLimit: "Unlimited",
                            checkOrigin: null,
                        },
                    },
                    {
                        Transact: {
                            originKind: "Superuser",
                            fallbackMaxWeight: null,
                            call: { encoded: remote.method.toHex() },
                        },
                    },
                ],
            },
        );
    const toRelay = (remote) => send({ parents: 1, interior: "Here" }, remote);
    const toCoretime = (remote) =>
        send(
            {
                parents: 1,
                interior: { X1: [{ Parachain: p.coretimeParaId }] },
            },
            remote,
        );
    const leg2 = ah.tx.utility.batchAll([
        ah.tx.staking.setValidatorCount(p.leg2.validators),
        toRelay(relayLeg2),
        toCoretime(coretimeLeg2),
    ]);
    const inner = ah.tx.utility.batchAll([
        ah.tx.staking.setValidatorCount(p.leg1.validators),
        toRelay(relayLeg1),
        toCoretime(coretimeLeg1),
        ah.tx.scheduler.scheduleNamedAfter(
            u8aToHex(stringToU8a(p.leg2Id)),
            p.leg2AfterRelayBlocks,
            null,
            p.priority,
            leg2,
        ),
    ]);
    const proposal = ah.tx.whitelist.dispatchWhitelistedCallWithPreimage(inner);

    return {
        params: { ...p, firstCore, saleCap },
        brokerConfig: { live: liveCfg, configured: capCfg },
        relayConfig: {
            nDelayTranches: relayCfg.nDelayTranches,
            zerothDelayTrancheWidth: relayCfg.zerothDelayTrancheWidth,
            relayVrfModuloSamples: relayCfg.relayVrfModuloSamples,
            neededApprovals: relayCfg.neededApprovals,
        },
        proposal: summarize(proposal),
        inner: summarize(inner),
        leg2: summarize(leg2),
        coretimeLeg1: summarize(coretimeLeg1),
        coretimeLeg2: summarize(coretimeLeg2),
        relayLeg1: summarize(relayLeg1),
        relayLeg2: summarize(relayLeg2),
    };
}

async function chainInfo(api, url) {
    const header = await api.rpc.chain.getHeader();
    return {
        rpc: url,
        chain: (await api.rpc.system.chain()).toString(),
        spec: api.runtimeVersion.specVersion.toNumber(),
        block: header.number.toNumber(),
        hash: header.hash.toHex(),
    };
}

async function main() {
    const opts = parseArgs();
    const [ah, relay, ct] = await Promise.all(
        [opts.assetHub, opts.relay, opts.coretime].map((url) =>
            ApiPromise.create({
                provider: new WsProvider(url),
                noInitWarn: true,
            }),
        ),
    );
    try {
        const chains = {
            assetHub: await chainInfo(ah, opts.assetHub),
            relay: await chainInfo(relay, opts.relay),
            coretime: await chainInfo(ct, opts.coretime),
        };
        const res = { chains, ...(await build(ah, relay, ct)) };
        for (const c of Object.values(chains))
            console.log(`${c.chain} #${c.block} spec ${c.spec} (${c.rpc})`);
        console.log(
            `relay n_delay_tranches: live ${res.relayConfig.nDelayTranches}, leg 1 ${res.params.leg1.nDelayTranches}, leg 2 ${res.params.leg2.nDelayTranches}`,
        );
        console.log(
            `sale cap: limitCoresOffered ${res.params.saleCap} (first bulk core ${res.params.firstCore})`,
        );
        const rows = [
            ["proposal", "whitelist.dispatchWhitelistedCallWithPreimage"],
            ["inner", "utility.batchAll, the call the Fellowship whitelists"],
            ["leg2", "utility.batchAll, staged by scheduler.scheduleNamedAfter"],
            ["coretimeLeg1", "Coretime call in the leg-1 Transact"],
            ["coretimeLeg2", "Coretime call in the leg-2 Transact"],
            ["relayLeg1", "relay call in the leg-1 Transact"],
            ["relayLeg2", "relay call in the leg-2 Transact"],
        ];
        for (const [key, what] of rows) {
            const s = res[key];
            console.log(`\n${key} (${what})`);
            console.log(`  hex         ${s.hex}`);
            console.log(`  len         ${s.len}`);
            console.log(`  blake2-256  ${s.blake2_256}`);
        }
        if (opts.json) {
            writeFileSync(opts.json, JSON.stringify(res, null, 1) + "\n");
            console.log(`\nwrote ${opts.json}`);
        }
    } finally {
        await Promise.all([
            ah.disconnect(),
            relay.disconnect(),
            ct.disconnect(),
        ]);
    }
}

main().catch((err) => {
    console.error(`ERROR: ${err?.message ?? err}`);
    process.exit(1);
});
