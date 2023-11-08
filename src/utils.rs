// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Misc utilities required by the library implementation.

use crate::{tx, MAX_LEN, RNG_SEED};

use alloc::vec::Vec;
use core::mem;

use dusk_bytes::DeserializableSlice;
use dusk_pki::PublicSpendKey;
use phoenix_core::Note;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Composes a `i64` from the provided arguments. This will be returned from the
/// WASM module functions.
pub const fn compose(success: bool, ptr: u32, len: u32) -> i64 {
    let success = (!success) as u64;
    let ptr = (ptr as u64) << 32;
    let len = ((len as u64) << 48) >> 32;
    (success | ptr | len) as i64
}

/// Decomposes a `i64` into its inner arguments, being:
///
/// - status: a boolean indicating the success of the operation
/// - ptr: a pointer to the underlying data
/// - len: the length of the underlying data
pub const fn decompose(result: i64) -> (bool, u64, u64) {
    let ptr = (result >> 32) as u64;
    let len = ((result << 32) >> 48) as u64;
    let success = ((result << 63) >> 63) == 0;

    (success, ptr, len)
}

/// Takes a JSON string from the memory slice and deserializes it into the
/// provided type.
pub fn take_args<T>(args: i32, len: i32) -> Option<T>
where
    T: for<'a> Deserialize<'a>,
{
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };
    let args = alloc::string::String::from_utf8(args).ok()?;
    serde_json::from_str(&args).ok()
}

/// Sanitizes arbitrary bytes into well-formed seed.
pub fn sanitize_seed(bytes: Vec<u8>) -> Option<[u8; RNG_SEED]> {
    (bytes.len() == RNG_SEED).then(|| {
        let mut seed = [0u8; RNG_SEED];
        seed.copy_from_slice(&bytes);
        seed
    })
}

/// Fails the operation
pub const fn fail() -> i64 {
    compose(false, 0, 0)
}

/// Converts the provided response into an allocated pointer and returns the
/// composed success value.
pub fn into_ptr<T>(response: T) -> i64
where
    T: Serialize,
{
    let response = serde_json::to_string(&response).unwrap_or_default();
    let ptr = response.as_ptr() as u32;
    let len = response.len() as u32;
    let result = compose(true, ptr, len);

    mem::forget(response);

    result
}

/// Returns the provided bytes as a pointer
pub fn rkyv_into_ptr<T>(value: T) -> i64
where
    T: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<MAX_LEN>>,
{
    let bytes = match rkyv::to_bytes(&value) {
        Ok(t) => t.into_vec(),
        Err(_) => return fail(),
    };

    let ptr = bytes.as_ptr() as u32;
    let len = bytes.len() as u32;

    mem::forget(bytes);
    compose(true, ptr, len)
}

/// Creates a secure RNG directly a seed.
pub fn rng(seed: [u8; 32]) -> ChaCha12Rng {
    ChaCha12Rng::from_seed(seed)
}

/// Creates a secure RNG from a seed with embedded index.
pub fn rng_with_index(
    seed: &[u8; RNG_SEED],
    index: u64,
    termination: &[u8],
) -> ChaCha12Rng {
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(index.to_le_bytes());
    hash.update(termination);

    let hash = hash.finalize().into();
    ChaCha12Rng::from_seed(hash)
}

/// Sanitize a notes input into a consumable notes set
pub fn sanitize_notes(mut notes: Vec<Note>) -> Vec<Note> {
    notes.sort_by_key(|n| n.hash());
    notes.dedup();
    notes
}

/// Converts a Base58 string into a [PublicSpendKey].
pub fn bs58_to_psk(psk: &str) -> Option<PublicSpendKey> {
    // TODO this should be defined in dusk-pki
    let bytes = bs58::decode(psk).into_vec().ok()?;
    PublicSpendKey::from_reader(&mut &bytes[..]).ok()
}

/// Perform a knapsack algorithm to define the notes to be used as input.
///
/// Returns a tuple containing (unspent, inputs). `unspent` contains the notes
/// that are not used.
#[allow(clippy::type_complexity)]
pub fn knapsack(
    mut nodes: Vec<(Note, tx::Opening, u64, usize)>,
    target_sum: u64,
) -> Option<(Vec<Note>, Vec<(Note, tx::Opening, u64, usize)>)> {
    if nodes.is_empty() {
        return None;
    }

    // TODO implement a knapsack algorithm
    // here we do a naive, desc order pick. optimally, we should maximize the
    // number of smaller inputs that fits the target sum so we reduce the number
    // of available small notes on the wallet. a knapsack implementation is
    // optimal for such problems as it can deliver high confidence results
    // with moderate memory space.
    nodes.sort_by(|a, b| b.2.cmp(&a.2));

    let mut i = 0;
    let mut sum = 0;
    while sum < target_sum && i < nodes.len() {
        sum = sum.saturating_add(nodes[i].2);
        i += 1;
    }

    if sum < target_sum {
        return None;
    }
    let unspent = nodes.split_off(i).into_iter().map(|n| n.0).collect();

    Some((unspent, nodes))
}

#[test]
fn compose_works() {
    assert_eq!(decompose(compose(true, 0, 0)), (true, 0, 0));
    assert_eq!(decompose(compose(false, 0, 0)), (false, 0, 0));
    assert_eq!(decompose(compose(false, 1, 0)), (false, 1, 0));
    assert_eq!(decompose(compose(false, 0, 1)), (false, 0, 1));
    assert_eq!(decompose(compose(false, 4837, 383)), (false, 4837, 383));
}

#[test]
fn knapsack_works() {
    use core::mem;
    use dusk_jubjub::JubJubScalar;
    use dusk_pki::SecretSpendKey;
    use rand::{rngs::StdRng, SeedableRng};

    // openings are not checked here; no point in setting them up properly
    let o = unsafe { mem::zeroed() };
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    // sanity check
    assert_eq!(knapsack(vec![], 70), None);

    // basic check
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &key.public_spend_key(), 100, blinder);
    let available = vec![(note, o, 100, 0)];
    let unspent = vec![];
    let inputs = available.clone();
    assert_eq!(knapsack(available, 70), Some((unspent, inputs)));

    // out of balance basic check
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &key.public_spend_key(), 100, blinder);
    let available = vec![(note, o, 100, 0)];
    assert_eq!(knapsack(available, 101), None);

    // multiple inputs check
    // note: this test is checking a naive, simple order-based output
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note1 = Note::obfuscated(rng, &key.public_spend_key(), 100, blinder);
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note2 = Note::obfuscated(rng, &key.public_spend_key(), 500, blinder);
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note3 = Note::obfuscated(rng, &key.public_spend_key(), 300, blinder);
    let available =
        vec![(note1, o, 100, 0), (note2, o, 500, 1), (note3, o, 300, 2)];
    let unspent = vec![note1];
    let inputs = vec![(note2, o, 500, 1), (note3, o, 300, 2)];
    assert_eq!(knapsack(available, 600), Some((unspent, inputs)));

    // multiple inputs, out of balance check
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note1 = Note::obfuscated(rng, &key.public_spend_key(), 100, blinder);
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note2 = Note::obfuscated(rng, &key.public_spend_key(), 500, blinder);
    let key = SecretSpendKey::random(rng);
    let blinder = JubJubScalar::random(rng);
    let note3 = Note::obfuscated(rng, &key.public_spend_key(), 300, blinder);
    let available =
        vec![(note1, o, 100, 0), (note2, o, 500, 1), (note3, o, 300, 2)];
    assert_eq!(knapsack(available, 901), None);
}
