// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Kusama validator set reduction 700 -> 500 -> 300, with cores in lockstep (140 -> 100 -> 60)
//! and the relay's `n_delay_tranches` scaled with the set (299 -> 214 -> 128).
//!
//! Tests the WhitelistedCaller referendum on Kusama Asset Hub. The pinned bytes are the output of
//! `dry-run/validator-set-reduction-500-300-preimage.sh` against live Kusama, and
//! `preimage_encoding_matches_pinned_bytes` checks that this repo's runtimes encode the same.
//!
//! ```text
//! whitelist.dispatch_whitelisted_call_with_preimage(utility.batch_all([
//!   staking.set_validator_count(500),
//!   polkadot_xcm.send(relay, [UnpaidExecution, Transact(Superuser,
//!     configuration.set_n_delay_tranches(214))]),
//!   polkadot_xcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//!     utility.batch_all([broker.request_core_count(100),
//!                        broker.configure(live config, limit_cores_offered: Some(52))]))]),
//!   scheduler.schedule_named_after("ksm-validator-set-reduction-leg2", 35 * DAYS, None, 63,
//!     utility.batch_all([staking.set_validator_count(300),
//!       polkadot_xcm.send(relay, [UnpaidExecution, Transact(Superuser,
//!         configuration.set_n_delay_tranches(128))]),
//!       polkadot_xcm.send(Coretime, [UnpaidExecution, Transact(Superuser,
//!         broker.request_core_count(60))])])),
//! ]))
//! ```
//!
//! Approval voting draws each validator's delay tranche uniformly from `n_delay_tranches`, so
//! validators per delay tranche is `validators / n_delay_tranches`. The new values keep the live
//! 700 / 299 ratio. The relay applies them at session + 2, like `num_cores`.

use crate::{
	common::collectives_send_whitelist, imports::*,
	open_gov_on_asset_hub::dispatch_whitelisted_asset_hub_call_and,
};
use asset_hub_kusama_runtime::governance::pallet_custom_origins::Origin as AhOrigin;
use codec::Decode;
use emulated_integration_tests_common::impls::{
	cumulus_pallet_parachain_system, pallet_message_queue,
};
use frame_support::traits::{Bounded, OnFinalize, OneSessionHandler};
use hex_literal::hex;
use pallet_broker::ConfigRecord;
use polkadot_primitives::ValidatorId;
use runtime_parachains::configuration::{ActiveConfig, PendingConfigs};
use sp_runtime::{traits::BlockNumberProvider, AccountId32, Perbill};
use std::collections::BTreeMap;

fn blake2_256(bytes: &[u8]) -> [u8; 32] {
	use sp_runtime::traits::Hash;
	sp_runtime::traits::BlakeTwo256::hash(bytes).0
}

type AhRuntime = <AssetHubKusama as Chain>::Runtime;
type AhCall = <AssetHubKusama as Chain>::RuntimeCall;
type AhEvent = <AssetHubKusama as Chain>::RuntimeEvent;
type AhRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;
type CtRuntime = <CoretimeKusama as Chain>::Runtime;
type CtCall = <CoretimeKusama as Chain>::RuntimeCall;
type CtEvent = <CoretimeKusama as Chain>::RuntimeEvent;
type RcRuntime = <Kusama as Chain>::Runtime;
type RcCall = <Kusama as Chain>::RuntimeCall;

/// The inner `utility.batch_all`: to be whitelisted by Fellowship, and dispatched as Root by the
/// whitelist pallet
const PREIMAGE_INNER: [u8; 172] = hex!("2802105909d1071f0005010005082f0000060200183313d60000001f0005010100b50f05082f0000060200a02802083212640032000a000000c0890100c0890100b01300000084d71701340080c3c901b013000007056b736d2d76616c696461746f722d7365742d726564756374696f6e2d6c656732c0b00700003f28020c5909b1041f0005010005082f0000060200183313800000001f0005010100b50f05082f00000602001032123c00");
/// blake2-256 of the inner call (what `whitelist.whitelist_call` takes).
const PREIMAGE_INNER_BLAKE2_256: [u8; 32] =
	hex!("c7817c613b1756c81bf7139de882f6ecaff4d1e8835db2ce701404ef709280e8");
/// Length and blake2-256 of the preimage (the outer call the referendum submits).
const PREIMAGE_LEN: usize = 174;
const PREIMAGE_BLAKE2_256: [u8; 32] =
	hex!("b4efb4f5228cfe09b6df3f8e4938a2e514febc7c8858f87751a65060f46f239c");
