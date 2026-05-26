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

use crate::{common::collectives_send_whitelist, imports::*};
use asset_hub_kusama_runtime::governance::pallet_custom_origins::Origin;

#[test]
fn assethub_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![AssetHubRuntimeCall::System(frame_system::Call::authorize_upgrade {
				code_hash,
			})],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubKusama::execute_with(|| {
		assert!(<AssetHubKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	AssetHubKusama::execute_with(|| {
		assert_eq!(
			<AssetHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_relay_chain() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let code_hash = [1u8; 32].into();
	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![build_xcm_send_authorize_upgrade_call::<AssetHubKusama, Kusama>(
				AssetHubKusama::parent_location(),
				&code_hash,
				None,
			)],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Kusama::execute_with(|| assert!(<Kusama as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	Kusama::execute_with(|| {
		assert_eq!(<Kusama as Chain>::System::authorized_upgrade().unwrap().code_hash(), &code_hash)
	});
}

#[test]
fn assethub_can_reduce_kusama_validator_set_size() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubStakingCall = pallet_staking_async::Call<AssetHubRuntime>;
	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeCall = <Kusama as Chain>::RuntimeCall;
	type KusamaRuntimeEvent = <Kusama as Chain>::RuntimeEvent;

	let target_validator_count = 700u32;
	let validator_set_report_id = 1u32;
	let min_validator_set_size_key = checked_min_validator_set_size_key();

	let set_relay_min_validator_set_size: KusamaRuntimeCall =
		frame_system::Call::<KusamaRuntime>::set_storage {
			items: vec![(min_validator_set_size_key.to_vec(), target_validator_count.encode())],
		}
		.into();

	let set_relay_min_validator_set_size_from_asset_hub: AssetHubRuntimeCall =
		pallet_xcm::Call::<AssetHubRuntime>::send {
			dest: bx!(VersionedLocation::from(AssetHubKusama::parent_location())),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: set_relay_min_validator_set_size.encode().into(),
				},
			]))),
		}
		.into();

	let set_asset_hub_validator_count =
		AssetHubRuntimeCall::Staking(AssetHubStakingCall::set_validator_count {
			new: target_validator_count,
		});

	let reduce_validator_set_size = AssetHubRuntimeCall::Utility(pallet_utility::Call::<
		AssetHubRuntime,
	>::batch_all {
		calls: vec![set_relay_min_validator_set_size_from_asset_hub, set_asset_hub_validator_count],
	});

	// check before
	AssetHubKusama::execute_with(|| {
		pallet_staking_async::ValidatorCount::<AssetHubRuntime>::put(1000);
		assert_eq!(pallet_staking_async::ValidatorCount::<AssetHubRuntime>::get(), 1000)
	});
	Kusama::execute_with(|| {
		assert_eq!(kusama_runtime::MinimumValidatorSetSize::get(), 1000);
		assert_eq!(kusama_runtime::Staking::minimum_validator_count(), 0);
		assert_eq!(frame_support::storage::unhashed::get::<u32>(&min_validator_set_size_key), None);
	});

	// ok - reduced
	dispatch_whitelisted_asset_hub_call(reduce_validator_set_size);

	// check after - reduced
	AssetHubKusama::execute_with(|| {
		assert_eq!(
			pallet_staking_async::ValidatorCount::<AssetHubRuntime>::get(),
			target_validator_count
		)
	});
	Kusama::execute_with(|| {
		assert_eq!(kusama_runtime::Staking::minimum_validator_count(), 0);
		assert_eq!(
			frame_support::storage::unhashed::get::<u32>(&min_validator_set_size_key),
			Some(target_validator_count)
		);
		assert_eq!(kusama_runtime::MinimumValidatorSetSize::get(), target_validator_count);

		pallet_staking_async_ah_client::Mode::<KusamaRuntime>::put(
			pallet_staking_async_ah_client::OperatingMode::Active,
		);
		<Kusama as Chain>::System::reset_events();
		assert_ok!(validator_set_report_call(target_validator_count, validator_set_report_id)
			.dispatch(<Kusama as Chain>::RuntimeOrigin::root()));

		assert!(<Kusama as Chain>::System::events().iter().any(|record| {
			match &record.event {
				KusamaRuntimeEvent::StakingAhClient(
					pallet_staking_async_ah_client::Event::ValidatorSetReceived {
						id,
						new_validator_set_count,
						prune_up_to,
						leftover,
					},
				) =>
					*id == validator_set_report_id &&
						*new_validator_set_count == target_validator_count &&
						prune_up_to.is_none() &&
						!*leftover,
				_ => false,
			}
		}));

		let (id, validators) = pallet_staking_async_ah_client::ValidatorSet::<KusamaRuntime>::get()
			.expect("700-validator report should be accepted by relay");
		assert_eq!(id, validator_set_report_id);
		assert_eq!(validators.len() as u32, target_validator_count);
	});
}

#[test]
fn assethub_validator_count_without_relay_minimum_update_is_dropped() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubStakingCall = pallet_staking_async::Call<AssetHubRuntime>;
	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeEvent = <Kusama as Chain>::RuntimeEvent;

	let target_validator_count = 700u32;
	let validator_set_report_id = 1u32;
	let min_validator_set_size_key = checked_min_validator_set_size_key();

	let set_asset_hub_validator_count =
		AssetHubRuntimeCall::Staking(AssetHubStakingCall::set_validator_count {
			new: target_validator_count,
		});

	AssetHubKusama::execute_with(|| {
		pallet_staking_async::ValidatorCount::<AssetHubRuntime>::put(1000);
		assert_eq!(pallet_staking_async::ValidatorCount::<AssetHubRuntime>::get(), 1000)
	});
	Kusama::execute_with(|| {
		assert_eq!(kusama_runtime::MinimumValidatorSetSize::get(), 1000);
		assert_eq!(kusama_runtime::Staking::minimum_validator_count(), 0);
		assert_eq!(frame_support::storage::unhashed::get::<u32>(&min_validator_set_size_key), None);
	});

	dispatch_whitelisted_asset_hub_call(set_asset_hub_validator_count);

	AssetHubKusama::execute_with(|| {
		assert_eq!(
			pallet_staking_async::ValidatorCount::<AssetHubRuntime>::get(),
			target_validator_count
		)
	});
	Kusama::execute_with(|| {
		assert_eq!(kusama_runtime::MinimumValidatorSetSize::get(), 1000);
		assert_eq!(kusama_runtime::Staking::minimum_validator_count(), 0);
		assert_eq!(frame_support::storage::unhashed::get::<u32>(&min_validator_set_size_key), None);

		pallet_staking_async_ah_client::Mode::<KusamaRuntime>::put(
			pallet_staking_async_ah_client::OperatingMode::Active,
		);
		<Kusama as Chain>::System::reset_events();
		assert_ok!(validator_set_report_call(target_validator_count, validator_set_report_id)
			.dispatch(<Kusama as Chain>::RuntimeOrigin::root()));

		assert!(<Kusama as Chain>::System::events().iter().any(|record| matches!(
			&record.event,
			KusamaRuntimeEvent::StakingAhClient(
				pallet_staking_async_ah_client::Event::SetTooSmallAndDropped
			)
		)));
		assert!(pallet_staking_async_ah_client::ValidatorSet::<KusamaRuntime>::get().is_none());
	});
}

