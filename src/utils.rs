// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Misc utilities required by the library implementation.

use crate::{ffi, tx, MAX_INPUT_NOTES, MAX_LEN, RNG_SEED};

use alloc::vec::Vec;
use core::ptr;

use dusk_bytes::DeserializableSlice;
use dusk_jubjub::JubJubScalar;
use phoenix_core::{Note, PublicKey};
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type Node = (Note, tx::Opening, u64, JubJubScalar);
const MAX_ALLOC_LEN: u32 = 2u32.pow(24);

/// Composes a `i64` from the provided arguments. This will be returned from the
/// WASM module functions.
pub fn compose(success: bool, ptr: u32, len: u32) -> i64 {
    assert!(
        len <= MAX_ALLOC_LEN,
        "len must be less than {MAX_ALLOC_LEN}"
    );

    let success = (!success) as u64;
    let ptr = (ptr as u64) << 32;
    let len = ((len as u64) << 40) >> 32;

    (success | ptr | len) as i64
}

/// Decomposes a `i64` into its inner arguments, being:
///
/// - status: a boolean indicating the success of the operation
/// - ptr: a pointer to the underlying data
/// - len: the length of the underlying data
pub fn decompose(result: i64) -> (bool, u32, u32) {
    let ptr = (result >> 32) as u32;
    let len = ((result & 0xFFFFFFF0) >> 8) as u32;
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
    let args: Vec<u8> = unsafe { Vec::from_raw_parts(args, len, len) };
    let args = alloc::string::String::from_utf8(args).ok()?;
    serde_json::from_str(&args).ok()
}

/// reads the raw bytes at the pointer for the length and returns what it reason
pub fn take_args_raw<'a>(args: i32, len: i32) -> &'a [u8] {
    let args = args as *mut u8;
    let len = len as usize;

    unsafe { core::slice::from_raw_parts(args, len) }
}

/// Sanitizes arbitrary bytes into well-formed seed.
pub fn sanitize_seed(bytes: Vec<u8>) -> Option<[u8; RNG_SEED]> {
    (bytes.len() == RNG_SEED).then(|| {
        let mut seed = [0u8; RNG_SEED];
        seed.copy_from_slice(&bytes);
        seed
    })
}

/// Sanitizes arbitrary bytes into well-formed seed.
pub fn sanitize_rng_seed(bytes: Vec<u8>) -> Option<[u8; 32]> {
    (bytes.len() == 32).then(|| {
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        seed
    })
}

/// Fails the operation
pub fn fail() -> i64 {
    compose(false, 0, 0)
}

/// Converts the provided response into an allocated pointer and returns the
/// composed success value.
pub fn into_ptr<T>(response: T) -> i64
where
    T: Serialize,
{
    let response = serde_json::to_string(&response).unwrap_or_default().leak();
    let (ptr, len) = allocated_copy(response);
    compose(true, ptr as _, len as _)
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

    let (ptr, len) = allocated_copy(bytes);

    compose(true, ptr, len)
}

/// Allocated a new buffer, copies the provided bytes to it, and returns the
/// pointer and length of the new buffer.
pub fn allocated_copy<B: AsRef<[u8]>>(bytes: B) -> (u32, u32) {
    unsafe {
        let bytes = bytes.as_ref();
        let len = bytes.len();

        let ptr = ffi::allocate(bytes.len() as _);
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as _, len);

        (ptr as _, len as _)
    }
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

/// Converts a Base58 string into a [`PublicKey`].
pub fn bs58_to_pk(pk: &str) -> Option<PublicKey> {
    // TODO this should be defined in phoenix-core
    let bytes = bs58::decode(pk).into_vec().ok()?;
    PublicKey::from_reader(&mut &bytes[..]).ok()
}