/// The leg-2 call (300 validators, 60 cores, 128 tranches) that leg 1 stages in the scheduler,
/// stored inline.
const LEG2_STAGED_CALL: [u8; 48] = hex!("28020c5909b1041f0005010005082f0000060200183313800000001f0005010100b50f05082f00000602001032123c00");
/// The Coretime call inside the leg-1 Transact.
const PREIMAGE_CORETIME_LEG1: [u8; 40] =
	hex!("2802083212640032000a000000c0890100c0890100b01300000084d71701340080c3c901b0130000");
/// The relay calls inside the leg-1 and leg-2 Transacts.
const PREIMAGE_RELAY_LEG1: [u8; 6] = hex!("3313d6000000");
const PREIMAGE_RELAY_LEG2: [u8; 6] = hex!("331380000000");

/// Proposal parameters
const LEG2_ID: [u8; 32] = *b"ksm-validator-set-reduction-leg2";
const PRIORITY: u8 = 63;
const LEG1_VALIDATORS: u32 = 500;
const LEG1_CORES: u16 = 100;
const LEG2_VALIDATORS: u32 = 300;
const LEG2_CORES: u16 = 60;
const LEG1_N_DELAY_TRANCHES: u32 = 214;
const LEG2_N_DELAY_TRANCHES: u32 = 128;
const SALE_CAP: u16 = 52;
const LEG2_AFTER: u32 = 35 * kusama_runtime_constants::time::DAYS;

/// Live Kusama values the proposal starts from.
const LIVE_VALIDATOR_COUNT: u32 = 700;
const LIVE_NUM_CORES: u32 = 140;
const LIVE_MAX_VALIDATORS_PER_CORE: u32 = 5;
const LIVE_MAX_VALIDATORS: u32 = 700;
const LIVE_N_DELAY_TRANCHES: u32 = 299;

/// Validators per delay tranche may not drop below the plan's lowest steady state, 500 / 214
/// (2.336; 700 / 299 is 2.341). Cutting validators before tranches would give 500 / 299.
/// Compared as fractions: `v / t >= MIN.0 / MIN.1`.
const MIN_PER_TRANCHE: (u32, u32) = (LEG1_VALIDATORS, LEG1_N_DELAY_TRANCHES);

/// Live `broker.configuration` on Kusama Coretime. The emulated Coretime genesis has none, so
/// `preimage_encoding_matches_pinned_bytes` decodes the pinned bytes and checks them against this
/// record field by field.
fn live_broker_config() -> ConfigRecord<u32> {
	ConfigRecord {
		advance_notice: 10,
		interlude_length: 100_800,
		leadin_length: 100_800,
		region_length: 5_040,
		ideal_bulk_proportion: Perbill::from_parts(400_000_000),
		limit_cores_offered: None,
		renewal_bump: Perbill::from_parts(30_000_000),
		contribution_timeout: 5_040,
	}
}

/// The proposal's `broker.configure` argument: the live record with only the sale cap changed.
fn proposed_broker_config() -> ConfigRecord<u32> {
	ConfigRecord { limit_cores_offered: Some(SALE_CAP), ..live_broker_config() }
}

// ---- The proposal, rebuilt from typed calls --------------------------------------------------

fn coretime_leg1_call() -> CtCall {
	CtCall::Utility(pallet_utility::Call::<CtRuntime>::batch_all {
		calls: vec![
			CtCall::Broker(pallet_broker::Call::<CtRuntime>::request_core_count {
				core_count: LEG1_CORES,
			}),
			CtCall::Broker(pallet_broker::Call::<CtRuntime>::configure {
				config: proposed_broker_config(),
			}),
		],
	})
}

fn coretime_leg2_call() -> CtCall {
	CtCall::Broker(pallet_broker::Call::<CtRuntime>::request_core_count { core_count: LEG2_CORES })
}

fn relay_n_delay_tranches_call(new: u32) -> RcCall {
	RcCall::Configuration(
		runtime_parachains::configuration::Call::<RcRuntime>::set_n_delay_tranches { new },
	)
}

