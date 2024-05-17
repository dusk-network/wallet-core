// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utilities to derive keys from the seed.

use crate::{utils, RNG_SEED};

use bls12_381_bls::SecretKey as StakeSecretKey;
use phoenix_core::{PublicKey, SecretKey, ViewKey};

/// Generates a stake secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_stake_sk(seed: &[u8; RNG_SEED], index: u64) -> StakeSecretKey {
    StakeSecretKey::random(&mut utils::rng_with_index(seed, index, b"SK"))
}

/// Generates a secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_sk(seed: &[u8; RNG_SEED], index: u64) -> SecretKey {
    SecretKey::random(&mut utils::rng_with_index(seed, index, b"SSK"))
}

/// Generates a public key from its seed and index.
///
/// First the secret key is derived with [`derive_sk`], then the public key
/// is generated from it and the secret key is erased from memory.
pub fn derive_pk(seed: &[u8; RNG_SEED], index: u64) -> PublicKey {
    let sk = derive_sk(seed, index);
    PublicKey::from(&sk)
}

/// Generates a view key from its seed and index.
///
/// First the secret key is derived with [`derive_sk`], then the view key is
/// generated from it and the secret key is erased from memory.
pub fn derive_vk(seed: &[u8; RNG_SEED], index: u64) -> ViewKey {
    let sk = derive_sk(seed, index);
    ViewKey::from(&sk)
}