/// Calculate the inputs for a transaction.
pub fn inputs(nodes: Vec<Node>, target_sum: u64) -> Option<Vec<Node>> {
    if nodes.is_empty() {
        return None;
    }

    let mut i = 0;
    let mut sum = 0;
    while sum < target_sum && i < nodes.len() {
        sum = sum.saturating_add(nodes[i].2);
        i += 1;
    }

    if sum < target_sum {
        return None;
    }

    let inputs = pick_notes(target_sum, nodes);

    Some(inputs)
}

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The notes are picked in a way to maximize the number of notes used, while
/// minimizing the value employed. To do this we sort the notes in ascending
/// value order, and go through each combination in a lexicographic order
/// until we find the first combination whose sum is larger or equal to
/// the given value. If such a slice is not found, an empty vector is returned.
///
/// Note: it is presupposed that the input notes contain enough balance to cover
/// the given `value`.
fn pick_notes(value: u64, notes_and_values: Vec<Node>) -> Vec<Node> {
    let mut notes_and_values = notes_and_values;
    let len = notes_and_values.len();

    if len <= MAX_INPUT_NOTES {
        return notes_and_values;
    }

    notes_and_values.sort_by(|(_, _, aval, _), (_, _, bval, _)| aval.cmp(bval));

    pick_lexicographic(notes_and_values.len(), |indices| {
        indices
            .iter()
            .map(|index| notes_and_values[*index].2)
            .sum::<u64>()
            >= value
    })
    .map(|indices| {
        indices
            .into_iter()
            .map(|index| notes_and_values[index])
            .collect()
    })
    .unwrap_or_default()
}

fn pick_lexicographic<F: Fn(&[usize; MAX_INPUT_NOTES]) -> bool>(
    max_len: usize,
    is_valid: F,
) -> Option<[usize; MAX_INPUT_NOTES]> {
    let mut indices = [0; MAX_INPUT_NOTES];
    indices
        .iter_mut()
        .enumerate()
        .for_each(|(i, index)| *index = i);

    loop {
        if is_valid(&indices) {
            return Some(indices);
        }

        let mut i = MAX_INPUT_NOTES - 1;

        while indices[i] == i + max_len - MAX_INPUT_NOTES {
            if i > 0 {
                i -= 1;
            } else {
                break;
            }
        }

        indices[i] += 1;
        for j in i + 1..MAX_INPUT_NOTES {
            indices[j] = indices[j - 1] + 1;
        }

        if indices[MAX_INPUT_NOTES - 1] == max_len {
            break;
        }
    }

    None
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
    use ff::Field;
    use phoenix_core::{PublicKey, SecretKey};
    use rand::{rngs::StdRng, SeedableRng};

    // openings are not checked here; no point in setting them up properly
    let o = unsafe { mem::zeroed() };
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    // sanity check
    assert_eq!(inputs(vec![], 70), None);

    // basic check
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder = JubJubScalar::random(&mut *rng);
    let note = Note::obfuscated(rng, &pk, 100, blinder);
    let available = vec![(note, o, 100, blinder)];
    let inputs_notes = available.clone();
    assert_eq!(inputs(available, 70), Some(inputs_notes));

    // out of balance basic check
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder = JubJubScalar::random(&mut *rng);
    let note = Note::obfuscated(rng, &pk, 100, blinder);
    let available = vec![(note, o, 100, blinder)];
    assert_eq!(inputs(available, 101), None);

    // multiple inputs check
    // note: this test is checking a naive, simple order-based output
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder1 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder1?
    let note1 = Note::obfuscated(rng, &pk, 100, blinder);
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder2 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder2?
    let note2 = Note::obfuscated(rng, &pk, 500, blinder);
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder3 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder3?
    let note3 = Note::obfuscated(rng, &pk, 300, blinder);
    let available = vec![
        (note1, o, 100, blinder1),
        (note2, o, 500, blinder2),
        (note3, o, 300, blinder3),
    ];

    assert_eq!(inputs(available.clone(), 600), Some(available));

    // multiple inputs, out of balance check
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder1 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder1?
    let note1 = Note::obfuscated(rng, &pk, 100, blinder);
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder2 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder2?
    let note2 = Note::obfuscated(rng, &pk, 500, blinder);
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);
    let blinder3 = JubJubScalar::random(&mut *rng);
    // shouldn't this note be created with blinder3?
    let note3 = Note::obfuscated(rng, &pk, 300, blinder);
    let available = vec![
        (note1, o, 100, blinder1),
        (note2, o, 500, blinder2),
        (note3, o, 300, blinder3),
    ];
    assert_eq!(inputs(available, 901), None);
}
