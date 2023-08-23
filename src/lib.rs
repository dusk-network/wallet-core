// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use alloc::vec::Vec;
use core::mem;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

pub mod ffi;
pub mod key;
pub mod tx;
pub mod utils;

/// The maximum number of keys to derive when attempting to decrypt a note.
pub const MAX_KEY: usize = 24;

/// The maximum allocated buffer for rkyv serialization.
pub const MAX_LEN: usize = rusk_abi::ARGBUF_LEN;

/// The arguments of the balance function.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct BalanceArgs {
    /// Seed used to derive the keys of the wallet.
    pub seed: [u8; utils::RNG_SEED],
    /// A rkyv serialized [Vec<phoenix_core::note::Note>]; all notes should
    /// have their keys derived from `seed`.
    pub notes: Vec<u8>,
}

/// The response of the balance function.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct BalanceResponse {
    /// Status of the execution
    pub success: bool,
    /// Total computed balance
    pub value: u64,
}

impl BalanceResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedBalanceResponse>();
}

/// The arguments of the execute function.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ExecuteArgs {
    /// Seed used to derive the keys of the wallet.
    pub seed: [u8; utils::RNG_SEED],
    /// Seed used to derive the entropy for the notes.
    pub rng_seed: [u8; utils::RNG_SEED],
    /// A rkyv serialized [Vec<phoenix_core::note::Note>] to be used as inputs
    pub inputs: Vec<u8>,
    /// A rkyv serialized [Vec<tx::Opening>] to open the inputs to a Merkle
    /// root.
    pub openings: Vec<u8>,
    /// A rkyv serialize [dusk_pki::PublicSpendKey] to whom the remainder
    /// balance will be refunded
    pub refund: Vec<u8>,
    /// A rkyv serialized [Option<tx::OutputValue>] to define the receiver.
    pub output: Vec<u8>,
    /// The [phoenix_core::Crossover] value; will be skipped if `0`.
    pub crossover: u64,
    /// The gas limit of the transaction.
    pub gas_limit: u64,
    /// The gas price per unit for the transaction.
    pub gas_price: u64,
    /// A rkyv serialized [Option<tx::CallData>] to perform contract calls.
    pub call: Vec<u8>,
}

/// The response of the execute function.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct ExecuteResponse {
    /// Status of the execution
    pub success: bool,
    /// The pointer to a rkyv serialized [Vec<phoenix_core::note::Note>>]
    /// containing the notes that weren't used.
    pub unspent_ptr: u64,
    /// The length of the rkyv serialized `unspent_ptr`.
    pub unspent_len: u64,
    /// The pointer to a rkyv serialized [tx::UnspentTransaction].
    pub tx_ptr: u64,
    /// The length of the rkyv serialized `tx_ptr`.
    pub tx_len: u64,
}

impl ExecuteResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedExecuteResponse>();
}
