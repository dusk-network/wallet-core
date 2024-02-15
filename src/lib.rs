// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate alloc;

#[cfg(feature = "compat")]
/// compat module adds compatiblity functions for non rust platforms
pub mod compat;
pub mod ffi;
pub mod key;
pub mod tx;
pub mod types;
pub mod utils;
/// The maximum number of keys (inclusive) to derive when attempting to decrypt
/// a note.
pub const MAX_KEY: usize = 3;

/// The maximum allocated buffer for rkyv serialization.
pub const MAX_LEN: usize = rusk_abi::ARGBUF_LEN;

/// Length of the seed of the generated rng.
pub const RNG_SEED: usize = 64;

/// The length of the allocated response.
pub const RESPONSE_LEN: usize = 3 * i32::BITS as usize / 8;

/// The maximum number of input notes that are sent with the transaction
pub const MAX_INPUT_NOTES: usize = 4;
