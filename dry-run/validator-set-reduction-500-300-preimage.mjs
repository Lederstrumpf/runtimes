import { ApiPromise, WsProvider } from "@polkadot/api";
import { blake2AsHex } from "@polkadot/util-crypto";
import { stringToU8a, u8aToHex } from "@polkadot/util";
import { writeFileSync } from "node:fs";

// Builds the preimage for validator set 700 -> 500 -> 300 proposal, with cores
// 140 -> 100 -> 60, from live Kusama Asset Hub and Coretime metadata. Prints hex,
// length and blake2-256 of each call. Never signs or submits anything.
//
//   whitelist.dispatchWhitelistedCallWithPreimage(        <- proposal
//     utility.batchAll([                                  <- inner: the Fellowship whitelists its hash
//       staking.setValidatorCount(500),
//       polkadotXcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//         utility.batchAll([broker.requestCoreCount(100),
//                           broker.configure(live config, limitCoresOffered: 52)]))]),
//       scheduler.scheduleNamedAfter(leg2Id, 504000, None, 63,
//         utility.batchAll([                              <- leg 2: staged call
//           staking.setValidatorCount(300),
//           polkadotXcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//             broker.requestCoreCount(60))])])),
//     ]))
//
// broker.requestCoreCount makes the broker ask the relay for the new core count; the relay
// applies num_cores at session + 2 and notifies the broker back.

const PARAMS = {
    leg1: { validators: 500, cores: 100 },
    leg2: { validators: 300, cores: 60 },
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
        coretime: "wss://kusama-coretime-rpc.polkadot.io",
        json: null,
    };
    for (let i = 0; i < args.length; i++) {
        switch (args[i]) {
            case "--asset-hub":
                opts.assetHub = args[++i];
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

async function build(ah, ct, p = PARAMS) {
    if (stringToU8a(p.leg2Id).length !== 32)
        throw new Error("leg2Id must be 32 bytes");

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
    const toCoretime = (remote) =>
        ah.tx.polkadotXcm.send(
            {
                V5: {
                    parents: 1,
                    interior: { X1: [{ Parachain: p.coretimeParaId }] },
                },
            },
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
    const leg2 = ah.tx.utility.batchAll([
        ah.tx.staking.setValidatorCount(p.leg2.validators),
        toCoretime(coretimeLeg2),
    ]);
    const inner = ah.tx.utility.batchAll([
        ah.tx.staking.setValidatorCount(p.leg1.validators),
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
        proposal: summarize(proposal),
        inner: summarize(inner),
        leg2: summarize(leg2),
        coretimeLeg1: summarize(coretimeLeg1),
        coretimeLeg2: summarize(coretimeLeg2),
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
    const [ah, ct] = await Promise.all(
        [opts.assetHub, opts.coretime].map((url) =>
            ApiPromise.create({
                provider: new WsProvider(url),
                noInitWarn: true,
            }),
        ),
    );
    try {
        const chains = {
            assetHub: await chainInfo(ah, opts.assetHub),
            coretime: await chainInfo(ct, opts.coretime),
        };
        const res = { chains, ...(await build(ah, ct)) };
        for (const c of Object.values(chains))
            console.log(`${c.chain} #${c.block} spec ${c.spec} (${c.rpc})`);
        console.log(
            `sale cap: limitCoresOffered ${res.params.saleCap} (first bulk core ${res.params.firstCore})`,
        );
        const rows = [
            ["proposal", "whitelist.dispatchWhitelistedCallWithPreimage"],
            ["inner", "utility.batchAll, the call the Fellowship whitelists"],
            ["leg2", "utility.batchAll, staged by scheduler.scheduleNamedAfter"],
            ["coretimeLeg1", "Coretime call in the leg-1 Transact"],
            ["coretimeLeg2", "Coretime call in the leg-2 Transact"],
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
        await Promise.all([ah.disconnect(), ct.disconnect()]);
    }
}

main().catch((err) => {
    console.error(`ERROR: ${err?.message ?? err}`);
    process.exit(1);
});
