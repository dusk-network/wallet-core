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

use dusk_bls12_381_sign::{PublicKey, SecretKey, Signature as BlsSignature};
use dusk_bytes::Serializable;
use dusk_bytes::Write;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_plonk::proof_system::Proof;
use phoenix_core::{transaction::*, Note, *};

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

    let sender = derive_ssk(&seed, sender_index);
    let refund = match bs58_to_psk(&refund) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let rng = &mut utils::rng(rng_seed);

    let blinder = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &refund, 0, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let unstake_note =
        Note::transparent(rng, &sender.public_spend_key(), value);
    let unstake_blinder: dusk_jubjub::Fr = unstake_note
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

    let sk = derive_sk(&seed, sender_index);
    let pk = PublicKey::from(&sk);

    let signature = unstake_sign(&sk, &pk, counter, unstake_note);

    let unstake = Unstake {
        public_key: pk,
        signature,
        note: unstake_note,
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

/// Creates a signature compatible with what the stake contract expects for a
/// unstake transaction.
///
/// The counter is the number of transactions that have been sent to the
/// transfer contract by a given key, and is reported in `StakeInfo`.
fn unstake_sign(
    sk: &SecretKey,
    pk: &PublicKey,
    counter: u64,
    note: Note,
) -> BlsSignature {
    let mut msg: Vec<u8> = Vec::with_capacity(u64::SIZE + Note::SIZE);

    msg.extend(counter.to_bytes());
    msg.extend(note.to_bytes());

    sk.sign(pk, &msg)
}