/// `polkadot_xcm.send` from AH, in the shape the proposal uses (V5,
/// `UnpaidExecution { Unlimited, None }`, `Transact { Superuser, None, call }`).
fn send_from_ah(dest: Location, encoded_call: Vec<u8>) -> AhCall {
	AhCall::PolkadotXcm(pallet_xcm::Call::<AhRuntime>::send {
		dest: bx!(VersionedLocation::V5(dest)),
		message: bx!(VersionedXcm::V5(Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: encoded_call.into(),
			},
		]))),
	})
}

fn send_to_coretime(call: CtCall) -> AhCall {
	send_from_ah(Location::new(1, [Parachain(CoretimeKusama::para_id().into())]), call.encode())
}

fn send_to_relay(call: RcCall) -> AhCall {
	send_from_ah(Location::parent(), call.encode())
}

fn set_validator_count(new: u32) -> AhCall {
	AhCall::Staking(pallet_staking_async::Call::<AhRuntime>::set_validator_count { new })
}

fn leg2_call() -> AhCall {
	AhCall::Utility(pallet_utility::Call::<AhRuntime>::batch_all {
		calls: vec![
			set_validator_count(LEG2_VALIDATORS),
			send_to_relay(relay_n_delay_tranches_call(LEG2_N_DELAY_TRANCHES)),
			send_to_coretime(coretime_leg2_call()),
		],
	})
}

fn inner_call() -> AhCall {
	AhCall::Utility(pallet_utility::Call::<AhRuntime>::batch_all {
		calls: vec![
			set_validator_count(LEG1_VALIDATORS),
			send_to_relay(relay_n_delay_tranches_call(LEG1_N_DELAY_TRANCHES)),
			send_to_coretime(coretime_leg1_call()),
			AhCall::Scheduler(pallet_scheduler::Call::<AhRuntime>::schedule_named_after {
				id: LEG2_ID,
				after: LEG2_AFTER,
				maybe_periodic: None,
				priority: PRIORITY,
				call: Box::new(leg2_call()),
			}),
		],
	})
}

fn preimage_call() -> AhCall {
	AhCall::Whitelist(
		pallet_whitelist::Call::<AhRuntime>::dispatch_whitelisted_call_with_preimage {
			call: Box::new(inner_call()),
		},
	)
}

// ---- Emulator helpers ------------------------------------------------------------------------

/// Put the parts of the live relay configuration that decide backing groups into the emulated
/// relay (whose genesis uses `HostConfiguration::default()` with 0 cores), and set the live
/// `ValidatorCount` on AH.
fn setup_live_like_network() {
	Kusama::execute_with(|| {
		// The relay's `NotifyCoreCount` goes to the broker over DMP.
		Dmp::make_parachain_reachable(CoretimeKusama::para_id());
		let mut config = ActiveConfig::<RcRuntime>::get();
		config.scheduler_params.num_cores = LIVE_NUM_CORES;
		config.scheduler_params.max_validators_per_core = Some(LIVE_MAX_VALIDATORS_PER_CORE);
		config.max_validators = Some(LIVE_MAX_VALIDATORS);
		config.n_delay_tranches = LIVE_N_DELAY_TRANCHES;
		ActiveConfig::<RcRuntime>::put(config);
		assert!(PendingConfigs::<RcRuntime>::get().is_empty());
	});
	AssetHubKusama::execute_with(|| {
		pallet_staking_async::ValidatorCount::<AhRuntime>::put(LIVE_VALIDATOR_COUNT);
	});
}

#[derive(Debug)]
struct RelaySession {
	session: u32,
	active_num_cores: u32,
	active_n_delay_tranches: u32,
	/// Backing group size -> number of groups of that size (`ParaScheduler::ValidatorGroups`).
	group_sizes: BTreeMap<usize, usize>,
}

fn groups(spec: &[(usize, usize)]) -> BTreeMap<usize, usize> {
	spec.iter().copied().collect()
}

