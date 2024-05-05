// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Unspent transaction definition.

use alloc::string::String;
use alloc::vec::Vec;
use core::mem;

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubExtended, JubJubScalar, GENERATOR_NUMS_EXTENDED};
use ff::Field;
use jubjub_schnorr::SignatureDouble;
use phoenix_core::{
    Crossover as PhoenixCrossover, Fee, Note, NoteType, Ownable, PublicKey,
    SecretKey, Transaction, ViewKey,
};
use rand_core::{CryptoRng, RngCore};
use rkyv::{Archive, Deserialize, Serialize};
use rusk_abi::hash::Hasher;
use rusk_abi::{ContractId, POSEIDON_TREE_DEPTH};

use crate::{types, types::CrossoverType, utils};

/// Chosen arity for the Notes tree implementation.
pub const POSEIDON_TREE_ARITY: usize = 4;

/// The Merkle Opening used in Rusk.
pub type Opening =
    poseidon_merkle::Opening<(), POSEIDON_TREE_DEPTH, POSEIDON_TREE_ARITY>;

/// A preliminary input to a transaction that is yet to be proven.
pub struct PreInput<'a> {
    /// Input note to be used in the transaction.
    pub note: Note,
    /// Opening from the `input` to the Merkle root of the state.
    pub opening: Opening,
    /// Decrypted value of the input note.
    pub value: u64,
    /// Secret key to generate the nullifier of the input note.
    pub ssk: &'a SecretKey,
}

/// An input to a transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Input {
    /// Nulifier generated from the input note.
    pub nullifier: BlsScalar,
    /// Opening from the `input` to the Merkle root of the state.
    pub opening: Opening,
    /// Input note to be used in the transaction.
    pub note: Note,
    /// Decrypted value of the input note.
    pub value: u64,
    /// Blinding factor used to construct the note.
    pub blinder: JubJubScalar,
    /// Stealth address derived from the key of the owner of the note.
    pub pk_r_prime: JubJubExtended,
    /// Schnorr signature to prove the ownership of the note.
    pub sig: SignatureDouble,
}

/// A preliminary output to a transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct OutputValue {
    /// Type of the output note to be used in the transaction.
    pub r#type: NoteType,
    /// Value of the output.
    pub value: u64,
    /// Public key that will receive the note as spendable input.
    pub receiver: PublicKey,
    /// Nonce/reference to be attached to the note.
    pub ref_id: u64,
}

/// An output to a transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Output {
    /// Computed output note to be used in the transaction.
    pub note: Note,
    /// Decrypted value of the output note.
    pub value: u64,
    /// Blinding factor used to construct the note.
    pub blinder: JubJubScalar,
}

/// A crossover to a transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WasmCrossover {
    /// Crossover value to be used in inter-contract calls.
    pub crossover: PhoenixCrossover,
    /// Value of the crossover.
    pub value: u64,
    /// Blinding factor used to construct the crossover.
    pub blinder: JubJubScalar,
}

/// A call data payload to a transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct CallData {
    /// Contract ID to be called.
    pub contract: ContractId,
    /// Name of the method to be called.
    pub method: String,
    /// Payload of the call to be sent to the contract module.
    pub payload: Vec<u8>,
}

/// A transaction that is yet to be proven.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UnprovenTransaction {
    /// Inputs to the transaction.
    pub inputs: Vec<Input>,
    /// Outputs to the transaction.
    pub outputs: Vec<Output>,
    /// Merkle root of the state for the inputs openings.
    pub anchor: BlsScalar,
    /// Fee setup for the transaction.
    pub fee: Fee,
    /// Crossover value for inter-contract calls.
    pub crossover: Option<WasmCrossover>,
    /// Call data payload for contract calls.
    pub call: Option<CallData>,
}

