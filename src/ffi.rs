// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! FFI bindings exposed to WASM module.

use alloc::{vec, vec::Vec};
use core::mem;

use dusk_pki::{PublicSpendKey, SecretSpendKey};
use phoenix_core::note::{Note, NoteType};

use crate::{
    key, tx, utils, BalanceArgs, BalanceResponse, ExecuteArgs, ExecuteResponse,
    MAX_KEY, MAX_LEN,
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
        Ok(n) => n,
        Err(_) => return BalanceResponse::fail(),
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
        let openings: Vec<tx::Opening> = rkyv::from_bytes(&openings).ok()?;
        let refund: PublicSpendKey = rkyv::from_bytes(&refund).ok()?;
        let output: Option<tx::OutputValue> = rkyv::from_bytes(&output).ok()?;
        let call: Option<tx::CallData> = rkyv::from_bytes(&call).ok()?;

        let value = output.as_ref().map(|o| o.value).unwrap_or(0);
        let total_output =
            gas_limit.saturating_mul(gas_price).saturating_add(value);

        let mut keys = unsafe { [mem::zeroed(); MAX_KEY] };
        let mut keys_ssk =
            unsafe { [mem::zeroed::<SecretSpendKey>(); MAX_KEY] };
        let mut keys_len = 0;
        let mut openings = openings.into_iter();
        let mut full_inputs = Vec::with_capacity(inputs.len());

        'outer: for input in inputs {
            // we iterate all the available keys until one can successfully
            // decrypt the note. if any fails, returns false
            for idx in 0..MAX_KEY {
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

impl BalanceResponse {
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