/// Rotate the emulated relay into the next session with `n_validators` synthetic, distinct
/// validator keys, through the relay's real session path: bump `pallet_session::CurrentIndex`, then
/// call the parachains `Initializer`'s `OneSessionHandler::on_new_session` (as `pallet_session`
/// does at a rotation) and `on_finalize`. That makes due `PendingConfigs` active, builds
/// `ValidatorGroups` (`scheduler::initializer_on_new_session`), and has `coretime` send
/// `NotifyCoreCount` to the broker if `num_cores` changed. `pallet_session` itself isn't rotated
/// because the emulated relay has only a handful of session keys.
fn rotate_relay_session(n_validators: u32) -> RelaySession {
	Kusama::execute_with(|| {
		let next = pallet_session::CurrentIndex::<RcRuntime>::get() + 1;
		pallet_session::CurrentIndex::<RcRuntime>::put(next);

		let keys: Vec<(AccountId32, ValidatorId)> = (0..n_validators)
			.map(|i| {
				let mut raw = [0xa5u8; 32];
				raw[..4].copy_from_slice(&(i + 1).to_le_bytes());
				(AccountId32::new(raw), ValidatorId::from(sp_core::sr25519::Public::from_raw(raw)))
			})
			.collect();
		let as_iter = || keys.iter().map(|(a, v)| (a, v.clone())).collect::<Vec<_>>().into_iter();
		<kusama_runtime::Initializer as OneSessionHandler<AccountId32>>::on_new_session(
			true,
			as_iter(),
			as_iter(),
		);
		<kusama_runtime::Initializer as OnFinalize<u32>>::on_finalize(
			<Kusama as Chain>::System::block_number(),
		);

		assert_eq!(runtime_parachains::shared::CurrentSessionIndex::<RcRuntime>::get(), next);
		let mut group_sizes = BTreeMap::new();
		for g in runtime_parachains::scheduler::ValidatorGroups::<RcRuntime>::get() {
			*group_sizes.entry(g.len()).or_insert(0) += 1;
		}
		RelaySession {
			session: next,
			active_num_cores: ActiveConfig::<RcRuntime>::get().scheduler_params.num_cores,
			active_n_delay_tranches: ActiveConfig::<RcRuntime>::get().n_delay_tranches,
			group_sizes,
		}
	})
}

/// Rotate as `rotate_relay_session`, and check validators per delay tranche stays at or above
/// `MIN_PER_TRANCHE`.
fn rotate_and_check_tranches(n_validators: u32) -> RelaySession {
	let s = rotate_relay_session(n_validators);
	assert!(
		n_validators * MIN_PER_TRANCHE.1 >= MIN_PER_TRANCHE.0 * s.active_n_delay_tranches,
		"{n_validators} validators / {} tranches is below {}/{}: {s:?}",
		s.active_n_delay_tranches,
		MIN_PER_TRANCHE.0,
		MIN_PER_TRANCHE.1,
	);
	s
}

/// Relay checks after an AH leg: AH's UMP (n_delay_tranches) and the broker's UMP (num_cores)
/// were processed, and the last pending configuration, for session + 2, has both values. The two
/// messages take different routes; if they straddled a session change there are two pending
/// entries, so every entry may differ from the active config only in these two fields.
fn assert_relay_queued(expected_num_cores: u32, expected_n_delay_tranches: u32) {
	Kusama::execute_with(|| {
		Kusama::assert_ump_queue_processed(true, Some(AssetHubKusama::para_id()), None);
		Kusama::assert_ump_queue_processed(true, Some(CoretimeKusama::para_id()), None);
		let pending = PendingConfigs::<RcRuntime>::get();
		let current = runtime_parachains::shared::CurrentSessionIndex::<RcRuntime>::get();
		let (last_session, last) = pending.last().expect("a pending relay config");
		assert_eq!(*last_session, current + 2, "pending config must apply at session + 2");
		assert_eq!(
			(last.scheduler_params.num_cores, last.n_delay_tranches),
			(expected_num_cores, expected_n_delay_tranches)
		);
		let active = ActiveConfig::<RcRuntime>::get();
		let masked = |c: &runtime_parachains::configuration::HostConfiguration<u32>| {
			let mut c = c.clone();
			c.scheduler_params.num_cores = 0;
			c.n_delay_tranches = 0;
			c
		};
		for (session, config) in &pending {
			assert_eq!(
				masked(config),
				masked(&active),
				"pending config for session {session} may change only num_cores and n_delay_tranches"
			);
			assert!(
				[active.scheduler_params.num_cores, expected_num_cores]
					.contains(&config.scheduler_params.num_cores) &&
					[active.n_delay_tranches, expected_n_delay_tranches]
						.contains(&config.n_delay_tranches),
				"{config:?}"
			);
		}
	});
}

fn broker_core_count_inbox() -> Option<u16> {
	CoretimeKusama::ext_wrapper(pallet_broker::CoreCountInbox::<CtRuntime>::get)
}

// ---- Tests -----------------------------------------------------------------------------------