impl UnprovenTransaction {
    /// Creates a new unproven transaction from the arguments.
    ///
    /// The transaction can be sent to a prover service and it contains all the
    /// data required to generate a ZK proof of validity.
    #[allow(clippy::too_many_arguments)]
    pub fn new<'a, Rng: Clone, I, O>(
        rng: &mut Rng,
        inputs: I,
        outputs: O,
        fee: Fee,
        crossover: Option<CrossoverType>,
        call: Option<types::ExecuteCall>,
    ) -> Option<Self>
    where
        Rng: RngCore + CryptoRng,
        I: IntoIterator<Item = PreInput<'a>>,
        O: IntoIterator<Item = types::ExecuteOutput>,
    {
        let (nullifiers, inputs): (Vec<_>, Vec<_>) = inputs
            .into_iter()
            .map(|i| {
                let nullifier = i.note.gen_nullifier(i.ssk);
                (nullifier, i)
            })
            .unzip();

        let anchor = inputs.first().map(|i| i.opening.root().hash)?;

        let mut output_notes = Vec::with_capacity(4);
        let mut outputs_values = Vec::with_capacity(4);

        let rng = rng.clone();

        for types::ExecuteOutput {
            note_type,
            receiver,
            ref_id: _,
            value,
        } in outputs.into_iter()
        {
            let r#type = match note_type {
                types::OutputType::Transparent => NoteType::Transparent,
                types::OutputType::Obfuscated => NoteType::Obfuscated,
            };

            let r = JubJubScalar::random(rng.clone());
            let blinder = JubJubScalar::random(rng.clone());
            let nonce = BlsScalar::random(&mut rng.clone());
            let receiver = utils::bs58_to_psk(&receiver)?;
            let note = Note::deterministic(
                r#type, &r, nonce, &receiver, value, blinder,
            );

            output_notes.push(note);
            outputs_values.push(Output {
                note,
                value,
                blinder,
            });
        }

        let outputs = outputs_values;

        let call = match call {
            Some(types::ExecuteCall {
                contract,
                method,
                payload,
            }) => {
                let decoded = bs58::decode(contract).into_vec().ok()?;
                if decoded.len() != mem::size_of::<ContractId>() {
                    return None;
                }
                let mut contract = ContractId::uninitialized();
                contract.as_bytes_mut().copy_from_slice(&decoded);
                Some(CallData {
                    contract,
                    method,
                    payload,
                })
            }
            None => None,
        };
        let call_phoenix = call.as_ref().map(|c| {
            (c.contract.to_bytes(), c.method.clone(), c.payload.clone())
        });

        let crossover = crossover.and_then(
            |CrossoverType {
                 blinder,
                 crossover,
                 value,
             }| {
                Some({
                    WasmCrossover {
                        crossover: rkyv::from_bytes::<PhoenixCrossover>(
                            &crossover,
                        )
                        .ok()?,
                        value,
                        blinder: rkyv::from_bytes::<JubJubScalar>(&blinder)
                            .ok()?,
                    }
                })
            },
        );

        let tx_hash = Transaction::hash_input_bytes_from_components(
            &nullifiers,
            &output_notes,
            &anchor,
            &fee,
            &crossover.as_ref().map(|c| c.crossover),
            &call_phoenix,
        );
        let tx_hash = Hasher::digest(tx_hash);

        let inputs = inputs
            .into_iter()
            .zip(nullifiers.into_iter())
            .map(
                |(
                    PreInput {
                        note,
                        opening,
                        value,
                        ssk,
                    },
                    nullifier,
                )| {
                    let vk = ViewKey::from(ssk);
                    let nsk = ssk.sk_r(note.stealth_address());
                    let blinder =
                        note.blinding_factor(Some(&vk)).map_err(|_| ())?;

                    let pk_r_prime = GENERATOR_NUMS_EXTENDED * nsk.as_ref();
                    let sig = nsk.sign_double(&mut rng.clone(), tx_hash);

                    Ok(Input {
                        nullifier,
                        opening,
                        note,
                        value,
                        blinder,
                        pk_r_prime,
                        sig,
                    })
                },
            )
            .collect::<Result<Vec<_>, ()>>()
            .ok()?;

        Some(UnprovenTransaction {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
        })
    }
}
