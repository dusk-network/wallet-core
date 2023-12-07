// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    ffi::allocate,
    key::derive_vk,
    tx::{self},
    types, utils,
};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Proof as SchnorrSig;
use hashbrown::{hash_map::Entry, HashMap};
use phoenix_core::{transaction, Crossover, Fee, Note, Transaction};
use rusk_abi::{hash::Hasher, ContractId, CONTRACT_ID_BYTES};

/// Convert a tx::UnprovenTransaction to bytes ready to be sent to the node
#[no_mangle]
pub fn unproven_tx_to_bytes(args: i32, len: i32) -> i64 {
    // re-using this type
    let types::RkyvTreeLeaf { bytes } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let tx: tx::UnprovenTransaction = match rkyv::from_bytes(&bytes).ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let bytes = utx_to_var_bytes(&tx);

    let serialized = match bytes.ok() {
        Some(a) => a.to_vec(),
        None => return utils::fail(),
    };

    utils::into_ptr(types::UnprovenTxToBytesResponse { serialized })
}

/// Make sure the proof is okay and convert the given unproven tx
/// to a Proven Transaction
#[no_mangle]
pub fn prove_tx(args: i32, len: i32) -> i64 {
    let types::ProveTxArgs { unproven_tx, proof } =
        match utils::take_args(args, len) {
            Some(a) => a,
            None => return utils::fail(),
        };

    let utx: tx::UnprovenTransaction = match rkyv::from_bytes(&unproven_tx).ok()
    {
        Some(a) => a,
        None => {
            return utils::fail();
        }
    };

    let proof = match Proof::from_slice(&proof).ok() {
        Some(a) => a,
        None => return utils::fail(),
    };

    let mut call = None;

    if let Some(tx::CallData {
        contract,
        method,
        payload,
    }) = utx.clone().call
    {
        call = Some((contract.to_bytes(), method, payload));
    }

    let anchor = utx.anchor;

    let crossover = utx.crossover.clone().map(|e| e.crossover);
    let inputs = &utx.inputs;
    let outputs = utx.outputs.iter().map(|output| output.note).collect();
    let fee = utx.fee;
    let proof = proof.to_bytes().to_vec();
    let nullifiers = inputs.iter().map(|input| input.nullifier).collect();

    let tx = transaction::Transaction {
        nullifiers,
        anchor,
        outputs,
        proof,
        fee,
        crossover,
        call,
    };

    let bytes = tx.to_var_bytes();

    let tx_hash = rusk_abi::hash::Hasher::digest(tx.to_hash_input_bytes());
    let hash = hex::encode(tx_hash.to_bytes());

    utils::into_ptr(types::ProveTxResponse { bytes, hash })
}

