// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! FFI bindings exposed to WASM module.

use alloc::{vec, vec::Vec};
use core::mem;

use dusk_pki::PublicSpendKey;
use phoenix_core::note::{ArchivedNote, Note, NoteType};
use rkyv::validation::validators::FromBytesError;
use sha2::{Digest, Sha512};

use crate::{
    key, tx, utils, ArchivedBalanceResponse, ArchivedExecuteResponse,
    ArchivedFilterNotesResponse, ArchivedMergeNotesResponse,
    ArchivedNullifiersResponse, ArchivedSeedResponse, ArchivedViewKeysResponse,
    BalanceArgs, BalanceResponse, ExecuteArgs, ExecuteResponse,
    FilterNotesArgs, FilterNotesResponse, MergeNotesArgs, MergeNotesResponse,
    NullifiersArgs, NullifiersResponse, SeedArgs, SeedResponse, ViewKeysArgs,
    ViewKeysResponse, MAX_KEY, MAX_LEN,
};

/// Allocates a buffer of `len` bytes on the WASM memory.
#[no_mangle]
pub fn malloc(len: i32) -> i32 {
    let bytes = vec![0u8; len as usize];
    let ptr = bytes.as_ptr();
    mem::forget(bytes);
    ptr as i32
}

/// Frees a previously allocated buffer on the WASM memory.
#[no_mangle]
pub fn free_mem(ptr: i32, len: i32) {
    let ptr = ptr as *mut u8;
    let len = len as usize;
    unsafe {
        Vec::from_raw_parts(ptr, len, len);
    }
}

/// Computes a secure seed from the given passphrase.
///
/// The arguments are expected to be rkyv serialized [SeedArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn seed(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let SeedArgs { passphrase } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return SeedResponse::fail(),
    };

    let mut hash = Sha512::new();

    hash.update(passphrase);
    hash.update(b"SEED");

    let seed = hash.finalize().to_vec();
    let seed_ptr = seed.as_ptr() as u64;
    let seed_len = seed.len() as u64;

    mem::forget(seed);

    SeedResponse::success(seed_ptr, seed_len)
}

/// Computes the total balance of the given notes.
///
/// The arguments are expected to be rkyv serialized [BalanceArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn balance(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let BalanceArgs { seed, notes } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return BalanceResponse::fail(),
    };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => utils::sanitize_notes(n),
        Err(_) => return BalanceResponse::fail(),
    };

    let mut keys = unsafe { [mem::zeroed(); MAX_KEY + 1] };
    let mut values = Vec::with_capacity(notes.len());
    let mut keys_len = 0;
    let mut sum = 0u64;

    'outer: for note in notes {
        // we iterate all the available keys until one can successfully decrypt
        // the note. if all fails, returns false
        for idx in 0..=MAX_KEY {
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

        return BalanceResponse::fail();
    }

    // the top 4 notes are the maximum value a transaction can have, given the
    // circuit accepts up to 4 inputs
    values.sort_by(|a, b| b.cmp(a));
    let maximum = values.iter().take(4).sum::<u64>();

    BalanceResponse::success(sum, maximum)
}

/// Computes a serialized unproven transaction from the given arguments.
///
/// The arguments are expected to be rkyv serialized [ExecuteArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn execute(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };
    let args = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return ExecuteResponse::fail(),
    };

    fn inner(
        ExecuteArgs {
            seed,
            rng_seed,
            inputs,
            openings,
            refund,
            output,
            crossover,
            gas_limit,
            gas_price,
            call,
        }: ExecuteArgs,
    ) -> Option<(Vec<u8>, Vec<u8>)> {
        let inputs: Vec<Note> = rkyv::from_bytes(&inputs).ok()?;
        let inputs = utils::sanitize_notes(inputs);
        let openings: Vec<tx::Opening> = rkyv::from_bytes(&openings).ok()?;
        let refund: PublicSpendKey = rkyv::from_bytes(&refund).ok()?;
        let output: Option<tx::OutputValue> = rkyv::from_bytes(&output).ok()?;
        let call: Option<tx::CallData> = rkyv::from_bytes(&call).ok()?;

        let value = output.as_ref().map(|o| o.value).unwrap_or(0);
        let total_output =
            gas_limit.saturating_mul(gas_price).saturating_add(value);

        let mut keys = unsafe { [mem::zeroed(); MAX_KEY + 1] };
        let mut keys_ssk = unsafe { [mem::zeroed(); MAX_KEY + 1] };
        let mut keys_len = 0;
        let mut openings = openings.into_iter();
        let mut full_inputs = Vec::with_capacity(inputs.len());

        'outer: for input in inputs {
            // we iterate all the available keys until one can successfully
            // decrypt the note. if any fails, returns false
            for idx in 0..=MAX_KEY {
                if keys_len == idx {
                    keys_ssk[idx] = key::derive_ssk(&seed, idx as u64);
                    keys[idx] = keys_ssk[idx].view_key();
                    keys_len += 1;
                }

                if let Ok(value) = input.value(Some(&keys[idx])) {
                    let opening = openings.next()?;
                    full_inputs.push((input, opening, value, idx));
                    continue 'outer;
                }
            }

            return None;
        }

        // optimizes the inputs given the total amount
        let (unspent, inputs) = utils::knapsack(full_inputs, total_output)?;
        let inputs: Vec<_> = inputs
            .into_iter()
            .map(|(note, opening, value, idx)| tx::PreInput {
                note,
                opening,
                value,
                ssk: &keys_ssk[idx],
            })
            .collect();
        let total_input: u64 = inputs.iter().map(|i| i.value).sum();
        let total_refund = total_input.saturating_sub(total_output);

        let mut outputs: Vec<tx::OutputValue> = Vec::with_capacity(2);
        if let Some(o) = output {
            outputs.push(o);
        }
        if total_refund > 0 {
            outputs.push(tx::OutputValue {
                r#type: NoteType::Obfuscated,
                value: total_refund,
                receiver: refund,
                ref_id: 0,
            });
        }

        let rng = &mut utils::rng(&rng_seed);
        let tx = tx::UnprovenTransaction::new(
            rng, inputs, outputs, &refund, gas_limit, gas_price, crossover,
            call,
        )?;

        let unspent = rkyv::to_bytes::<_, MAX_LEN>(&unspent).ok()?.into_vec();
        let tx = rkyv::to_bytes::<_, MAX_LEN>(&tx).ok()?.into_vec();

        Some((unspent, tx))
    }

    let (unspent, tx) = match inner(args) {
        Some(t) => t,
        None => return ExecuteResponse::fail(),
    };

    let unspent_ptr = unspent.as_ptr() as u64;
    let unspent_len = unspent.len() as u64;
    let tx_ptr = tx.as_ptr() as u64;
    let tx_len = tx.len() as u64;

    mem::forget(unspent);
    mem::forget(tx);

    ExecuteResponse::success(unspent_ptr, unspent_len, tx_ptr, tx_len)
}

