// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use phoenix_core::Note;

use alloc::vec::Vec;

use crate::{
    key::{self},
    types::{self},
    utils::{self},
    MAX_KEY, MAX_LEN,
};

/// Returns true or false if the note is owned by the index
/// if its true then nullifier of that note if sent with it
#[no_mangle]
pub fn check_note_ownership(args: i32, len: i32) -> i64 {
    // we just use BalanceArgs again as we don't want to add more cluter types
    // when the data you want is the same
    let types::CheckNoteOwnershipArgs { note, seed } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let note: Note = match rkyv::from_bytes(&note) {
        Ok(n) => n,
        Err(_) => return utils::fail(),
    };

    let mut is_owned: bool = false;
    let mut nullifier_found = BlsScalar::default();
    let mut psk_found: Option<dusk_pki::PublicSpendKey> = None;

    for idx in 0..=MAX_KEY {
        let idx = idx as u64;
        let view_key = key::derive_vk(&seed, idx);

        if view_key.owns(&note) {
            let ssk = key::derive_ssk(&seed, idx);
            let nullifier = note.gen_nullifier(&ssk);

            nullifier_found = nullifier;
            is_owned = true;
            psk_found = Some(ssk.public_spend_key());

            break;
        }
    }

    let psk_found =
        psk_found.map(|psk| bs58::encode(psk.to_bytes()).into_string());

    let nullifier_found =
        match rkyv::to_bytes::<BlsScalar, MAX_LEN>(&nullifier_found).ok() {
            Some(n) => n.to_vec(),
            None => return utils::fail(),
        };

    utils::into_ptr(types::CheckNoteOwnershipResponse {
        is_owned,
        nullifier: nullifier_found,
        public_spend_key: psk_found,
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
        psks,
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

    for (index, ((note, nullifier), psk)) in notes
        .into_iter()
        .zip(nullifiers_of_notes)
        .zip(psks)
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
                psk,
                block_height,
                note,
                nullifier,
            });
        } else {
            unspent_notes.push(types::NoteInfoType {
                pos: *parsed_note.pos(),
                note,
                block_height,
                psk,
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