/// Calculate the history given the notes and tx data
#[no_mangle]
pub fn get_history(args: i32, len: i32) -> i64 {
    let types::GetHistoryArgs {
        seed,
        index,
        notes,
        tx_data,
    } = match utils::take_args(args, len) {
        Some(a) => a,
        None => return utils::fail(),
    };

    let mut ret: Vec<types::TransactionHistoryType> = Vec::new();

    let seed = match utils::sanitize_seed(seed) {
        Some(s) => s,
        None => return utils::fail(),
    };

    let mut nullifiers = Vec::new();

    for note_data in notes.iter() {
        let nullifier =
            match rkyv::from_bytes::<BlsScalar>(&note_data.nullifier) {
                Ok(a) => a,
                Err(_) => return utils::fail(),
            };

        let note = match rkyv::from_bytes::<Note>(&note_data.note).ok() {
            Some(a) => a,
            None => return utils::fail(),
        };

        nullifiers
            .push((nullifier, note.value(Some(&derive_vk(&seed, index)))));
    }

    let mut block_txs = HashMap::new();
    let vk = derive_vk(&seed, index);

    for (index, note_data) in notes.iter().enumerate() {
        let mut note = match rkyv::from_bytes::<Note>(&note_data.note).ok() {
            Some(a) => a,
            None => return utils::fail(),
        };

        note.set_pos(u64::MAX);

        let note_hash = note.hash();

        let txs = match block_txs.entry(note_data.block_height) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let transactions: Option<Vec<(Transaction, u64)>> = tx_data
                    [index]
                    .txs
                    .iter()
                    .map(|raw_tx| {
                        let decoded = hex::decode(&raw_tx.raw_tx).ok()?;

                        let tx = Transaction::from_slice(&decoded).ok()?;

                        Some((tx, raw_tx.gas_spent))
                    })
                    .collect();

                let txn = match transactions {
                    Some(a) => a,
                    None => return utils::fail(),
                };

                v.insert(txn)
            }
        };

        let note_amount = match note.value(Some(&vk)).ok() {
            Some(a) => a,
            None => return utils::fail(),
        } as f64;

        let note_creator = txs.iter().find(|(t, _)| {
            t.outputs().iter().any(|&n| n.hash().eq(&note_hash))
        });

        if let Some((t, gas_spent)) = note_creator {
            let inputs_amount: Result<Vec<u64>, _> = t
                .nullifiers()
                .iter()
                .filter_map(|input| {
                    nullifiers
                        .clone()
                        .into_iter()
                        .find_map(|n| n.0.eq(input).then_some(n.1))
                })
                .collect();

            let inputs_amount = match inputs_amount {
                Ok(a) => a.iter().sum::<u64>() as f64,
                Err(_) => return utils::fail(),
            };

            let direction = match inputs_amount > 0f64 {
                true => types::TransactionDirectionType::Out,
                false => types::TransactionDirectionType::In,
            };
            let hash_to_find = Hasher::digest(t.to_hash_input_bytes());
            match ret.iter_mut().find(|th| th.id == hash_to_find.to_string()) {
                Some(tx) => tx.amount += note_amount,
                None => ret.push(types::TransactionHistoryType {
                    direction,
                    block_height: note_data.block_height,
                    amount: note_amount - inputs_amount,
                    fee: gas_spent * t.fee().gas_price,
                    id: hash_to_find.to_string(),
                }),
            }
        } else {
            let outgoing_tx = ret.iter_mut().find(|th| {
                th.direction == types::TransactionDirectionType::Out
                    && th.block_height == note_data.block_height
            });

            if let Some(th) = outgoing_tx {
                th.amount += note_amount
            }
        }
    }

    ret.sort_by(|a, b| a.block_height.cmp(&b.block_height));

    ret = ret
        .into_iter()
        .map(|mut th| {
            let dusk = th.amount / rusk_abi::dusk::dusk(1.0) as f64;

            th.amount = dusk;

            th
        })
        .collect::<Vec<_>>();

    utils::into_ptr(types::GetHistoryResponse { history: ret })
}

