// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    tx,
    types::{self},
    utils, MAX_LEN,
};

use dusk_jubjub::BlsScalar;
use phoenix_core::{transaction::TreeLeaf, Note};

use alloc::vec::Vec;

/// Serialize a u64 integer to bytes using rkyv so we can send it to the network
#[no_mangle]
pub fn rkyv_u64(args: i32, len: i32) -> i64 {
    let types::RkyvU64 { value } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    utils::rkyv_into_ptr(value)
}

/// Get block_heignt a rkyv serialized note from a tree leaf
#[no_mangle]
pub fn rkyv_tree_leaf(args: i32, len: i32) -> i64 {
    let types::RkyvTreeLeaf { bytes } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let TreeLeaf { block_height, note } = match rkyv::from_bytes(&bytes) {
        Ok(n) => n,
        Err(_) => return utils::fail(),
    };

    let last_pos = *note.pos();

    let note = match rkyv::to_bytes::<_, MAX_LEN>(&note).ok() {
        Some(t) => t.into_vec(),
        None => return utils::fail(),
    };

    utils::into_ptr(types::RkyvTreeLeafResponse {
        block_height,
        note,
        last_pos,
    })
}

/// Convert a Vec<Note> (where note is a U8initArray into a rkyv serialized
/// Vec<Note>
#[no_mangle]
pub fn rkyv_notes_array(args: i32, len: i32) -> i64 {
    let types::RkyvNotesArray { notes } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let mut vec_notes = Vec::with_capacity(notes.len());

    for note in notes {
        let parsed_note: Note = match rkyv::from_bytes(&note).ok() {
            Some(t) => t,
            None => return utils::fail(),
        };

        vec_notes.push(parsed_note);
    }

    utils::rkyv_into_ptr(vec_notes)
}

/// Convert a Vec<Vec<u8>> of rkyv serialized Vec<BlsScalar> to Vec<u8>
#[no_mangle]
pub fn rkyv_bls_scalar_array(args: i32, len: i32) -> i64 {
    // we reuse this argument
    let types::RkyvBlsScalarArrayArgs { bytes } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let mut bls_scalars = Vec::new();

    for scalar in bytes {
        match rkyv::from_bytes::<BlsScalar>(&scalar).ok() {
            Some(v) => bls_scalars.push(v),
            None => return utils::fail(),
        }
    }

    let bls_scalars =
        match rkyv::to_bytes::<Vec<BlsScalar>, MAX_LEN>(&bls_scalars).ok() {
            Some(v) => v.to_vec(),
            None => return utils::fail(),
        };

    utils::rkyv_into_ptr(bls_scalars)
}

/// Opposite of the rkyv_bls_scalar_array function
/// Converts a rkyv serialized Vec<u8> of Vec<BlsScalar> to Array<Uint8Array>
#[no_mangle]
pub fn bls_scalar_array_rkyv(args: i32, len: i32) -> i64 {
    // reusing this type
    let types::RkyvTreeLeaf { bytes } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let scalars: Vec<BlsScalar> = match rkyv::from_bytes(&bytes).ok() {
        Some(n) => n,
        None => return utils::fail(),
    };

    let mut scalar_array = Vec::new();

    for scalar in scalars {
        let serialized =
            match rkyv::to_bytes::<BlsScalar, MAX_LEN>(&scalar).ok() {
                Some(n) => n.to_vec(),
                None => return utils::fail(),
            };

        scalar_array.push(serialized);
    }

    utils::into_ptr(types::RkyvBlsScalarArrayArgs {
        bytes: scalar_array,
    })
}

/// convert a [rkyv_serailized_tx::Opening] (we get it from the node) into a
/// Vec<Tx::Opening> rkyv serialized Vec<u8> to send to execute fn
#[no_mangle]
pub fn rkyv_openings_array(args: i32, len: i32) -> i64 {
    let types::RkyvOpeningsArray { openings } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let mut openings_vec = Vec::new();

    for opening in openings {
        let opening: tx::Opening = match rkyv::from_bytes(&opening).ok() {
            Some(x) => x,
            None => return utils::fail(),
        };

        openings_vec.push(opening);
    }

    utils::rkyv_into_ptr(openings_vec)
}
