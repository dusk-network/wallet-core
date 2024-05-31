// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! FFI bindings exposed to WASM module.

use alloc::{
    alloc::{alloc, dealloc, Layout},
    vec::Vec,
};
use core::mem;

use dusk_bytes::Serializable;
use phoenix_core::{Fee, Note, ViewKey};
use sha2::{Digest, Sha512};

use crate::{key, tx, types, utils, MAX_KEY, MAX_LEN};

/// The alignment of the memory allocated by the FFI.
///
/// This is 1 because we're not allocating any complex data structures, and
/// just interacting with the memory directly.
const ALIGNMENT: usize = 1;

/// Allocates a buffer of `len` bytes on the WASM memory.
#[no_mangle]
pub fn allocate(len: i32) -> i32 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        let ptr = alloc(layout);
        ptr as _
    }
}

/// Frees a previously allocated buffer on the WASM memory.
#[no_mangle]
pub fn free_mem(ptr: i32, len: i32) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        dealloc(ptr as _, layout);
    }
}

/// Computes a secure seed from the given passphrase.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::SeedArgs].
///
/// Will return a triplet (status, ptr, len) pointing to the seed.
#[no_mangle]
pub fn seed(args: i32, len: i32) -> i64 {
    let types::SeedArgs { passphrase } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let mut hash = Sha512::new();

    hash.update(passphrase);
    hash.update(b"SEED");

    let seed = hash.finalize().to_vec();

    let (ptr, len) = utils::allocated_copy(seed);
    utils::compose(true, ptr as _, len as _)
}

/// Computes the total balance of the given notes.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::BalanceArgs].
///
/// Will return a triplet (status, ptr, len) pointing to JSON string
/// representing [types::BalanceResult].
#[no_mangle]
#[allow(clippy::needless_range_loop)]
pub fn balance(args: i32, len: i32) -> i64 {
    let types::BalanceArgs { notes, seed } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => utils::sanitize_notes(n),
        Err(_) => return utils::fail(),
    };

    let mut keys = unsafe { [mem::zeroed(); MAX_KEY] };
    let mut values = Vec::with_capacity(notes.len());
    let mut keys_len = 0;
    let mut sum = 0u64;

    'outer: for note in notes {
        // we iterate all the available keys until one can successfully decrypt
        // the note. if all fails, returns false
        for idx in 0..MAX_KEY {
            if keys_len == idx {
                keys[idx] = key::derive_vk(&seed, idx as u64);
                keys_len += 1;
            }

            if let Ok(v) = note.value(Some(&keys[idx])) {
                values.push(v);
                sum = sum.saturating_add(v);
                continue 'outer;
            }
        }

        return utils::fail();
    }

    // the top 4 notes are the maximum value a transaction can have, given the
    // circuit accepts up to 4 inputs
    values.sort_by(|a, b| b.cmp(a));
    let maximum = values.iter().take(4).sum::<u64>();

    utils::into_ptr(types::BalanceResponse {
        maximum,
        value: sum,
    })
}

