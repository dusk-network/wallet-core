// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::tx::UnprovenTransaction;
use crate::{
    BalanceInfo, ProverClient, StakeInfo, StateClient, Store, MAX_CALL_SIZE,
};

use core::convert::Infallible;

use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{Error as BytesError, Serializable};
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{
    Ownable, PublicSpendKey, SecretKey as SchnorrKey, SecretSpendKey,
};
use dusk_schnorr::Signature as SchnorrSignature;
use ff::Field;
use phoenix_core::transaction::{stct_signature_message, Transaction};
use phoenix_core::{Error as PhoenixError, Fee, Note, NoteType};
use rand_core::{CryptoRng, Error as RngError, RngCore};
use rkyv::ser::serializers::{
    AllocScratchError, AllocSerializer, CompositeSerializerError,
    SharedSerializeMapError,
};
use rkyv::validation::validators::CheckDeserializeError;
use rkyv::Serialize;
use rusk_abi::ContractId;
use stake_contract_types::{
    allow_signature_message, stake_signature_message,
    unstake_signature_message, withdraw_signature_message,
};
use stake_contract_types::{Allow, Stake, Unstake, Withdraw};

const MAX_INPUT_NOTES: usize = 4;

const TX_STAKE: &str = "stake";
const TX_UNSTAKE: &str = "unstake";
const TX_WITHDRAW: &str = "withdraw";
const TX_ADD_ALLOWLIST: &str = "allow";

type SerializerError = CompositeSerializerError<
    Infallible,
    AllocScratchError,
    SharedSerializeMapError,
>;

