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

//! Autogenerated weights for `runtime_common::crowdloan`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("../kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=../kusama-chain-spec.json
// --steps
// 50
// --repeat
// 20
// --pallet=runtime_common::crowdloan
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output
// ./kusama-weights/
// --header
// ./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `runtime_common::crowdloan`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> runtime_common::crowdloan::WeightInfo for WeightInfo<T> {
	/// Storage: `Crowdloan::Funds` (r:1 w:1)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Registrar::Paras` (r:1 w:0)
	/// Proof: `Registrar::Paras` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Paras::ParaLifecycles` (r:1 w:0)
	/// Proof: `Paras::ParaLifecycles` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdloan::NextFundIndex` (r:1 w:1)
	/// Proof: `Crowdloan::NextFundIndex` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn create() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `415`
		//  Estimated: `3880`
		// Minimum execution time: 56_969_000 picoseconds.
		Weight::from_parts(64_965_000, 0)
			.saturating_add(Weight::from_parts(0, 3880))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:1)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Slots::Leases` (r:1 w:0)
	/// Proof: `Slots::Leases` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Auctions::AuctionInfo` (r:1 w:0)
	/// Proof: `Auctions::AuctionInfo` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Crowdloan::EndingsCount` (r:1 w:0)
	/// Proof: `Crowdloan::EndingsCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdloan::NewRaise` (r:1 w:1)
	/// Proof: `Crowdloan::NewRaise` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: UNKNOWN KEY `0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291` (r:1 w:1)
	fn contribute() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `529`
		//  Estimated: `3994`
		// Minimum execution time: 129_936_000 picoseconds.
		Weight::from_parts(139_635_000, 0)
			.saturating_add(Weight::from_parts(0, 3994))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:1)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xc85982571aa615c788ef9b2c16f54f25773fd439e8ee1ed2aa3ae43d48e880f0` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0xc85982571aa615c788ef9b2c16f54f25773fd439e8ee1ed2aa3ae43d48e880f0` (r:1 w:1)
	fn withdraw() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `686`
		//  Estimated: `6196`
		// Minimum execution time: 79_290_000 picoseconds.
		Weight::from_parts(91_409_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Skipped::Metadata` (r:0 w:0)
	/// Proof: `Skipped::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `k` is `[0, 1000]`.
	fn refund(k: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `124 + k * (189 ±0)`
		//  Estimated: `137 + k * (189 ±0)`
		// Minimum execution time: 74_780_000 picoseconds.
		Weight::from_parts(84_658_000, 0)
			.saturating_add(Weight::from_parts(0, 137))
			// Standard Error: 25_249
			.saturating_add(Weight::from_parts(39_464_813, 0).saturating_mul(k.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(k.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(k.into())))
			.saturating_add(Weight::from_parts(0, 189).saturating_mul(k.into()))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:1)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn dissolve() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `514`
		//  Estimated: `6196`
		// Minimum execution time: 41_614_000 picoseconds.
		Weight::from_parts(43_513_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:1)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn edit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `3699`
		// Minimum execution time: 21_080_000 picoseconds.
		Weight::from_parts(25_802_000, 0)
			.saturating_add(Weight::from_parts(0, 3699))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:0)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: UNKNOWN KEY `0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291` (r:1 w:1)
	fn add_memo() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `411`
		//  Estimated: `3876`
		// Minimum execution time: 31_369_000 picoseconds.
		Weight::from_parts(35_823_000, 0)
			.saturating_add(Weight::from_parts(0, 3876))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Crowdloan::Funds` (r:1 w:0)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdloan::NewRaise` (r:1 w:1)
	/// Proof: `Crowdloan::NewRaise` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn poke() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `238`
		//  Estimated: `3703`
		// Minimum execution time: 21_053_000 picoseconds.
		Weight::from_parts(22_171_000, 0)
			.saturating_add(Weight::from_parts(0, 3703))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Auctions::AuctionInfo` (r:1 w:0)
	/// Proof: `Auctions::AuctionInfo` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Crowdloan::EndingsCount` (r:1 w:1)
	/// Proof: `Crowdloan::EndingsCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdloan::NewRaise` (r:1 w:1)
	/// Proof: `Crowdloan::NewRaise` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Crowdloan::Funds` (r:100 w:0)
	/// Proof: `Crowdloan::Funds` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Auctions::AuctionCounter` (r:1 w:0)
	/// Proof: `Auctions::AuctionCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Paras::ParaLifecycles` (r:100 w:0)
	/// Proof: `Paras::ParaLifecycles` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Slots::Leases` (r:100 w:0)
	/// Proof: `Slots::Leases` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Auctions::Winning` (r:1 w:1)
	/// Proof: `Auctions::Winning` (`max_values`: None, `max_size`: Some(1920), added: 4395, mode: `MaxEncodedLen`)
	/// Storage: `Auctions::ReservedAmounts` (r:100 w:100)
	/// Proof: `Auctions::ReservedAmounts` (`max_values`: None, `max_size`: Some(60), added: 2535, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:100 w:100)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[2, 100]`.
	fn on_initialize(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `196 + n * (356 ±0)`
		//  Estimated: `5385 + n * (2832 ±0)`
		// Minimum execution time: 123_688_000 picoseconds.
		Weight::from_parts(8_501_333, 0)
			.saturating_add(Weight::from_parts(0, 5385))
			// Standard Error: 45_641
			.saturating_add(Weight::from_parts(54_574_136, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().reads((5_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2832).saturating_mul(n.into()))
	}
}
