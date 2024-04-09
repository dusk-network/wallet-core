// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_wallet_core::{derive_sk, derive_stake_sk};

const SEED: [u8; 64] = [0; 64];
const INDEX: u64 = 42;

#[test]
fn test_derive_sk() {
    // it is important that derive_sk always derives the same key from a seed
    let sk_bytes = [
        12, 16, 72, 188, 33, 76, 44, 178, 86, 123, 107, 153, 230, 149, 238,
        131, 87, 30, 94, 88, 52, 129, 247, 167, 30, 167, 163, 246, 68, 254, 14,
        9, 218, 135, 245, 104, 11, 190, 143, 129, 83, 202, 64, 179, 157, 248,
        175, 120, 157, 220, 98, 211, 141, 50, 224, 8, 1, 125, 29, 180, 206,
        195, 34, 0,
    ];
    assert_eq!(derive_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}

#[test]
fn test_derive_stake_sk() {
    // it is important that derive_stake_sk always derives the same key from a
    // seed
    let sk_bytes = [
        95, 35, 167, 191, 106, 171, 71, 158, 159, 39, 84, 1, 132, 238, 152,
        235, 154, 5, 250, 158, 255, 195, 79, 95, 193, 58, 36, 189, 0, 99, 230,
        86,
    ];
    assert_eq!(derive_stake_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}
