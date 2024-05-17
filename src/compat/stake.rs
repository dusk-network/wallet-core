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
use dusk_bls12_381::BlsScalar;
use dusk_bytes::{Serializable, Write};
use dusk_jubjub::JubJubScalar;
use dusk_plonk::prelude::Proof;
use ff::Field;
use jubjub_schnorr::Signature;
use phoenix_core::{
    transaction::{stct_signature_message, StakeData},
    Crossover, Fee, Note, Ownable,
};
use stake_contract_types::{stake_signature_message, Stake};

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

/// Get the bytes to send to the node to prove stct proof
/// and then we can get the proof verified from the node
#[no_mangle]
pub fn get_stct_proof(args: i32, len: i32) -> i64 {
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
    let note = Note::obfuscated(rng, &refund, value, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let contract_id = rusk_abi::STAKE_CONTRACT;
    let address = rusk_abi::contract_to_scalar(&contract_id);

    let contract_id = rusk_abi::contract_to_scalar(&contract_id);

    let stct_message = stct_signature_message(&crossover, value, contract_id);
    let stct_message = dusk_poseidon::sponge::hash(&stct_message);

    let note_sk = sender.sk_r(fee.stealth_address());
    let stct_signature = note_sk.sign(rng, stct_message);

    let vec_allocation = allocate(STCT_INPUT_SIZE as i32) as *mut _;
    let mut buf: Vec<u8> = unsafe {
        Vec::from_raw_parts(vec_allocation, STCT_INPUT_SIZE, STCT_INPUT_SIZE)
    };

    let mut writer = &mut buf[..];

    let mut bytes = || {
        writer.write(&fee.to_bytes()).ok()?;
        writer.write(&crossover.to_bytes()).ok()?;
        writer.write(&value.to_bytes()).ok()?;
        writer.write(&blinder.to_bytes()).ok()?;
        writer.write(&address.to_bytes()).ok()?;
        writer.write(&stct_signature.to_bytes()).ok()?;

        Some(())
    };

    let bytes = match bytes() {
        Some(_) => buf,
        None => return utils::fail(),
    }
    .to_vec();

    let stct_sig = match rkyv::to_bytes::<Signature, MAX_LEN>(&stct_signature) {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail(),
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

    utils::into_ptr(types::GetStctProofResponse {
        bytes,
        signature: stct_sig,
        crossover,
        blinder,
        fee,
    })
}

/// Get the (contract_id, method, payload) for stake
#[no_mangle]
pub fn get_stake_call_data(args: i32, len: i32) -> i64 {
    let types::GetStakeCallDataArgs {
        staker_index,
        seed,
        proof,
        value,
        counter,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let proof: [u8; Proof::SIZE] = match proof.try_into().ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let proof = match Proof::from_bytes(&proof).ok() {
        Some(a) => a.to_bytes().to_vec(),
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let stake_sk = derive_stake_sk(&seed, staker_index);
    let stake_pk = StakePublicKey::from(&stake_sk);

    let msg = stake_signature_message(counter, value);
    let stake_sig = stake_sk.sign(&stake_pk, &msg);

    let stake = Stake {
        public_key: stake_pk,
        signature: stake_sig,
        value,
        proof,
    };

    let contract = bs58::encode(rusk_abi::STAKE_CONTRACT).into_string();
    let method = String::from("stake");
    let payload = match rkyv::to_bytes::<_, MAX_LEN>(&stake).ok() {
        Some(a) => a.to_vec(),
        None => return utils::fail(),
    };

    utils::into_ptr(types::GetStakeCallDataResponse {
        contract,
        method,
        payload,
    })
}

#[no_mangle]
fn get_stake_info(args: i32, len: i32) -> i64 {
    let types::GetStakeInfoArgs { stake_info } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let mut has_staked = false;

    match rkyv::from_bytes::<Option<StakeData>>(&stake_info).ok() {
        Some(Some(a)) => {
            let (amount, eligiblity) = match a.amount {
                Some((x, y)) => {
                    has_staked = true;

                    (Some(x), Some(y))
                }
                None => (None, None),
            };
            let reward = Some(a.reward);
            let counter = Some(a.counter);

            utils::into_ptr(types::GetStakeInfoRespose {
                has_staked,
                amount,
                eligiblity,
                reward,
                counter,
                has_key: true,
            })
        }
        Some(None) | None => utils::into_ptr(types::GetStakeInfoRespose {
            has_staked,
            amount: None,
            reward: None,
            eligiblity: None,
            counter: None,
            has_key: false,
        }),
    }
}
