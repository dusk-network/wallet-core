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

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_bytes::Write;
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::{Ownable, SecretKey as SchnorrKey};
use dusk_plonk::prelude::*;
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use phoenix_core::{transaction::*, Note, *};

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

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

    let rng_seed = match utils::sanitize_seed(rng_seed) {
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

    let rng = &mut utils::rng(&rng_seed);

    let blinder = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &refund, value, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    let contract_id = rusk_abi::STAKE_CONTRACT;
    let address = rusk_abi::contract_to_scalar(&contract_id);

    let contract_id = rusk_abi::contract_to_scalar(&contract_id);

    let stct_message = stct_signature_message(&crossover, value, contract_id);
    let stct_message = dusk_poseidon::sponge::hash(&stct_message);

    let sk_r = *sender.sk_r(fee.stealth_address()).as_ref();
    let secret = SchnorrKey::from(sk_r);

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let stct_signature = Signature::new(&secret, rng, stct_message);

    let vec_allocation = allocate(STCT_INPUT_SIZE as i32) as *mut _;
    let mut buf = unsafe {
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
        None => return utils::fail_with(),
    }
    .to_vec();

    let signature = match rkyv::to_bytes::<Signature, MAX_LEN>(&stct_signature)
    {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail_with(),
    };

    let crossover = match rkyv::to_bytes::<Crossover, MAX_LEN>(&crossover) {
        Ok(a) => a.to_vec(),
        Err(_) => return utils::fail_with(),
    };

    utils::into_ptr(types::GetStctProofResponse {
        bytes,
        signature,
        crossover,
    })
}

/// Get the (contract_id, method, payload) for stake
#[no_mangle]
pub fn get_stake_call_data(args: i32, len: i32) -> i64 {
    let types::GetStakeCallDataArgs {
        staker_index,
        seed,
        spend_proof,
        value,
        signature,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let spend_proof: [u8; Proof::SIZE] = match spend_proof.try_into().ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let proof = match Proof::from_bytes(&spend_proof).ok() {
        Some(a) => a.to_bytes().to_vec(),
        None => return utils::fail(),
    };

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let signature = match rkyv::from_bytes(&signature).ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let sk = derive_sk(&seed, staker_index);
    let pk = PublicKey::from(&sk);

    let stake = Stake {
        public_key: pk,
        signature,
        value,
        proof: proof,
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
