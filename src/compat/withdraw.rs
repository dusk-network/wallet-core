// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{key::*, types, utils, MAX_LEN};

use alloc::string::String;

use bls12_381_bls::PublicKey as StakePublicKey;
use dusk_jubjub::{BlsScalar, JubJubScalar};
use ff::Field;
use phoenix_core::{Crossover, Fee, Note, PublicKey, StealthAddress};
use stake_contract_types::{withdraw_signature_message, Withdraw};

/// Get unstake call data
#[no_mangle]
pub fn get_withdraw_call_data(args: i32, len: i32) -> i64 {
    // reusing the type
    let types::GetAllowCallDataArgs {
        seed,
        rng_seed,
        sender_index,
        refund,
        owner_index,
        counter,
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

    let refund = match utils::bs58_to_pk(&refund) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let sender_sk = derive_sk(&seed, sender_index);
    let sender_pk = PublicKey::from(&sender_sk);

    let stake_sk = derive_stake_sk(&seed, owner_index);
    let stake_pk = StakePublicKey::from(&stake_sk);

    let rng = &mut utils::rng(rng_seed);

    let withdraw_r = JubJubScalar::random(&mut *rng);
    let address: StealthAddress = sender_pk.gen_stealth_address(&withdraw_r);
    let nonce = BlsScalar::random(&mut *rng);

    let msg = withdraw_signature_message(counter, address, nonce);
    let signature = stake_sk.sign(&stake_pk, &msg);

    // Since we're not transferring value *to* the contract the crossover
    // shouldn't contain a value. As such the note used to created it should
    // be valueless as well.
    let blinder = JubJubScalar::random(&mut *rng);
    let note = Note::obfuscated(rng, &refund, 0, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let withdraw = Withdraw {
        public_key: stake_pk,
        signature,
        address,
        nonce,
    };

    let contract = bs58::encode(rusk_abi::STAKE_CONTRACT).into_string();
    let method = String::from("withdraw");
    let payload = match rkyv::to_bytes::<_, MAX_LEN>(&withdraw).ok() {
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

    // reusing this type
    utils::into_ptr(types::GetAllowCallDataResponse {
        contract,
        method,
        payload,
        blinder,
        crossover,
        fee,
    })
}
