// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bip39::Mnemonic;

use crate::{
    types,
    types::{GetMnemonicSeedArgs, MnemonicNewArgs},
    utils,
};

use alloc::string::ToString;

/// Create a new mnemonic randomized on the seed bytes provided
/// Its the host's job to provide a crypto
/// secure seed because we cannot generate a secure rng
/// in no_std
#[no_mangle]
pub fn new_mnemonic(args: i32, len: i32) -> i64 {
    let MnemonicNewArgs { rng_seed } = match utils::take_args(args, len) {
        Some(val) => val,
        None => return utils::fail(),
    };

    // check if we our seed is secure
    let bytes_check: [u8; 32] = match rng_seed.try_into().ok() {
        Some(bytes) => bytes,
        None => return utils::fail(),
    };

    let mnemonic = match Mnemonic::from_entropy(&bytes_check).ok() {
        Some(m) => m,
        None => return utils::fail_with(),
    };

    utils::into_ptr(types::MnewmonicNewResponse {
        mnemonic_string: mnemonic.to_string(),
    })
}

/// Get the wallet seed bytes [u8; 64] from the given normalized
/// passphrase and Mnemomnic
#[no_mangle]
pub fn get_mnemonic_seed(args: i32, len: i32) -> i64 {
    let GetMnemonicSeedArgs {
        mnemonic,
        passphrase,
    } = match utils::take_args(args, len) {
        Some(val) => val,
        None => return utils::fail(),
    };

    let mnemonic = match Mnemonic::parse_normalized(&mnemonic).ok() {
        Some(m) => m,
        None => return utils::fail(),
    };

    let seed = mnemonic.to_seed_normalized(&passphrase).to_vec();

    utils::into_ptr(types::GetMnemonicSeedResponse {
        mnemonic_seed: seed,
    })
}
