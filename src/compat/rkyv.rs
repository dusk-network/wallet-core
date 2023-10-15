// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{types, utils, MAX_LEN};

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

    let note = match rkyv::to_bytes::<_, MAX_LEN>(&note).ok() {
        Some(t) => t.into_vec(),
        None => return utils::fail(),
    };

    utils::into_ptr(types::RkyvTreeLeafResponse { block_height, note })
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