/// The error type returned by this crate.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Error<S: Store, SC: StateClient, PC: ProverClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the state client.
    State(SC::Error),
    /// Error originating from the prover client.
    Prover(PC::Error),
    /// Rkyv serialization.
    Rkyv,
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// Bytes were meant to be utf8 but aren't.
    Utf8(FromUtf8Error),
    /// Originating from the transaction model.
    Phoenix(PhoenixError),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
    /// The key is already staked. This happens when there already is an amount
    /// staked for a key and the user tries to make a stake transaction.
    AlreadyStaked {
        /// The key that already has a stake.
        key: PublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
    /// The key is not staked. This happens when a key doesn't have an amount
    /// staked and the user tries to make an unstake transaction.
    NotStaked {
        /// The key that is not staked.
        key: PublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
    /// The key has no reward. This happens when a key has no reward in the
    /// stake contract and the user tries to make a withdraw transaction.
    NoReward {
        /// The key that has no reward.
        key: PublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
}

impl<S: Store, SC: StateClient, PC: ProverClient> Error<S, SC, PC> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
    /// Returns an error from the underlying state client.
    pub fn from_state_err(se: SC::Error) -> Self {
        Self::State(se)
    }
    /// Returns an error from the underlying prover client.
    pub fn from_prover_err(pe: PC::Error) -> Self {
        Self::Prover(pe)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<SerializerError>
    for Error<S, SC, PC>
{
    fn from(_: SerializerError) -> Self {
        Self::Rkyv
    }
}

impl<C, D, S: Store, SC: StateClient, PC: ProverClient>
    From<CheckDeserializeError<C, D>> for Error<S, SC, PC>
{
    fn from(_: CheckDeserializeError<C, D>) -> Self {
        Self::Rkyv
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<RngError>
    for Error<S, SC, PC>
{
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<BytesError>
    for Error<S, SC, PC>
{
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<FromUtf8Error>
    for Error<S, SC, PC>
{
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<PhoenixError>
    for Error<S, SC, PC>
{
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, SC, PC> {
    store: S,
    state: SC,
    prover: PC,
}

impl<S, SC, PC> Wallet<S, SC, PC> {
    /// Create a new wallet given the underlying store and node client.
    pub const fn new(store: S, state: SC, prover: PC) -> Self {
        Self {
            store,
            state,
            prover,
        }
    }

    /// Return the inner Store reference
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Return the inner State reference
    pub const fn state(&self) -> &SC {
        &self.state
    }

    /// Return the inner Prover reference
    pub const fn prover(&self) -> &PC {
        &self.prover
    }
}

impl<S, SC, PC> Wallet<S, SC, PC>
where
    S: Store,
    SC: StateClient,
    PC: ProverClient,
{
    /// Retrieve the public spend key with the given index.
    pub fn public_spend_key(
        &self,
        index: u64,
    ) -> Result<PublicSpendKey, Error<S, SC, PC>> {
        self.store
            .retrieve_ssk(index)
            .map(|ssk| ssk.public_spend_key())
            .map_err(Error::from_store_err)
    }

    /// Retrieve the public key with the given index.
    pub fn public_key(
        &self,
        index: u64,
    ) -> Result<PublicKey, Error<S, SC, PC>> {
        self.store
            .retrieve_sk(index)
            .map(|sk| From::from(&sk))
            .map_err(Error::from_store_err)
    }

    /// Fetches the notes and nullifiers in the state and returns the notes that
    /// are still available for spending.
    fn unspent_notes(
        &self,
        ssk: &SecretSpendKey,
    ) -> Result<Vec<Note>, Error<S, SC, PC>> {
        let vk = ssk.view_key();

        let notes =
            self.state.fetch_notes(&vk).map_err(Error::from_state_err)?;

        let nullifiers: Vec<_> =
            notes.iter().map(|(n, _)| n.gen_nullifier(ssk)).collect();

        let existing_nullifiers = self
            .state
            .fetch_existing_nullifiers(&nullifiers)
            .map_err(Error::from_state_err)?;

        let unspent_notes = notes
            .into_iter()
            .zip(nullifiers.into_iter())
            .filter(|(_, nullifier)| !existing_nullifiers.contains(nullifier))
            .map(|((note, _), _)| note)
            .collect();

        Ok(unspent_notes)
    }

    /// Here we fetch the notes and perform a "minimum number of notes
    /// required" algorithm to select which ones to use for this TX. This is
    /// done by picking notes largest to smallest until they combined have
    /// enough accumulated value.
    ///
    /// We also return the outputs with a possible change note (if applicable).
    #[allow(clippy::type_complexity)]
    fn inputs_and_change_output<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender: &SecretSpendKey,
        refund: &PublicSpendKey,
        value: u64,
    ) -> Result<
        (
            Vec<(Note, u64, JubJubScalar)>,
            Vec<(Note, u64, JubJubScalar)>,
        ),
        Error<S, SC, PC>,
    > {
        let notes = self.unspent_notes(sender)?;
        let mut notes_and_values = Vec::with_capacity(notes.len());

        let sender_vk = sender.view_key();

        let mut accumulated_value = 0;
        for note in notes.into_iter() {
            let val = note.value(Some(&sender_vk))?;
            let blinder = note.blinding_factor(Some(&sender_vk))?;

            accumulated_value += val;
            notes_and_values.push((note, val, blinder));
        }

        if accumulated_value < value {
            return Err(Error::NotEnoughBalance);
        }

        let inputs = pick_notes(value, notes_and_values);

        if inputs.is_empty() {
            return Err(Error::NoteCombinationProblem);
        }

        let change = inputs.iter().map(|v| v.1).sum::<u64>() - value;

        let mut outputs = vec![];
        if change > 0 {
            let nonce = BlsScalar::random(&mut *rng);
            let (change_note, change_blinder) =
                generate_obfuscated_note(rng, refund, change, nonce);

            outputs.push((change_note, change, change_blinder))
        }

        Ok((inputs, outputs))
    }

    /// Execute a generic contract call
    #[allow(clippy::too_many_arguments)]
    pub fn execute<Rng, C>(
        &self,
        rng: &mut Rng,
        contract_id: ContractId,
        call_name: String,
        call_data: C,
        sender_index: u64,
        refund: &PublicSpendKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>>
    where
        Rng: RngCore + CryptoRng,
        C: Serialize<AllocSerializer<MAX_CALL_SIZE>>,
    {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            gas_limit * gas_price,
        )?;

        let fee = Fee::new(rng, gas_limit, gas_price, refund);

        let call_data = rkyv::to_bytes(&call_data)?.to_vec();
        let call = (contract_id, call_name, call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            None,
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Transfer Dusk from one key to another.
    #[allow(clippy::too_many_arguments)]
    pub fn transfer<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        refund: &PublicSpendKey,
        receiver: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
        ref_id: BlsScalar,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let (inputs, mut outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            value + gas_limit * gas_price,
        )?;

        let (output_note, output_blinder) =
            generate_obfuscated_note(rng, receiver, value, ref_id);

        outputs.push((output_note, value, output_blinder));

        let crossover = None;
        let fee = Fee::new(rng, gas_limit, gas_price, refund);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            crossover,
            None,
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Stakes an amount of Dusk.
    #[allow(clippy::too_many_arguments)]
    pub fn stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let pk = PublicKey::from(&sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            value + gas_limit * gas_price,
        )?;

        let stake =
            self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;
        if stake.amount.is_some() {
            return Err(Error::AlreadyStaked { key: pk, stake });
        }

        let blinder = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, refund, value, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let contract_id = rusk_abi::STAKE_CONTRACT;
        let address = rusk_abi::contract_to_scalar(&contract_id);

        let contract_id = rusk_abi::contract_to_scalar(&contract_id);

        let stct_message =
            stct_signature_message(&crossover, value, contract_id);
        let stct_message = dusk_poseidon::sponge::hash(&stct_message);

        let sk_r = *sender.sk_r(fee.stealth_address()).as_ref();
        let secret = SchnorrKey::from(sk_r);

        let stct_signature = SchnorrSignature::new(&secret, rng, stct_message);

        let spend_proof = self
            .prover
            .request_stct_proof(
                &fee,
                &crossover,
                value,
                blinder,
                address,
                stct_signature,
            )
            .map_err(Error::from_prover_err)?
            .to_bytes()
            .to_vec();

        let msg = stake_signature_message(stake.counter, value);
        let signature = sk.sign(&pk, &msg);

        let stake = Stake {
            public_key: pk,
            signature,
            value,
            proof: spend_proof,
        };

        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&stake)?.to_vec();
        let call =
            (rusk_abi::STAKE_CONTRACT, String::from(TX_STAKE), call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            Some((crossover, value, blinder)),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Unstake a key from the stake contract.
    pub fn unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let public_key = PublicKey::from(&sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            gas_limit * gas_price,
        )?;

        let stake = self
            .state
            .fetch_stake(&public_key)
            .map_err(Error::from_state_err)?;
        let (value, _) = stake.amount.ok_or(Error::NotStaked {
            key: public_key,
            stake,
        })?;

        let blinder = JubJubScalar::random(rng);

        // Since we're not transferring value *to* the contract the crossover
        // shouldn't contain a value. As such the note used to create it should
        // be valueless as well.
        let note = Note::obfuscated(rng, refund, 0, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let unstake_note =
            Note::transparent(rng, &sender.public_spend_key(), value);
        let unstake_blinder = unstake_note
            .blinding_factor(None)
            .expect("Note is transparent so blinding factor is unencrypted");

        let unstake_proof = self
            .prover
            .request_wfct_proof(
                unstake_note.value_commitment().into(),
                value,
                unstake_blinder,
            )
            .map_err(Error::from_prover_err)?
            .to_bytes()
            .to_vec();

        let unstake_note = unstake_note.to_bytes();
        let signature_message =
            unstake_signature_message(stake.counter, unstake_note);

        let signature = sk.sign(&public_key, &signature_message);

        let unstake = Unstake {
            public_key,
            signature,
            note: unstake_note.to_vec(),
            proof: unstake_proof,
        };

        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&unstake)?.to_vec();
        let call = (
            rusk_abi::STAKE_CONTRACT,
            String::from(TX_UNSTAKE),
            call_data,
        );

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            Some((crossover, 0, blinder)),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Withdraw the reward a key has reward if accumulated by staking and
    /// taking part in operating the network.
    pub fn withdraw<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_psk = sender.public_spend_key();

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let pk = PublicKey::from(&sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            gas_limit * gas_price,
        )?;

        let stake =
            self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;
        if stake.reward == 0 {
            return Err(Error::NoReward { key: pk, stake });
        }

        let withdraw_r = JubJubScalar::random(rng);
        let address = sender_psk.gen_stealth_address(&withdraw_r);
        let nonce = BlsScalar::random(&mut *rng);

        let msg = withdraw_signature_message(stake.counter, address, nonce);
        let signature = sk.sign(&pk, &msg);

        // Since we're not transferring value *to* the contract the crossover
        // shouldn't contain a value. As such the note used to created it should
        // be valueless as well.
        let blinder = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, refund, 0, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let withdraw = Withdraw {
            public_key: pk,
            signature,
            address,
            nonce,
        };
        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&withdraw)?.to_vec();

        let contract_id = rusk_abi::STAKE_CONTRACT;
        let call = (contract_id, String::from(TX_WITHDRAW), call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            Some((crossover, 0, blinder)),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Allow a `staker` public key.
    #[allow(clippy::too_many_arguments)]
    pub fn allow<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        owner_index: u64,
        refund: &PublicSpendKey,
        staker: &PublicKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let owner_sk = self
            .store
            .retrieve_sk(owner_index)
            .map_err(Error::from_store_err)?;
        let owner_pk = PublicKey::from(&owner_sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            gas_limit * gas_price,
        )?;

        let stake = self
            .state
            .fetch_stake(&owner_pk)
            .map_err(Error::from_state_err)?;

        let msg = allow_signature_message(stake.counter, staker);
        let signature = owner_sk.sign(&owner_pk, &msg);

        // Since we're not transferring value *to* the contract the crossover
        // shouldn't contain a value. As such the note used to created it should
        // be valueless as well.
        let blinder = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, refund, 0, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let allow = Allow {
            public_key: *staker,
            owner: owner_pk,
            signature,
        };
        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&allow)?.to_vec();

        let contract_id = rusk_abi::STAKE_CONTRACT;
        let call = (contract_id, String::from(TX_ADD_ALLOWLIST), call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            Some((crossover, 0, blinder)),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Gets the balance of a key.
    pub fn get_balance(
        &self,
        ssk_index: u64,
    ) -> Result<BalanceInfo, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(ssk_index)
            .map_err(Error::from_store_err)?;
        let vk = sender.view_key();

        let notes = self.unspent_notes(&sender)?;
        let mut values = Vec::with_capacity(notes.len());

        for note in notes.into_iter() {
            values.push(note.value(Some(&vk))?);
        }
        values.sort_by(|a, b| b.cmp(a));

        let spendable = values.iter().take(MAX_INPUT_NOTES).sum();
        let value =
            spendable + values.iter().skip(MAX_INPUT_NOTES).sum::<u64>();

        Ok(BalanceInfo { value, spendable })
    }

    /// Gets the stake and the expiration of said stake for a key.
    pub fn get_stake(
        &self,
        sk_index: u64,
    ) -> Result<StakeInfo, Error<S, SC, PC>> {
        let sk = self
            .store
            .retrieve_sk(sk_index)
            .map_err(Error::from_store_err)?;

        let pk = PublicKey::from(&sk);

        let s = self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;

        Ok(s)
    }
}

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The notes are picked in a way to maximize the number of notes used, while
/// minimizing the value employed. To do this we sort the notes in ascending
/// value order, and go through each combination in a lexicographic order
/// until we find the first combination whose sum is larger or equal to
/// the given value. If such a slice is not found, an empty vector is returned.
///
/// Note: it is presupposed that the input notes contain enough balance to cover
/// the given `value`.
fn pick_notes(
    value: u64,
    notes_and_values: Vec<(Note, u64, JubJubScalar)>,
) -> Vec<(Note, u64, JubJubScalar)> {
    let mut notes_and_values = notes_and_values;
    let len = notes_and_values.len();

    if len <= MAX_INPUT_NOTES {
        return notes_and_values;
    }

    notes_and_values.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

    pick_lexicographic(notes_and_values.len(), |indices| {
        indices
            .iter()
            .map(|index| notes_and_values[*index].1)
            .sum::<u64>()
            >= value
    })
    .map(|indices| {
        indices
            .into_iter()
            .map(|index| notes_and_values[index])
            .collect()
    })
    .unwrap_or_default()
}

fn pick_lexicographic<F: Fn(&[usize; MAX_INPUT_NOTES]) -> bool>(
    max_len: usize,
    is_valid: F,
) -> Option<[usize; MAX_INPUT_NOTES]> {
    let mut indices = [0; MAX_INPUT_NOTES];
    indices
        .iter_mut()
        .enumerate()
        .for_each(|(i, index)| *index = i);

    loop {
        if is_valid(&indices) {
            return Some(indices);
        }

        let mut i = MAX_INPUT_NOTES - 1;

        while indices[i] == i + max_len - MAX_INPUT_NOTES {
            if i > 0 {
                i -= 1;
            } else {
                break;
            }
        }

        indices[i] += 1;
        for j in i + 1..MAX_INPUT_NOTES {
            indices[j] = indices[j - 1] + 1;
        }

        if indices[MAX_INPUT_NOTES - 1] == max_len {
            break;
        }
    }

    None
}

/// Generates an obfuscated note for the given public spend key.
fn generate_obfuscated_note<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    psk: &PublicSpendKey,
    value: u64,
    nonce: BlsScalar,
) -> (Note, JubJubScalar) {
    let r = JubJubScalar::random(rng);
    let blinder = JubJubScalar::random(rng);

    (
        Note::deterministic(
            NoteType::Obfuscated,
            &r,
            nonce,
            psk,
            value,
            blinder,
        ),
        blinder,
    )
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand_core::SeedableRng;

    use super::*;

    fn gen_notes(values: &[u64]) -> Vec<(Note, u64, JubJubScalar)> {
        let mut rng = StdRng::seed_from_u64(0xbeef);

        let ssk = SecretSpendKey::random(&mut rng);
        let psk = ssk.public_spend_key();

        let mut notes_and_values = Vec::with_capacity(values.len());

        for value in values {
            let note = Note::transparent(&mut rng, &psk, *value);
            let blinder = JubJubScalar::random(&mut rng);

            notes_and_values.push((note, *value, blinder));
        }

        notes_and_values
    }

    #[test]
    fn note_picking_none() {
        let values = [2, 1, 4, 3, 5, 7, 6];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(100, notes_and_values);

        assert_eq!(picked.len(), 0);
    }

    #[test]
    fn note_picking_1() {
        let values = [1];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(1, notes_and_values);
        assert_eq!(picked.len(), 1);
    }

    #[test]
    fn note_picking_2() {
        let values = [1, 2];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(2, notes_and_values);
        assert_eq!(picked.len(), 2);
    }

    #[test]
    fn note_picking_3() {
        let values = [1, 3, 2];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(2, notes_and_values);
        assert_eq!(picked.len(), 3);
    }

    #[test]
    fn note_picking_4() {
        let values = [4, 2, 1, 3];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(2, notes_and_values);
        assert_eq!(picked.len(), 4);
    }

    #[test]
    fn note_picking_4_plus() {
        let values = [2, 1, 4, 3, 5, 7, 6];

        let notes_and_values = gen_notes(&values);

        let picked = pick_notes(20, notes_and_values);

        assert_eq!(picked.len(), 4);
        assert_eq!(picked.iter().map(|v| v.1).sum::<u64>(), 20);
    }
}