/// Computes a serialized unproven transaction from the given arguments.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::ExecuteArgs].
///
/// Will return a triplet (status, ptr, len) pointing to JSON string
/// representing [types::ExecuteResponse].
#[no_mangle]
pub fn execute(args: i32, len: i32) -> i64 {
    let types::ExecuteArgs {
        call,
        crossover,
        fee,
        inputs,
        openings,
        output,
        gas_limit,
        gas_price,
        refund,
        rng_seed,
        sender_index,
        seed,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let inputs: Vec<Note> = match rkyv::from_bytes(&inputs) {
        Ok(n) => utils::sanitize_notes(n),
        Err(_) => return utils::fail(),
    };

    let fee: Option<Fee> =
        fee.and_then(|fee| match rkyv::from_bytes::<Fee>(&fee) {
            Ok(n) => Some(n),
            Err(_) => None,
        });

    let openings: Vec<(tx::Opening, u64)> = match rkyv::from_bytes(&openings) {
        Ok(n) => n,
        Err(_) => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let rng_seed: [u8; 32] = match utils::sanitize_rng_seed(rng_seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let value = output.as_ref().map(|o| o.value).unwrap_or(0);
    let total_output = gas_limit
        .saturating_mul(gas_price)
        .saturating_add(value)
        .saturating_add(crossover.clone().map(|c| c.value).unwrap_or_default());

    let mut full_inputs = Vec::with_capacity(inputs.len());

    let sk = key::derive_sk(&seed, sender_index);
    let vk = ViewKey::from(&sk);

    'outer: for input in inputs {
        if let Ok(value) = input.value(Some(&vk)) {
            let opening =
                match openings.iter().find(|(_, pos)| input.pos() == pos) {
                    Some(a) => a.0,
                    None => {
                        return utils::fail();
                    }
                };

            let blinder = match input.blinding_factor(Some(&vk)).ok() {
                Some(a) => a,
                None => return utils::fail(),
            };

            full_inputs.push((input, opening, value, blinder));
            continue 'outer;
        }

        return utils::fail();
    }

    // optimizes the inputs given the total amount
    let inputs = match utils::inputs(full_inputs, total_output) {
        Some(k) => k,
        None => return utils::fail(),
    };

    let inputs: Vec<_> = inputs
        .into_iter()
        .map(|(note, opening, value, _)| tx::PreInput {
            note,
            opening,
            value,
            sk: &sk,
        })
        .collect();

    let total_input: u64 = inputs.iter().map(|i| i.value).sum();
    let total_refund = total_input.saturating_sub(total_output);

    let mut outputs = Vec::with_capacity(2);
    if total_refund > 0 {
        outputs.push(types::ExecuteOutput {
            note_type: types::OutputType::Obfuscated,
            receiver: refund.clone(),
            ref_id: None,
            value: total_refund,
        });
    }
    if let Some(o) = output {
        outputs.push(o);
    }

    let rng: &mut rand_chacha::ChaCha12Rng = &mut utils::rng(rng_seed);
    let actual_fee;
    let refund = match utils::bs58_to_pk(&refund) {
        Some(r) => r,
        None => return utils::fail(),
    };

    if let Some(fee) = fee {
        actual_fee = fee;
    } else {
        actual_fee = Fee::new(rng, gas_limit, gas_price, &refund);
    }

    let tx = tx::UnprovenTransaction::new(
        rng, inputs, outputs, actual_fee, crossover, call,
    );

    let tx = match tx {
        Some(t) => t,
        None => return utils::fail(),
    };

    let tx = match rkyv::to_bytes::<tx::UnprovenTransaction, MAX_LEN>(&tx).ok()
    {
        Some(t) => t.to_vec(),
        None => return utils::fail(),
    };

    utils::into_ptr(types::ExecuteResponse { tx })
}

/// Merges many lists of serialized notes into a unique, sanitized set.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::MergeNotesArgs].
///
/// Will return a triplet (status, ptr, len) pointing to the rkyv serialized
/// [Vec<phoenix_core::Note>].
#[no_mangle]
pub fn merge_notes(args: i32, len: i32) -> i64 {
    let types::MergeNotesArgs { notes } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let mut list = Vec::with_capacity(10);
    for notes in notes {
        if !notes.is_empty() {
            match rkyv::from_bytes::<Vec<Note>>(&notes) {
                Ok(n) => list.extend(n),
                Err(_) => return utils::fail(),
            };
        }
    }

    let notes = utils::sanitize_notes(list);

    utils::rkyv_into_ptr(notes)
}

/// Filters a list of notes from a list of negative flags. The flags that are
/// `true` will represent a note that must be removed from the set.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::FilterNotesArgs].
///
/// Will return a triplet (status, ptr, len) pointing to the rkyv serialized
/// [Vec<phoenix_core::Note>].
#[no_mangle]
pub fn filter_notes(args: i32, len: i32) -> i64 {
    let types::FilterNotesArgs { flags, notes } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => n,
        Err(_) => return utils::fail(),
    };

    let notes: Vec<_> = notes
        .into_iter()
        .zip(flags)
        .filter_map(|(n, f)| (!f).then_some(n))
        .collect();

    let notes = utils::sanitize_notes(notes);
    utils::rkyv_into_ptr(notes)
}

/// Returns a list of [`PublicKey`]s that belongs to this wallet.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::PublicKeysArgs].
///
/// Will return a triplet (status, ptr, len) pointing to JSON string
/// representing [types::PublicKeysResponse].
#[no_mangle]
pub fn public_keys(args: i32, len: i32) -> i64 {
    let types::PublicKeysArgs { seed } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let keys = (0..MAX_KEY)
        .map(|idx| key::derive_pk(&seed, idx as u64))
        .map(|pk| bs58::encode(pk.to_bytes()).into_string())
        .collect();

    utils::into_ptr(types::PublicKeysResponse { keys })
}

/// Returns a list of [ViewKey] that belongs to this wallet.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::ViewKeysArgs].
///
/// Will return a triplet (status, ptr, len) pointing to the rkyv serialized
/// [`Vec<ViewKey>`].
#[no_mangle]
pub fn view_keys(args: i32, len: i32) -> i64 {
    let types::ViewKeysArgs { seed } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let keys: Vec<_> = (0..MAX_KEY)
        .map(|idx| key::derive_vk(&seed, idx as u64))
        .collect();

    utils::rkyv_into_ptr(keys)
}

/// Returns a list of [BlsScalar] nullifiers for the given [Vec<Note>] combined
/// with the keys of this wallet.
///
/// Expects as argument a fat pointer to a JSON string representing
/// [types::NullifiersArgs].
///
/// Will return a triplet (status, ptr, len) pointing to the rkyv serialized
/// [Vec<dusk_jubjub::BlsScalar>].
#[no_mangle]
pub fn nullifiers(args: i32, len: i32) -> i64 {
    let types::NullifiersArgs { notes, seed } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => n,
        Err(_) => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let mut nullifiers = Vec::with_capacity(notes.len());
    let mut sks = unsafe { [mem::zeroed(); MAX_KEY] };
    let mut vks = unsafe { [mem::zeroed(); MAX_KEY] };
    let mut keys_len = 0;

    'outer: for note in notes {
        // we iterate all the available view key until one can successfully
        // decrypt the note. if any fails, returns false
        for idx in 0..MAX_KEY {
            if keys_len == idx {
                sks[idx] = key::derive_sk(&seed, idx as u64);
                vks[idx] = ViewKey::from(&sks[idx]);
                keys_len += 1;
            }

            if vks[idx].owns(&note) {
                nullifiers.push(note.gen_nullifier(&sks[idx]));
                continue 'outer;
            }
        }

        return utils::fail();
    }

    utils::rkyv_into_ptr(nullifiers)
}