/// The pinned bytes are exactly what this repo's typed calls encode to.
#[test]
fn preimage_encoding_matches_pinned_bytes() {
	assert_eq!(LEG2_AFTER, 504_000);
	assert_eq!(u32::from(CoretimeKusama::para_id()), 1005);

	// Coretime leg-1 call: decode the pinned bytes with the Coretime runtime and check the
	// structure, then that it's the live broker config with only `limit_cores_offered` changed.
	let decoded =
		CtCall::decode(&mut &PREIMAGE_CORETIME_LEG1[..]).expect("decodes as Coretime call");
	match &decoded {
		CtCall::Utility(pallet_utility::Call::batch_all { calls }) => {
			assert_eq!(calls.len(), 2);
			assert_eq!(
				calls[0],
				CtCall::Broker(pallet_broker::Call::request_core_count { core_count: LEG1_CORES })
			);
			match &calls[1] {
				CtCall::Broker(pallet_broker::Call::configure { config }) => {
					assert_eq!(config.limit_cores_offered, Some(SALE_CAP));
					let as_live = ConfigRecord { limit_cores_offered: None, ..config.clone() };
					assert_eq!(as_live, live_broker_config());
				},
				other => panic!("second Coretime call is not broker.configure: {other:?}"),
			}
		},
		other => panic!("Coretime leg-1 call is not utility.batch_all: {other:?}"),
	}
	assert_eq!(decoded, coretime_leg1_call());
	assert_eq!(coretime_leg1_call().encode(), PREIMAGE_CORETIME_LEG1.to_vec());
	assert_eq!(coretime_leg2_call().encode(), hex!("32123c00").to_vec());
	// Why the config is not rebuilt from the emulated genesis: it has none.
	CoretimeKusama::execute_with(|| {
		assert_eq!(pallet_broker::Configuration::<CtRuntime>::get(), None);
	});

	// Relay calls: configuration.set_n_delay_tranches(214), then (128).
	for (bytes, n) in
		[(PREIMAGE_RELAY_LEG1, LEG1_N_DELAY_TRANCHES), (PREIMAGE_RELAY_LEG2, LEG2_N_DELAY_TRANCHES)]
	{
		assert_eq!(
			RcCall::decode(&mut &bytes[..]).expect("decodes as relay call"),
			relay_n_delay_tranches_call(n)
		);
		assert_eq!(relay_n_delay_tranches_call(n).encode(), bytes.to_vec());
	}
	// Both keep validators per delay tranche at the live 700 / 299, rounded.
	assert_eq!(LEG1_N_DELAY_TRANCHES, (LIVE_N_DELAY_TRANCHES * LEG1_VALIDATORS + 350) / 700);
	assert_eq!(LEG2_N_DELAY_TRANCHES, (LIVE_N_DELAY_TRANCHES * LEG2_VALIDATORS + 350) / 700);

	// Leg 2 (stored inline by the scheduler, so it must stay <= 128 bytes).
	assert_eq!(leg2_call().encode(), LEG2_STAGED_CALL.to_vec());

	// Inner call: bytes, blake2-256, and the hash the emulator's whitelist helpers compute.
	let inner = inner_call();
	let inner_bytes = inner.encode();
	assert_eq!(inner_bytes.len(), PREIMAGE_INNER.len());
	assert_eq!(inner_bytes, PREIMAGE_INNER.to_vec());
	assert_eq!(blake2_256(&inner_bytes), PREIMAGE_INNER_BLAKE2_256);
	assert_eq!(call_hash_of::<AssetHubKusama>(&inner).0, PREIMAGE_INNER_BLAKE2_256);
	assert_eq!(AhCall::decode(&mut &PREIMAGE_INNER[..]).expect("decodes as AH call"), inner);

	// Outer call (the preimage): whitelist pallet 94, call 3, then the inner bytes.
	let outer_bytes = preimage_call().encode();
	assert_eq!(outer_bytes.len(), PREIMAGE_LEN);
	assert_eq!(&outer_bytes[..2], &[0x5e, 0x03]);
	assert_eq!(&outer_bytes[2..], &PREIMAGE_INNER[..]);
	assert_eq!(blake2_256(&outer_bytes), PREIMAGE_BLAKE2_256);
}