/// Merges many lists of serialized notes into a unique, sanitized set.
///
/// The arguments are expected to be rkyv serialized [MergeNotesArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn merge_notes(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let MergeNotesArgs { notes } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return MergeNotesResponse::fail(),
    };

    let len = 3 * notes.len() / mem::size_of::<ArchivedNote>() / 2;
    let notes = match notes
        .into_iter()
        .map(|n| rkyv::from_bytes::<Vec<Note>>(&n))
        .try_fold::<_, _, Result<_, FromBytesError<Vec<Note>>>>(
            Vec::with_capacity(len),
            |mut set, notes| {
                set.extend(notes?);
                Ok(utils::sanitize_notes(set))
            },
        ) {
        Ok(n) => n,
        Err(_) => return MergeNotesResponse::fail(),
    };

    let notes = match rkyv::to_bytes::<_, MAX_LEN>(&notes) {
        Ok(n) => n.into_vec(),
        Err(_) => return MergeNotesResponse::fail(),
    };

    let notes_ptr = notes.as_ptr() as u64;
    let notes_len = notes.len() as u64;

    MergeNotesResponse::success(notes_ptr, notes_len)
}

/// Filters a list of notes from a list of negative flags. The flags that are
/// `true` will represent a note that must be removed from the set.
///
/// The arguments are expected to be rkyv serialized [FilterNotesArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn filter_notes(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let FilterNotesArgs { notes, flags } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return FilterNotesResponse::fail(),
    };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => n,
        Err(_) => return FilterNotesResponse::fail(),
    };

    let flags: Vec<bool> = match rkyv::from_bytes(&flags) {
        Ok(f) => f,
        Err(_) => return FilterNotesResponse::fail(),
    };

    let notes: Vec<_> = notes
        .into_iter()
        .zip(flags.into_iter())
        .filter_map(|(n, f)| (!f).then_some(n))
        .collect();

    let notes = utils::sanitize_notes(notes);
    let notes = match rkyv::to_bytes::<_, MAX_LEN>(&notes) {
        Ok(n) => n.into_vec(),
        Err(_) => return FilterNotesResponse::fail(),
    };

    let notes_ptr = notes.as_ptr() as u64;
    let notes_len = notes.len() as u64;

    FilterNotesResponse::success(notes_ptr, notes_len)
}

/// Returns a list of [ViewKey] that belongs to this wallet.
///
/// The arguments are expected to be rkyv serialized [ViewKeysArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn view_keys(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let ViewKeysArgs { seed } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return ViewKeysResponse::fail(),
    };

    let vks: Vec<_> = (0..=MAX_KEY)
        .map(|idx| key::derive_vk(&seed, idx as u64))
        .collect();

    let vks = match rkyv::to_bytes::<_, MAX_LEN>(&vks) {
        Ok(k) => k.into_vec(),
        Err(_) => return ViewKeysResponse::fail(),
    };

    let vks_ptr = vks.as_ptr() as u64;
    let vks_len = vks.len() as u64;

    ViewKeysResponse::success(vks_ptr, vks_len)
}

