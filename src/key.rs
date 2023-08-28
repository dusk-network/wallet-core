// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utilities to derive keys from the seed.

use crate::{utils, RNG_SEED};

use dusk_bls12_381_sign::SecretKey;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};

/// Generates a secret spend key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_ssk(seed: &[u8; RNG_SEED], index: u64) -> SecretSpendKey {
    SecretSpendKey::random(&mut utils::rng_with_index(seed, index))
}

/// Generates a secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
pub fn derive_sk(seed: &[u8; RNG_SEED], index: u64) -> SecretKey {
    SecretKey::random(&mut utils::rng_with_index(seed, index))
}

/// Generates a public spend key from its seed and index.
///
/// The secret spend key is derived from [derive_ssk], and then the key is
/// generated via [SecretSpendKey::public_spend_key].
pub fn derive_psk(seed: &[u8; RNG_SEED], index: u64) -> PublicSpendKey {
    derive_ssk(seed, index).public_spend_key()
}

/// Generates a view key from its seed and index.
///
/// The secret spend key is derived from [derive_ssk], and then the key is
/// generated via [SecretSpendKey::view_key].
pub fn derive_vk(seed: &[u8; RNG_SEED], index: u64) -> ViewKey {
    derive_ssk(seed, index).view_key()
}