/// Leg 1 through the whitelisted path, then leg 2 by advancing the AH scheduler's clock, with the
/// effects on AH, Coretime and the relay, and relay session rotations that show the backing
/// groups after each leg.
///
/// Leg 2 runs through the scheduler itself, not by dispatching the staged call by hand: the
/// emulator's relay block number (the AH scheduler's clock) jumps to the leg-2 slot, and
/// `Scheduler::IncompleteSince` moves up to it so the scheduler doesn't walk ~504k empty agendas.
/// The test asserts that no other agenda exists, so nothing is skipped.
#[test]
fn preimage_legs_1_and_2_end_to_end() {
	setup_live_like_network();
	let inner = inner_call();
	let inner_hash = call_hash_of::<AssetHubKusama>(&inner);

	// Before: 700 validators, 140 cores -> 140 groups of 5, 299 delay tranches.
	let s = rotate_and_check_tranches(LIVE_VALIDATOR_COUNT);
	assert_eq!(s.active_num_cores, LIVE_NUM_CORES);
	assert_eq!(s.active_n_delay_tranches, LIVE_N_DELAY_TRANCHES);
	assert_eq!(s.group_sizes, groups(&[(5, 140)]), "{s:?}");

	// ---- Leg 1 ----
	let mut leg1_relay_now = 0u32;
	dispatch_whitelisted_asset_hub_call_and(inner, || {
		// A direct dispatch sees this block's relay parent. On mainnet the referendum enacts from
		// the scheduler's `on_initialize`, which sees the previous AH block's relay parent.
		leg1_relay_now =
			<AhRuntime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number();
		let expected_when = leg1_relay_now + LEG2_AFTER + 1;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				AhEvent::Utility(pallet_utility::Event::BatchCompleted) => {},
				AhEvent::PolkadotXcm(pallet_xcm::Event::Sent { destination, .. }) => {
					destination: *destination == Location::parent(),
				},
				AhEvent::PolkadotXcm(pallet_xcm::Event::Sent { destination, .. }) => {
					destination: *destination ==
						Location::new(1, [Parachain(CoretimeKusama::para_id().into())]),
				},
				AhEvent::Scheduler(pallet_scheduler::Event::Scheduled { when, index }) => {
					when: *when == expected_when,
					index: *index == 0,
				},
				AhEvent::Whitelist(pallet_whitelist::Event::WhitelistedCallDispatched {
					call_hash, result
				}) => {
					call_hash: *call_hash == inner_hash,
					result: result.is_ok(),
				},
			]
		);
	});
	let when = leg1_relay_now + LEG2_AFTER + 1;

	// AH after leg 1: count 500, whitelist entry consumed, leg 2 staged as Root under its name.
	AssetHubKusama::ext_wrapper(|| {
		assert_eq!(pallet_staking_async::ValidatorCount::<AhRuntime>::get(), LEG1_VALIDATORS);
		assert!(!pallet_whitelist::WhitelistedCall::<AhRuntime>::contains_key(inner_hash));
		assert_eq!(pallet_scheduler::Lookup::<AhRuntime>::get(LEG2_ID), Some((when, 0)));
		let agenda = pallet_scheduler::Agenda::<AhRuntime>::get(when);
		assert_eq!(agenda.len(), 1);
		let task = agenda[0].clone().expect("leg-2 task present");
		assert_eq!(task.maybe_id, Some(LEG2_ID));
		assert_eq!(task.priority, PRIORITY);
		assert_eq!(task.maybe_periodic, None);
		assert_eq!(
			task.origin,
			asset_hub_kusama_runtime::OriginCaller::system(frame_system::RawOrigin::Root)
		);
		match task.call {
			Bounded::Inline(bytes) => assert_eq!(bytes.into_inner(), LEG2_STAGED_CALL.to_vec()),
			other => panic!("leg 2 not stored inline: {other:?}"),
		}
	});

	// Coretime after leg 1: XCM from AH processed, 100 cores requested (UMP to the relay), sale
	// capped at 52.
	CoretimeKusama::execute_with(|| {
		assert_expected_events!(
			CoretimeKusama,
			vec![
				CtEvent::MessageQueue(pallet_message_queue::Event::Processed {
					success: true, ..
				}) => {},
				CtEvent::Broker(pallet_broker::Event::CoreCountRequested { core_count: 100 }) => {},
				CtEvent::Utility(pallet_utility::Event::BatchCompleted) => {},
				CtEvent::ParachainSystem(
					cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
		assert_eq!(
			pallet_broker::Configuration::<CtRuntime>::get(),
			Some(proposed_broker_config())
		);
	});

	// Relay after leg 1: num_cores 100 and n_delay_tranches 214 queued for session + 2.
	assert_relay_queued(LEG1_CORES.into(), LEG1_N_DELAY_TRANCHES);

	// Sessions after leg 1: cores and tranches drop first (validators still 700, so still 140
	// groups of 5, and 700 / 214 per tranche), then the 500 set gives 100 groups of 5.
	let s = rotate_and_check_tranches(LIVE_VALIDATOR_COUNT);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(LIVE_NUM_CORES, LIVE_N_DELAY_TRANCHES, groups(&[(5, 140)]))
	);
	let s = rotate_and_check_tranches(LIVE_VALIDATOR_COUNT);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(100, LEG1_N_DELAY_TRANCHES, groups(&[(5, 140)])),
		"{s:?}"
	);
	assert_eq!(broker_core_count_inbox(), Some(LEG1_CORES), "relay notifies the broker");
	let s = rotate_and_check_tranches(LEG1_VALIDATORS);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(100, LEG1_N_DELAY_TRANCHES, groups(&[(5, 100)])),
		"{s:?}"
	);

	// ---- Leg 2: advance the AH scheduler's clock (relay block number) to the leg-2 slot ----
	Kusama::ext_wrapper(|| <Kusama as Chain>::System::set_block_number(when));
	let mut relay_now = 0;
	for _ in 0..3 {
		AssetHubKusama::execute_with(|| {});
		relay_now = AssetHubKusama::ext_wrapper(
			<AhRuntime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number,
		);
		if relay_now >= when {
			break;
		}
	}
	assert!(relay_now >= when, "AH relay clock {relay_now} did not reach {when}");
	AssetHubKusama::ext_wrapper(|| {
		let agendas: Vec<u32> = pallet_scheduler::Agenda::<AhRuntime>::iter_keys().collect();
		assert_eq!(agendas, vec![when], "no other agenda may be skipped");
		pallet_scheduler::IncompleteSince::<AhRuntime>::put(when);
	});
	// The next AH block's scheduler `on_initialize` sees relay block >= `when` and runs leg 2.
	AssetHubKusama::execute_with(|| {
		assert_expected_events!(
			AssetHubKusama,
			vec![
				AhEvent::Scheduler(pallet_scheduler::Event::Dispatched { task, id, result }) => {
					task: *task == (when, 0),
					id: *id == Some(LEG2_ID),
					result: result.is_ok(),
				},
				AhEvent::Utility(pallet_utility::Event::BatchCompleted) => {},
				AhEvent::PolkadotXcm(pallet_xcm::Event::Sent { destination, .. }) => {
					destination: *destination == Location::parent(),
				},
				AhEvent::PolkadotXcm(pallet_xcm::Event::Sent { destination, .. }) => {
					destination: *destination ==
						Location::new(1, [Parachain(CoretimeKusama::para_id().into())]),
				},
			]
		);
		assert_eq!(pallet_staking_async::ValidatorCount::<AhRuntime>::get(), LEG2_VALIDATORS);
		assert_eq!(pallet_scheduler::Lookup::<AhRuntime>::get(LEG2_ID), None);
		assert!(pallet_scheduler::Agenda::<AhRuntime>::get(when).iter().all(|t| t.is_none()));
	});

	// Coretime after leg 2: 60 cores requested; the sale cap from leg 1 stays.
	CoretimeKusama::execute_with(|| {
		assert_expected_events!(
			CoretimeKusama,
			vec![
				CtEvent::MessageQueue(pallet_message_queue::Event::Processed {
					success: true, ..
				}) => {},
				CtEvent::Broker(pallet_broker::Event::CoreCountRequested { core_count: 60 }) => {},
			]
		);
		assert_eq!(
			pallet_broker::Configuration::<CtRuntime>::get(),
			Some(proposed_broker_config())
		);
	});

	// Relay after leg 2: num_cores 60 and n_delay_tranches 128 queued for session + 2.
	assert_relay_queued(LEG2_CORES.into(), LEG2_N_DELAY_TRANCHES);

	// Sessions after leg 2: cores and tranches drop first (500 validators -> still 100 groups of
	// 5, and 500 / 128 per tranche), then the 300 set gives 60 groups of 5.
	let s = rotate_and_check_tranches(LEG1_VALIDATORS);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(100, LEG1_N_DELAY_TRANCHES, groups(&[(5, 100)])),
		"{s:?}"
	);
	let s = rotate_and_check_tranches(LEG1_VALIDATORS);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(60, LEG2_N_DELAY_TRANCHES, groups(&[(5, 100)])),
		"{s:?}"
	);
	assert_eq!(broker_core_count_inbox(), Some(LEG2_CORES), "relay notifies the broker");
	let s = rotate_and_check_tranches(LEG2_VALIDATORS);
	assert_eq!(
		(s.active_num_cores, s.active_n_delay_tranches, s.group_sizes.clone()),
		(60, LEG2_N_DELAY_TRANCHES, groups(&[(5, 60)])),
		"{s:?}"
	);
	assert!(s.session > 0);
}