fn checked_min_validator_set_size_key() -> [u8; 16] {
	let key = kusama_runtime::MinimumValidatorSetSize::key();
	assert_eq!(
		key,
		[
			0xf3, 0xbc, 0x14, 0x3b, 0x8d, 0x00, 0x07, 0x89, 0x78, 0x3c, 0x73, 0x16, 0x62, 0xbc,
			0xb6, 0x41,
		],
	);
	key
}

fn dispatch_whitelisted_asset_hub_call(call: <AssetHubKusama as Chain>::RuntimeCall) {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();
	let call_hash = call_hash_of::<AssetHubKusama>(&call);

	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(call.clone(), ok_origin.clone()),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(call.clone(), invalid_origin),
		DispatchError::BadOrigin
	);

	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(call, ok_origin));
}

fn validator_set_report(
	validator_count: u32,
	report_id: u32,
) -> pallet_staking_async_rc_client::ValidatorSetReport<sp_runtime::AccountId32> {
	pallet_staking_async_rc_client::ValidatorSetReport::new_terminal(
		(0..validator_count)
			.map(|index| {
				let mut id = [0u8; 32];
				id[..4].copy_from_slice(&index.to_le_bytes());
				sp_runtime::AccountId32::from(id)
			})
			.collect(),
		report_id,
		None,
	)
}

fn validator_set_report_call(
	validator_count: u32,
	report_id: u32,
) -> <Kusama as Chain>::RuntimeCall {
	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeCall = <Kusama as Chain>::RuntimeCall;

	KusamaRuntimeCall::StakingAhClient(
		pallet_staking_async_ah_client::Call::<KusamaRuntime>::validator_set {
			report: validator_set_report(validator_count, report_id),
		},
	)
}

#[test]
fn assethub_can_authorize_upgrade_for_system_chains() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, BridgeHubKusama>(
					AssetHubKusama::sibling_location_of(BridgeHubKusama::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, CoretimeKusama>(
					AssetHubKusama::sibling_location_of(CoretimeKusama::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, PeopleKusama>(
					AssetHubKusama::sibling_location_of(PeopleKusama::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubKusama::execute_with(|| {
		assert!(<BridgeHubKusama as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimeKusama::execute_with(|| {
		assert!(<CoretimeKusama as Chain>::System::authorized_upgrade().is_none())
	});
	PeopleKusama::execute_with(|| {
		assert!(<PeopleKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	BridgeHubKusama::execute_with(|| {
		assert_eq!(
			<BridgeHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CoretimeKusama::execute_with(|| {
		assert_eq!(
			<CoretimeKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeopleKusama::execute_with(|| {
		assert_eq!(
			<PeopleKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}
