// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_jubjub::{BlsScalar, JubJubScalar, JubJubAffine};
use phoenix_core::{Note, Crossover, Fee, transaction::*};
use dusk_schnorr::{ Signature};
use dusk_pki::{Ownable, SecretKey as SchnorrKey};
use dusk_bytes::Write;

use alloc::vec::Vec;

use crate::{key::{self, derive_ssk}, types, utils, MAX_KEY, MAX_LEN};

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

/// Returns true or false if the note is owned by the index
/// if its true then nullifier of that note if sent with it
#[no_mangle]
pub fn check_note_ownership(args: i32, len: i32) -> i64 {
    // we just use BalanceArgs again as we don't want to add more cluter types
    // when the data you want is the same
    let types::CheckNoteOwnershipArgs { note, seed } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail_with(),
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
    let mut psk_found = None;

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

    for ((note, nullifier), psk) in
        notes.into_iter().zip(nullifiers_of_notes).zip(psks)
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

        if existing_nullifiers.contains(&parsed_nullifier) {
            spent_notes.push(types::NoteInfoType {
                pos: *parsed_note.pos(),
                psk,
                note,
            });
        } else {
            unspent_notes.push(types::NoteInfoType {
                pos: *parsed_note.pos(),
                note,
                psk,
            });
        }
    }

    utils::into_ptr(types::UnpsentSpentNotesResponse {
        spent_notes,
        unspent_notes,
    })
}

#[no_mangle]
pub fn get_stct_proof(args: i32, len: i32) -> i64 {
    let type::CrossoverFeeArgs { rng_seed, seed, refund, value, sender_index, gas_limit, gas_price } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail()
    };

    let rng_seed = match utils::sanitize_seed(rng_seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };


    let sender = derive_ssk(&seed, sender_index);

    let rng = &mut utils::rng(&rng_seed);

    let blinder = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, refund, value, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        let contract_id = rusk_abi::STAKE_CONTRACT;
        let address = rusk_abi::contract_to_scalar(&contract_id);

        let contract_id = rusk_abi::contract_to_scalar(&contract_id);

        let stct_message =
            stct_signature_message(&crossover, value, contract_id);
        let stct_message = dusk_poseidon::sponge::hash(&stct_message);

        let sk_r = *sender.sk_r(fee.stealth_address()).as_ref();
        let secret = SchnorrKey::from(sk_r);

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let stct_signature = Signature::new(&secret, rng, stct_message);

        fn get_buf(fee: Fee, crossover: Crossover, value: u64, blinder: JubJubScalar, address: BlsScalar, stct_signature: Signature) -> Option<[u8; STCT_INPUT_SIZE]> {
        let mut buf = [0; STCT_INPUT_SIZE];
        let mut writer = &mut buf[..];

        writer.write(&fee.to_bytes()).ok()?;
        writer.write(&crossover.to_bytes()).ok()?;
        writer.write(&value.to_bytes()).ok()?;
        writer.write(&blinder.to_bytes()).ok()?;
        writer.write(&address.to_bytes()).ok()?;
        writer.write(&stct_signature.to_bytes()).ok()?;

        Some(buf)
        }

        let buf = match get_buf(fee, crossover, value, blinder, address, stct_signature)  {
            Some(a) => a,
            None => return utils::fail(),
        };

        utils::into_ptr(types::CrossoverFeeArgsResponse {
            buffer: buf
        })

}