/// StakingAdmin can't enact the proposal: neither the inner batch directly (as the staking-admin
/// track would) nor through the whitelist wrapper, even once whitelisted, ergo nothing changes.
#[test]
fn preimage_rejected_for_staking_admin() {
	setup_live_like_network();
	let inner = inner_call();
	let inner_hash = call_hash_of::<AssetHubKusama>(&inner);

	AssetHubKusama::execute_with(|| {
		let origin: AhRuntimeOrigin = AhOrigin::StakingAdmin.into();
		let result = inner.clone().dispatch(origin);
		assert_eq!(result.map_err(|e| e.error), Err(DispatchError::BadOrigin));
	});

	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AhCall::Whitelist(pallet_whitelist::Call::<AhRuntime>::whitelist_call {
				call_hash: inner_hash,
			})
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, inner_hash);
	});
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			inner,
			AhOrigin::StakingAdmin.into()
		),
		DispatchError::BadOrigin
	);

	AssetHubKusama::ext_wrapper(|| {
		assert_eq!(pallet_staking_async::ValidatorCount::<AhRuntime>::get(), LIVE_VALIDATOR_COUNT);
		assert_eq!(pallet_scheduler::Lookup::<AhRuntime>::get(LEG2_ID), None);
		assert!(pallet_whitelist::WhitelistedCall::<AhRuntime>::contains_key(inner_hash));
	});
	CoretimeKusama::ext_wrapper(|| {
		assert_eq!(pallet_broker::Configuration::<CtRuntime>::get(), None);
	});
	Kusama::ext_wrapper(|| assert!(PendingConfigs::<RcRuntime>::get().is_empty()));
}