/// Serialize a unprovenTx we recieved from the wallet-core
/// this is copied from old wallet-core (0.20.0-piecrust.0.6)
fn utx_to_var_bytes(
    tx: &tx::UnprovenTransaction,
) -> Result<Vec<u8>, BytesError> {
    let serialized_inputs: Vec<Vec<u8>> =
        tx.inputs.iter().map(input_to_var_bytes).collect();
    let num_inputs = tx.inputs.len();
    let total_input_len = serialized_inputs
        .iter()
        .fold(0, |len, input| len + input.len());

    let serialized_outputs: Vec<
        [u8; Note::SIZE + u64::SIZE + JubJubScalar::SIZE],
    > = tx
        .outputs
        .iter()
        .map(
            |tx::Output {
                 note,
                 value,
                 blinder,
             }| {
                let mut buf = [0; Note::SIZE + u64::SIZE + JubJubScalar::SIZE];

                buf[..Note::SIZE].copy_from_slice(&note.to_bytes());
                buf[Note::SIZE..Note::SIZE + u64::SIZE]
                    .copy_from_slice(&value.to_bytes());
                buf[Note::SIZE + u64::SIZE
                    ..Note::SIZE + u64::SIZE + JubJubScalar::SIZE]
                    .copy_from_slice(&blinder.to_bytes());

                buf
            },
        )
        .collect();
    let num_outputs = tx.outputs.len();
    let total_output_len = serialized_outputs
        .iter()
        .fold(0, |len, output| len + output.len());

    let size = u64::SIZE
        + num_inputs * u64::SIZE
        + total_input_len
        + u64::SIZE
        + total_output_len
        + BlsScalar::SIZE
        + Fee::SIZE
        + u64::SIZE
        + tx.crossover
            .clone()
            .map_or(0, |_| Crossover::SIZE + u64::SIZE + JubJubScalar::SIZE)
        + u64::SIZE
        + tx.call
            .as_ref()
            .map(
                |tx::CallData {
                     contract: _,
                     method,
                     payload,
                 }| {
                    CONTRACT_ID_BYTES + u64::SIZE + method.len() + payload.len()
                },
            )
            .unwrap_or(0);

    let vec_allocation = allocate(size as i32) as *mut _;
    let mut buf = unsafe { Vec::from_raw_parts(vec_allocation, size, size) };
    let mut writer = &mut buf[..];

    writer.write(&(num_inputs as u64).to_bytes())?;
    for sinput in serialized_inputs {
        writer.write(&(sinput.len() as u64).to_bytes())?;
        writer.write(&sinput)?;
    }

    writer.write(&(num_outputs as u64).to_bytes())?;
    for soutput in serialized_outputs {
        writer.write(&soutput)?;
    }

    writer.write(&tx.anchor.to_bytes())?;
    writer.write(&tx.fee.to_bytes())?;

    let crossover = &tx.crossover;

    write_crossover_value_blinder(
        &mut writer,
        crossover.clone().map(|crossover| {
            (crossover.crossover, crossover.value, crossover.blinder)
        }),
    )?;
    write_optional_call(
        &mut writer,
        &tx.call
            .clone()
            .map(|call| (call.contract, call.method, call.payload)),
    )?;

    Ok(buf)
}

fn write_crossover_value_blinder<W: Write>(
    writer: &mut W,
    crossover: Option<(Crossover, u64, JubJubScalar)>,
) -> Result<(), BytesError> {
    match crossover {
        Some((crossover, value, blinder)) => {
            writer.write(&1_u64.to_bytes())?;
            writer.write(&crossover.to_bytes())?;
            writer.write(&value.to_bytes())?;
            writer.write(&blinder.to_bytes())?;
        }
        None => {
            writer.write(&0_u64.to_bytes())?;
        }
    }

    Ok(())
}

/// Writes an optional call into the writer, prepending it with a `u64` denoting
/// if it is present or not. This should be called at the end of writing other
/// fields since it doesn't write any information about the length of the call
/// data.
fn write_optional_call<W: Write>(
    writer: &mut W,
    call: &Option<(ContractId, String, Vec<u8>)>,
) -> Result<(), BytesError> {
    match call {
        Some((cid, cname, cdata)) => {
            writer.write(&1_u64.to_bytes())?;

            writer.write(cid.as_bytes())?;

            let cname_len = cname.len() as u64;
            writer.write(&cname_len.to_bytes())?;
            writer.write(cname.as_bytes())?;

            writer.write(cdata)?;
        }
        None => {
            writer.write(&0_u64.to_bytes())?;
        }
    };

    Ok(())
}

fn input_to_var_bytes(input: &tx::Input) -> Vec<u8> {
    let affine_pkr = JubJubAffine::from(&input.pk_r_prime);

    let opening_bytes = rkyv::to_bytes::<_, 256>(&input.opening)
        .expect("Rkyv serialization should always succeed for an opening")
        .to_vec();

    let size = BlsScalar::SIZE
        + Note::SIZE
        + JubJubAffine::SIZE
        + SchnorrSig::SIZE
        + u64::SIZE
        + JubJubScalar::SIZE
        + opening_bytes.len();

    let mut bytes: Vec<_> = Vec::with_capacity(size);

    bytes.extend_from_slice(&input.nullifier.to_bytes());
    bytes.extend_from_slice(&input.note.to_bytes());
    bytes.extend_from_slice(&input.value.to_bytes());
    bytes.extend_from_slice(&input.blinder.to_bytes());
    bytes.extend_from_slice(&affine_pkr.to_bytes());
    bytes.extend_from_slice(&input.sig.to_bytes());
    bytes.extend(opening_bytes);

    bytes
}
