// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Autogenerated weights for `pallet_core_fellowship`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-19, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("../collectives-polkadot-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=../collectives-polkadot-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_core_fellowship
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./collectives-polkadot-weights
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_core_fellowship`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_core_fellowship::WeightInfo for WeightInfo<T> {
	/// Storage: `FellowshipCore::Params` (r:0 w:1)
	/// Proof: `FellowshipCore::Params` (`max_values`: Some(1), `max_size`: Some(364), added: 859, mode: `MaxEncodedLen`)
	fn set_params() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_713_000 picoseconds.
		Weight::from_parts(6_123_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::Members` (r:1 w:1)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Params` (r:1 w:0)
	/// Proof: `FellowshipCore::Params` (`max_values`: Some(1), `max_size`: Some(364), added: 859, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::MemberCount` (r:1 w:1)
	/// Proof: `FellowshipCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IdToIndex` (r:1 w:0)
	/// Proof: `FellowshipCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:1 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn bump_offboard() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66111`
		//  Estimated: `69046`
		// Minimum execution time: 117_768_000 picoseconds.
		Weight::from_parts(119_729_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::Members` (r:1 w:1)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Params` (r:1 w:0)
	/// Proof: `FellowshipCore::Params` (`max_values`: Some(1), `max_size`: Some(364), added: 859, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::MemberCount` (r:1 w:1)
	/// Proof: `FellowshipCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IdToIndex` (r:1 w:0)
	/// Proof: `FellowshipCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:1 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn bump_demote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66221`
		//  Estimated: `69046`
		// Minimum execution time: 121_741_000 picoseconds.
		Weight::from_parts(124_073_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `FellowshipCollective::Members` (r:1 w:0)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	fn set_active() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `460`
		//  Estimated: `3514`
		// Minimum execution time: 16_038_000 picoseconds.
		Weight::from_parts(16_521_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::Members` (r:1 w:1)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::MemberCount` (r:1 w:1)
	/// Proof: `FellowshipCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IndexToId` (r:0 w:1)
	/// Proof: `FellowshipCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IdToIndex` (r:0 w:1)
	/// Proof: `FellowshipCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn induct() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3514`
		// Minimum execution time: 24_393_000 picoseconds.
		Weight::from_parts(25_210_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `FellowshipCollective::Members` (r:1 w:1)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Params` (r:1 w:0)
	/// Proof: `FellowshipCore::Params` (`max_values`: Some(1), `max_size`: Some(364), added: 859, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::MemberCount` (r:1 w:1)
	/// Proof: `FellowshipCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:1 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IndexToId` (r:0 w:1)
	/// Proof: `FellowshipCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::IdToIndex` (r:0 w:1)
	/// Proof: `FellowshipCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn promote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66089`
		//  Estimated: `69046`
		// Minimum execution time: 117_743_000 picoseconds.
		Weight::from_parts(120_083_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `FellowshipCollective::Members` (r:1 w:0)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:0 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn offboard() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `431`
		//  Estimated: `3514`
		// Minimum execution time: 16_957_000 picoseconds.
		Weight::from_parts(17_554_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCollective::Members` (r:1 w:0)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	fn import() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `385`
		//  Estimated: `3514`
		// Minimum execution time: 14_942_000 picoseconds.
		Weight::from_parts(15_213_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `FellowshipCollective::Members` (r:1 w:0)
	/// Proof: `FellowshipCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::Member` (r:1 w:1)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:1 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn approve() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66067`
		//  Estimated: `69046`
		// Minimum execution time: 105_255_000 picoseconds.
		Weight::from_parts(107_742_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `FellowshipCore::Member` (r:1 w:0)
	/// Proof: `FellowshipCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `FellowshipCore::MemberEvidence` (r:1 w:1)
	/// Proof: `FellowshipCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn submit_evidence() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `151`
		//  Estimated: `69046`
		// Minimum execution time: 89_506_000 picoseconds.
		Weight::from_parts(91_022_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
