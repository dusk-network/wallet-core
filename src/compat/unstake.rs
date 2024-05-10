// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    ffi::allocate,
    key::*,
    types::{self},
    utils::{self, *},
    MAX_LEN,
};

use alloc::string::String;
use alloc::vec::Vec;

use bls12_381_bls::PublicKey as StakePublicKey;
use dusk_bytes::{Serializable, Write};
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_plonk::prelude::Proof;
use ff::Field;
use phoenix_core::{Crossover, Fee, Note, PublicKey};
use stake_contract_types::{unstake_signature_message, Unstake};

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

/// Get the bytes to send to the node to prove wfct proof
#[no_mangle]
pub fn get_wfct_proof(args: i32, len: i32) -> i64 {
    // re-using the type
    let types::GetStctProofArgs {
        rng_seed,
        seed,
        refund,
        value,
        sender_index,
        gas_limit,
        gas_price,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let rng_seed = match utils::sanitize_rng_seed(rng_seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let sender = derive_sk(&seed, sender_index);
    let refund = match bs58_to_pk(&refund) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let rng = &mut utils::rng(rng_seed);

    let blinder = JubJubScalar::random(&mut *rng);
    let note = Note::obfuscated(rng, &refund, 0, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let unstake_note = Note::transparent(rng, &PublicKey::from(&sender), value);
    let unstake_blinder: JubJubScalar = unstake_note
        .blinding_factor(None)
        .expect("Note is transparent so blinding factor is unencrypted");

    let commitment: JubJubAffine = unstake_note.value_commitment().into();

    let vec_allocation = allocate(WFCT_INPUT_SIZE as i32) as *mut _;
    let mut buf: Vec<u8> = unsafe {
        Vec::from_raw_parts(vec_allocation, WFCT_INPUT_SIZE, WFCT_INPUT_SIZE)
    };

    let mut writer = &mut buf[..];

    let mut bytes = || {
        writer.write(&commitment.to_bytes()).ok()?;
        writer.write(&value.to_bytes()).ok()?;
        writer.write(&unstake_blinder.to_bytes()).ok()?;

        Some(())
    };

    let bytes = match bytes() {
        Some(_) => buf,
        None => return utils::fail(),
    }
    .to_vec();

    let unstake_note = match rkyv::to_bytes::<Note, MAX_LEN>(&unstake_note).ok()
    {
        Some(a) => a.to_vec(),
        None => return utils::fail(),
    };

    let crossover = match rkyv::to_bytes::<Crossover, MAX_LEN>(&crossover) {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail(),
    };

    let blinder = match rkyv::to_bytes::<JubJubScalar, MAX_LEN>(&blinder) {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail(),
    };

    let fee = match rkyv::to_bytes::<Fee, MAX_LEN>(&fee) {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail(),
    };

    utils::into_ptr(types::GetWfctProofResponse {
        bytes,
        blinder,
        crossover,
        fee,
        unstake_note,
    })
}

/// Get unstake call data
#[no_mangle]
pub fn get_unstake_call_data(args: i32, len: i32) -> i64 {
    let types::GetUnstakeCallDataArgs {
        seed,
        sender_index,
        unstake_note,
        counter,
        unstake_proof,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let unstake_proof: [u8; Proof::SIZE] = match unstake_proof.try_into().ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let proof = match Proof::from_bytes(&unstake_proof).ok() {
        Some(a) => a.to_bytes().to_vec(),
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let unstake_note = match rkyv::from_bytes::<Note>(&unstake_note).ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let stake_sk = derive_stake_sk(&seed, sender_index);
    let stake_pk = StakePublicKey::from(&stake_sk);

    let unstake_note = unstake_note.to_bytes();
    let signature_message = unstake_signature_message(counter, unstake_note);

    let signature = stake_sk.sign(&stake_pk, &signature_message);

    let unstake = Unstake {
        public_key: stake_pk,
        signature,
        note: unstake_note.to_vec(),
        proof,
    };

    let contract = bs58::encode(rusk_abi::STAKE_CONTRACT).into_string();
    let method = String::from("unstake");
    let payload = match rkyv::to_bytes::<_, MAX_LEN>(&unstake).ok() {
        Some(a) => a.to_vec(),
        None => return utils::fail(),
    };

    // reusing this type
    utils::into_ptr(types::GetStakeCallDataResponse {
        contract,
        method,
        payload,
    })
}
