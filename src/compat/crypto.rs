// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_pki::{PublicSpendKey, ViewKey};
use phoenix_core::Note;
use serde::Serialize;

use crate::{key, types, utils, MAX_KEY, MAX_LEN};

/// Returns a Vec<PublicSpendKeyAndNote> indicating which public spend key
/// owns which note
#[no_mangle]
pub fn check_note_validity(args: i32, len: i32) -> i64 {
    // we just use BalanceArgs again as we don't want to add more cluter types
    // when the data you want is the same
    let types::BalanceArgs { notes, seed } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail_with(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let notes: Vec<Note> = match rkyv::from_bytes(&notes) {
        Ok(n) => utils::sanitize_notes(n),
        Err(_) => return utils::fail(),
    };

    let mut response = Vec::new();

    for idx in 0..=MAX_KEY {
        let view_key = key::derive_vk(&seed, idx as u64);
        let mut temp = Vec::new();

        for note in &notes {
            if view_key.owns(note) {
                temp.push(*note);
            }
        }

        let notes: Vec<u8> = match rkyv::to_bytes::<Vec<Note>, MAX_LEN>(&temp) {
            Ok(n) => n.to_vec(),
            Err(_) => return utils::fail(),
        };

        response.push(types::PublicSpendKeysAndNotesType {
            public_spend_key: bs58::encode(
                view_key.public_spend_key().to_bytes(),
            )
            .into_string(),
            notes,
        });

        temp.clear();
    }

    utils::into_ptr(types::CheckNoteValidityResponse {
        public_spend_key_and_note: response,
    })
}
