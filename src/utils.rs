// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Misc utilities required by the library implementation.

use crate::tx;

use alloc::vec::Vec;

use phoenix_core::Note;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

/// Length of the seed of the generated rng.
pub const RNG_SEED: usize = 64;

/// Creates a secure RNG from a seed.
pub fn rng(seed: &[u8; RNG_SEED]) -> ChaCha12Rng {
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(b"RNG");

    let hash = hash.finalize().into();
    ChaCha12Rng::from_seed(hash)
}

/// Creates a secure RNG from a seed with embedded index.
pub fn rng_with_index(seed: &[u8; RNG_SEED], index: u64) -> ChaCha12Rng {
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(index.to_le_bytes());
    hash.update(b"INDEX");

    let hash = hash.finalize().into();
    ChaCha12Rng::from_seed(hash)
}

/// Sanitize a notes input into a consumable notes set
pub fn sanitize_notes(mut notes: Vec<Note>) -> Vec<Note> {
    notes.sort_by_key(|n| n.hash());
    notes.dedup();
    notes
}

/// Perform a knapsack algorithm to define the notes to be used as input.
///
/// Returns a tuple containing (unspent, inputs). `unspent` contains the notes
/// that are not used.
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
    let available = vec![
        (note1.clone(), o, 100, 0),
        (note2.clone(), o, 500, 1),
        (note3.clone(), o, 300, 2),
    ];
    let unspent = vec![note1];
    let inputs = vec![(note2.clone(), o, 500, 1), (note3.clone(), o, 300, 2)];
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
    let available = vec![
        (note1.clone(), o, 100, 0),
        (note2.clone(), o, 500, 1),
        (note3.clone(), o, 300, 2),
    ];
    assert_eq!(knapsack(available, 901), None);
}
