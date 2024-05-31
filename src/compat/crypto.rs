// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::mem::size_of;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use phoenix_core::{
    transaction::{ArchivedTreeLeaf, TreeLeaf},
    Note, PublicKey,
};

use alloc::{string::ToString, vec::Vec};

use crate::alloc::borrow::ToOwned;
use crate::{
    key::{self},
    types::{self},
    utils::{self},
    MAX_KEY, MAX_LEN,
};

const TREE_LEAF_SIZE: usize = size_of::<ArchivedTreeLeaf>();

/// Returns true or false if the note is owned by the index
/// if its true then nullifier of that note if sent with it
#[no_mangle]
pub fn check_note_ownership(args: i32, len: i32) -> i64 {
    // SAFETY: We assume the caller has passed a valid pointer and len as the
    // function arguments else we might get undefined behavior
    let args = unsafe { core::slice::from_raw_parts(args as _, len as _) };

    let seed = &args[..64];
    let leaves: &[u8] = &args[64..];

    let seed = match seed.try_into() {
        Ok(s) => s,
        Err(_) => return utils::fail(),
    };

    let mut leaf_chunk = leaves.chunks_exact(TREE_LEAF_SIZE);
    let mut last_pos = 0;

    let mut notes = Vec::new();
    let mut nullifiers = Vec::new();
    let mut block_heights = Vec::new();
    let mut public_spend_keys = Vec::new();
    let mut view_keys = Vec::with_capacity(MAX_KEY);
    let mut secret_keys = Vec::with_capacity(MAX_KEY);

    for idx in 0..MAX_KEY {
        let idx = idx as u64;
        let view_key = key::derive_vk(&seed, idx);
        let sk = key::derive_sk(&seed, idx as _);
        view_keys.push(view_key);
        secret_keys.push(sk);
    }

    for leaf_bytes in leaf_chunk.by_ref() {
        let TreeLeaf { block_height, note } = match rkyv::from_bytes(leaf_bytes)
        {
            Ok(a) => a,
            Err(_) => {
                return utils::fail();
            }
        };

        last_pos = core::cmp::max(last_pos, *note.pos());

        for idx in 0..MAX_KEY {
            if view_keys[idx].owns(&note) {
                let sk = secret_keys[idx];
                let nullifier = note.gen_nullifier(&sk);

                let nullifier_found =
                    match rkyv::to_bytes::<BlsScalar, MAX_LEN>(&nullifier).ok()
                    {
                        Some(n) => n.to_vec(),
                        None => return utils::fail(),
                    };

                let psk_found =
                    bs58::encode(PublicKey::from(sk).to_bytes()).into_string();

                let raw_note: Vec<u8> =
                    match rkyv::to_bytes::<Note, MAX_LEN>(&note) {
                        Ok(n) => n.to_vec(),
                        Err(_) => return utils::fail(),
                    };

                notes.push(raw_note.to_owned());
                block_heights.push(block_height);
                public_spend_keys.push(psk_found);
                nullifiers.push(nullifier_found);
            }
        }
    }

    let block_heights = block_heights
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",");

    utils::into_ptr(types::CheckNoteOwnershipResponse {
        notes,
        block_heights,
        public_spend_keys,
        nullifiers,
        last_pos,
    })
}

/// Given array of notes, nullifiers of those notes and some existing
/// nullifiers, sort the notes into unspent and spent arrays
#[no_mangle]
pub fn unspent_spent_notes(args: i32, len: i32) -> i64 {
    let types::UnspentSpentNotesArgs {
        notes,
        nullifiers_of_notes,
        block_heights,
        existing_nullifiers,
        pks,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let existing_nullifiers =
        match rkyv::from_bytes::<Vec<BlsScalar>>(&existing_nullifiers).ok() {
            Some(a) => a,
            None => return utils::fail(),
        };

    let mut spent_notes = Vec::new();
    let mut unspent_notes = Vec::new();

    for (index, ((note, nullifier), pk)) in notes
        .into_iter()
        .zip(nullifiers_of_notes)
        .zip(pks)
        .enumerate()
    {
        let parsed_note: Note = match rkyv::from_bytes::<Note>(&note).ok() {
            Some(a) => a,
            None => return utils::fail(),
        };

        let parsed_nullifier =
            match rkyv::from_bytes::<BlsScalar>(&nullifier).ok() {
                Some(a) => a,
                None => return utils::fail(),
            };

        let block_height = match block_heights.get(index) {
            Some(a) => *a as u64,
            None => return utils::fail(),
        };

        if existing_nullifiers.contains(&parsed_nullifier) {
            spent_notes.push(types::NoteInfoType {
                pos: *parsed_note.pos(),
                pk,
                block_height,
                note,
                nullifier,
            });
        } else {
            unspent_notes.push(types::NoteInfoType {
                pos: *parsed_note.pos(),
                note,
                block_height,
                pk,
                nullifier,
            });
        }
    }

    utils::into_ptr(types::UnpsentSpentNotesResponse {
        spent_notes,
        unspent_notes,
    })
}

/// Convert dusk to lux to send to methods
#[no_mangle]
fn dusk_to_lux(args: i32, len: i32) -> i64 {
    let types::DuskToLuxArgs { dusk } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    utils::into_ptr(types::DuskToLuxResponse {
        lux: rusk_abi::dusk::from_dusk(dusk),
    })
}

/// Convert lux to dusk
#[no_mangle]
fn lux_to_dusk(args: i32, len: i32) -> i64 {
    // reusing the type from above, two less type definitions
    let types::DuskToLuxResponse { lux } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    utils::into_ptr(types::DuskToLuxArgs {
        dusk: rusk_abi::dusk::dusk(lux),
    })
}