/// Without the Fellowship's whitelist entry the referendum fails with `CallIsNotWhitelisted`.
#[test]
fn preimage_rejected_when_not_whitelisted() {
	setup_live_like_network();
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			inner_call(),
			AhOrigin::WhitelistedCaller.into()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);
	AssetHubKusama::ext_wrapper(|| {
		assert_eq!(pallet_staking_async::ValidatorCount::<AhRuntime>::get(), LIVE_VALIDATOR_COUNT);
		assert_eq!(pallet_scheduler::Lookup::<AhRuntime>::get(LEG2_ID), None);
	});
}

/// The relay's backing groups are `max(num_cores, validators / max_validators_per_core)`, filled
/// as evenly as possible (`scheduler::initializer_on_new_session`). This runs the real session
/// path for each of 700/500/300 validators x 140/100/60 cores, with `max_validators_per_core` 5.
#[test]
fn relay_backing_groups_table() {
	setup_live_like_network();
	let table: [(u32, u32, &[(usize, usize)]); 9] = [
		(700, 140, &[(5, 140)]),
		(700, 100, &[(5, 140)]),
		(700, 60, &[(5, 140)]),
		(500, 140, &[(3, 60), (4, 80)]),
		(500, 100, &[(5, 100)]),
		(500, 60, &[(5, 100)]),
		(300, 140, &[(2, 120), (3, 20)]),
		(300, 100, &[(3, 100)]),
		(300, 60, &[(5, 60)]),
	];
	for (validators, cores, expected) in table {
		Kusama::ext_wrapper(|| {
			let mut config = ActiveConfig::<RcRuntime>::get();
			config.scheduler_params.num_cores = cores;
			ActiveConfig::<RcRuntime>::put(config);
		});
		let s = rotate_relay_session(validators);
		assert_eq!(s.active_num_cores, cores);
		assert_eq!(s.active_n_delay_tranches, LIVE_N_DELAY_TRANCHES, "groups don't touch tranches");
		assert_eq!(s.group_sizes, groups(expected), "{validators} validators x {cores} cores");
	}
}
