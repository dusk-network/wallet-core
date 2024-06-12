// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    key::{self},
    tx,
    types::{self},
    utils, MAX_LEN,
};

use bls12_381_bls::PublicKey as StakePublicKey;
use dusk_bls12_381::BlsScalar;
use phoenix_core::Note;

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

    let mut openings_vec: Vec<(tx::Opening, u64)> = Vec::new();

    for opening in openings {
        let opening_parsed: tx::Opening =
            match rkyv::from_bytes(&opening.opening).ok() {
                Some(x) => x,
                None => return utils::fail(),
            };

        openings_vec.push((opening_parsed, opening.pos));
    }

    utils::rkyv_into_ptr::<Vec<(tx::Opening, u64)>>(openings_vec)
}

/// Rkyv serialize stake public key to send to the node to obtain
/// stake-info
#[no_mangle]
fn get_stake_pk_rkyv_serialized(args: i32, len: i32) -> i64 {
    let types::GetStakePKrkyvSerializedArgs { seed, index } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let stake_sk = key::derive_stake_sk(&seed, index);
    let stake_pk = StakePublicKey::from(&stake_sk);

    utils::rkyv_into_ptr::<StakePublicKey>(stake_pk)
}
