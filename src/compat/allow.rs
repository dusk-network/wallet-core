// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{key::*, types, utils, MAX_LEN};

use alloc::string::String;
use alloc::vec::Vec;

use dusk_bls12_381_sign::{PublicKey, SecretKey, Signature as BlsSignature};
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubScalar;
use phoenix_core::{transaction::*, Note, *};

/// Get unstake call data
#[no_mangle]
pub fn get_allow_call_data(args: i32, len: i32) -> i64 {
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

    let refund: dusk_pki::PublicSpendKey = match utils::bs58_to_psk(&refund) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let sk = derive_sk(&seed, owner_index);
    let staker = PublicKey::from(&sk);

    let owner_sk = derive_sk(&seed, sender_index);
    let owner_pk = PublicKey::from(&owner_sk);

    let rng = &mut utils::rng(rng_seed);

    let signature = allow_sign(&owner_sk, &owner_pk, counter, &staker);

    let blinder = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &refund, 0, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Obfuscated notes should always yield crossovers");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let allow = Allow {
        public_key: staker,
        owner: owner_pk,
        signature,
    };

    let contract = bs58::encode(rusk_abi::STAKE_CONTRACT).into_string();
    let method = String::from("allow");
    let payload = match rkyv::to_bytes::<_, MAX_LEN>(&allow).ok() {
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

/// Creates a signature compatible with what the stake contract expects for a
/// ADD_ALLOWLIST transaction.
///
/// The counter is the number of transactions that have been sent to the
/// transfer contract by a given key, and is reported in `StakeInfo`.
fn allow_sign(
    sk: &SecretKey,
    pk: &PublicKey,
    counter: u64,
    staker: &PublicKey,
) -> BlsSignature {
    let mut msg = Vec::with_capacity(u64::SIZE + PublicKey::SIZE);

    msg.extend(counter.to_bytes());
    msg.extend(staker.to_bytes());

    sk.sign(pk, &msg)
}