/// Returns a list of [BlsScalar] nullifiers for the given [Vec<Note>] combined
/// with the keys of this wallet.
///
/// The arguments are expected to be rkyv serialized [NullifiersArgs] with a
/// pointer defined via [malloc]. It will consume the `args` allocated region
/// and drop it.
#[no_mangle]
pub fn nullifiers(args: i32, len: i32) -> i32 {
    let args = args as *mut u8;
    let len = len as usize;
    let args = unsafe { Vec::from_raw_parts(args, len, len) };

    let NullifiersArgs { seed, notes } = match rkyv::from_bytes(&args) {
        Ok(a) => a,
        Err(_) => return NullifiersResponse::fail(),
    };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => n,
        Err(_) => return NullifiersResponse::fail(),
    };

    let mut nullifiers = Vec::with_capacity(notes.len());
    let mut keys = unsafe { [mem::zeroed(); MAX_KEY + 1] };
    let mut keys_ssk = unsafe { [mem::zeroed(); MAX_KEY + 1] };
    let mut keys_len = 0;

    'outer: for note in notes {
        // we iterate all the available keys until one can successfully
        // decrypt the note. if any fails, returns false
        for idx in 0..=MAX_KEY {
            if keys_len == idx {
                keys_ssk[idx] = key::derive_ssk(&seed, idx as u64);
                keys[idx] = keys_ssk[idx].view_key();
                keys_len += 1;
            }

            if keys[idx].owns(&note) {
                nullifiers.push(note.gen_nullifier(&keys_ssk[idx]));
                continue 'outer;
            }
        }

        return NullifiersResponse::fail();
    }

    let nullifiers = match rkyv::to_bytes::<_, MAX_LEN>(&nullifiers) {
        Ok(n) => n.into_vec(),
        Err(_) => return NullifiersResponse::fail(),
    };

    let nullifiers_ptr = nullifiers.as_ptr() as u64;
    let nullifiers_len = nullifiers.len() as u64;

    NullifiersResponse::success(nullifiers_ptr, nullifiers_len)
}

impl SeedResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedSeedResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful seed response.
    pub fn success(seed_ptr: u64, seed_len: u64) -> i32 {
        Self {
            success: true,
            seed_ptr,
            seed_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the seed operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            seed_ptr: 0,
            seed_len: 0,
        }
        .as_i32_ptr()
    }
}

impl BalanceResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedBalanceResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful balance operation with the
    /// computed value.
    pub fn success(value: u64, maximum: u64) -> i32 {
        Self {
            success: true,
            value,
            maximum,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the balance operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            value: 0,
            maximum: 0,
        }
        .as_i32_ptr()
    }
}

impl ExecuteResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedExecuteResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful execute operation with the
    /// underlying unspent notes list and the unproven transaction.
    pub fn success(
        unspent_ptr: u64,
        unspent_len: u64,
        tx_ptr: u64,
        tx_len: u64,
    ) -> i32 {
        Self {
            success: true,
            unspent_ptr,
            unspent_len,
            tx_ptr,
            tx_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the execute operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            unspent_ptr: 0,
            unspent_len: 0,
            tx_ptr: 0,
            tx_len: 0,
        }
        .as_i32_ptr()
    }
}

impl MergeNotesResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedMergeNotesResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful merge_notes operation.
    pub fn success(notes_ptr: u64, notes_len: u64) -> i32 {
        Self {
            success: true,
            notes_ptr,
            notes_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the merge_notes operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            notes_ptr: 0,
            notes_len: 0,
        }
        .as_i32_ptr()
    }
}

impl FilterNotesResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedFilterNotesResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful filter_notes operation.
    pub fn success(notes_ptr: u64, notes_len: u64) -> i32 {
        Self {
            success: true,
            notes_ptr,
            notes_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the filter_notes operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            notes_ptr: 0,
            notes_len: 0,
        }
        .as_i32_ptr()
    }
}

impl ViewKeysResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedViewKeysResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful view_keys operation.
    pub fn success(vks_ptr: u64, vks_len: u64) -> i32 {
        Self {
            success: true,
            vks_ptr,
            vks_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the view_keys operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            vks_ptr: 0,
            vks_len: 0,
        }
        .as_i32_ptr()
    }
}

impl NullifiersResponse {
    /// Rkyv serialized length of the response
    pub const LEN: usize = mem::size_of::<ArchivedNullifiersResponse>();

    fn as_i32_ptr(&self) -> i32 {
        let b = match rkyv::to_bytes::<_, MAX_LEN>(self) {
            Ok(b) => b.into_vec(),
            Err(_) => return 0,
        };

        let ptr = b.as_ptr() as i32;
        mem::forget(b);

        ptr
    }

    /// Returns a representation of a successful nullifiers operation.
    pub fn success(nullifiers_ptr: u64, nullifiers_len: u64) -> i32 {
        Self {
            success: true,
            nullifiers_ptr,
            nullifiers_len,
        }
        .as_i32_ptr()
    }

    /// Returns a representation of the failure of the nullifiers operation.
    pub fn fail() -> i32 {
        Self {
            success: false,
            nullifiers_ptr: 0,
            nullifiers_len: 0,
        }
        .as_i32_ptr()
    }
}
