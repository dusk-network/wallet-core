// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::tx::UnprovenTransaction;
use crate::{
    BalanceInfo, ProverClient, StakeInfo, StateClient, Store, Transaction,
};

use alloc::vec::Vec;

use canonical::CanonError;
use canonical::EncodeToVec;
use dusk_bls12_381_sign::{PublicKey, SecretKey, Signature};
use dusk_bytes::{Error as BytesError, Serializable};
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{
    Ownable, PublicSpendKey, SecretKey as SchnorrKey, SecretSpendKey,
};
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_schnorr::Signature as SchnorrSignature;
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Note, NoteType};
use rand_core::{CryptoRng, Error as RngError, RngCore};

const MAX_INPUT_NOTES: usize = 4;

/// The error type returned by this crate.
#[derive(Debug)]
pub enum Error<S: Store, SC: StateClient, PC: ProverClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the state client.
    State(SC::Error),
    /// Error originating from the prover client.
    Prover(PC::Error),
    /// Canonical stores.
    Canon(CanonError),
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// Originating from the transaction model.
    Phoenix(PhoenixError),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
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

impl<S: Store, SC: StateClient, PC: ProverClient> From<PhoenixError>
    for Error<S, SC, PC>
{
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<CanonError>
    for Error<S, SC, PC>
{
    fn from(ce: CanonError) -> Self {
        Self::Canon(ce)
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
}

const TX_STAKE: u8 = 0x00;
const TX_WITHDRAW: u8 = 0x01;

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
        height: u64,
        ssk: &SecretSpendKey,
    ) -> Result<Vec<Note>, Error<S, SC, PC>> {
        let vk = ssk.view_key();

        let notes = self
            .state
            .fetch_notes(height, &vk)
            .map_err(Error::from_state_err)?;

        let nullifiers: Vec<_> =
            notes.iter().map(|n| n.gen_nullifier(ssk)).collect();

        let existing_nullifiers = self
            .state
            .fetch_existing_nullifiers(&nullifiers)
            .map_err(Error::from_state_err)?;

        let unspent_notes = notes
            .into_iter()
            .zip(nullifiers.into_iter())
            .filter(|(_, nullifier)| !existing_nullifiers.contains(nullifier))
            .map(|(note, _)| note)
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
        // TODO find a way to get the block height from somewhere. Maybe it
        //  should be determined by the client?
        let mut notes = self.unspent_notes(0, sender)?;
        let mut notes_and_values = Vec::with_capacity(notes.len());

        let sender_vk = sender.view_key();

        let mut accumulated_value = 0;
        for note in notes.drain(..) {
            let val = note.value(Some(&sender_vk))?;
            let blinder = note.blinding_factor(Some(&sender_vk))?;

            accumulated_value += val;
            notes_and_values.push((note, val, blinder));
        }

        if accumulated_value < value {
            return Err(Error::NotEnoughBalance);
        }

        // This sorts the notes from least valuable to most valuable. It
        // helps in the minimum gas spent algorithm, where the largest notes
        // are "popped" first.
        notes_and_values.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

        let mut inputs = Vec::with_capacity(notes.len());

        let mut accumulated_value = 0;
        while accumulated_value < value {
            // This unwrap is ok because at this point we can be sure there
            // is enough value in the notes.
            let (note, val, blinder) = notes_and_values.pop().unwrap();
            accumulated_value += val;
            inputs.push((note, val, blinder));
        }

        if inputs.len() > MAX_INPUT_NOTES {
            return Err(Error::NoteCombinationProblem);
        }

        let change = accumulated_value - value;

        let mut outputs = vec![];
        if change > 0 {
            let nonce = BlsScalar::random(rng);
            let (change_note, change_blinder) =
                generate_obfuscated_note(rng, refund, change, nonce);

            outputs.push((change_note, change, change_blinder))
        }

        Ok((inputs, outputs))
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

        let blinder = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, refund, value, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let contract_id = rusk_abi::stake_contract();
        let address = rusk_abi::contract_to_scalar(&contract_id);

        let stct_signature =
            sign_stct(rng, &sender, &fee, &crossover, value, &address);

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

        let created_at = self
            .state
            .fetch_block_height()
            .map_err(Error::from_state_err)?;

        let signature = stake_sign(&sk, &pk, value, created_at);

        let call_data =
            (TX_STAKE, pk, signature, value, created_at, spend_proof)
                .encode_to_vec();

        let call = (contract_id, call_data);

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

    /// Withdraw a key's stake.
    pub fn withdraw_stake<Rng: RngCore + CryptoRng>(
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
        let pk = PublicKey::from(&sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender,
            refund,
            gas_limit * gas_price,
        )?;

        let stake =
            self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;

        let blinder = JubJubScalar::random(rng);

        // Since we're not transferring value *to* the contract the crossover
        // shouldn't contain a value. As such the note used to created it should
        // be valueless as well.
        let note = Note::obfuscated(rng, refund, 0, blinder);
        let (mut fee, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        fee.gas_limit = gas_limit;
        fee.gas_price = gas_price;

        let withdraw_note =
            Note::transparent(rng, &sender.public_spend_key(), stake.value);
        let withdraw_blinder = withdraw_note
            .blinding_factor(None)
            .expect("Note is transparent so blinding factor is unencrypted");

        let withdraw_proof = self
            .prover
            .request_wfct_proof(
                withdraw_note.value_commitment().into(),
                stake.value,
                withdraw_blinder,
            )
            .map_err(Error::from_prover_err)?
            .to_bytes()
            .to_vec();

        let signature = withdraw_sign(&sk, &pk, &stake, &withdraw_note);

        let call_data =
            (TX_WITHDRAW, pk, signature, withdraw_note, withdraw_proof)
                .encode_to_vec();

        let contract_id = rusk_abi::stake_contract();
        let call = (contract_id, call_data);

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

        let mut notes = self.unspent_notes(0, &sender)?;
        let mut values = Vec::with_capacity(notes.len());

        for note in notes.drain(..) {
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

const STCT_MESSAGE_SIZE: usize = 5 + PoseidonCipher::cipher_size();

// TODO: this is copied from the circuits. We should find a way to reuse this
//  instead of duplicating it.
fn sign_stct<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    ssk: &SecretSpendKey,
    fee: &Fee,
    crossover: &Crossover,
    value: u64,
    address: &BlsScalar,
) -> SchnorrSignature {
    let sk_r = *ssk.sk_r(fee.stealth_address()).as_ref();
    let secret = SchnorrKey::from(sk_r);

    let message = {
        let mut message = [BlsScalar::zero(); STCT_MESSAGE_SIZE];
        let mut m = message.iter_mut();

        crossover
            .value_commitment()
            .to_hash_inputs()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        if let Some(m) = m.next() {
            *m = *crossover.nonce();
        }

        crossover
            .encrypted_data()
            .cipher()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        if let Some(m) = m.next() {
            *m = value.into();
        }
        if let Some(m) = m.next() {
            *m = *address;
        }

        sponge::hash(&message)
    };

    SchnorrSignature::new(&secret, rng, message)
}

/// Creates a signature compatible with what the stake contract expects for a
/// stake transaction.
fn stake_sign(
    sk: &SecretKey,
    pk: &PublicKey,
    value: u64,
    created_at: u64,
) -> Signature {
    let mut bytes = Vec::with_capacity(16);

    bytes.extend(value.to_bytes());
    bytes.extend(created_at.to_le_bytes());

    sk.sign(pk, &bytes)
}

/// Creates a signature compatible with what the stake contract expects for a
/// withdraw transaction.
fn withdraw_sign(
    sk: &SecretKey,
    pk: &PublicKey,
    stake: &StakeInfo,
    note: &Note,
) -> Signature {
    let mut bytes = Vec::with_capacity(24 + Note::SIZE);

    bytes.extend(stake.value.to_le_bytes());
    bytes.extend(stake.eligibility.to_le_bytes());
    bytes.extend(stake.created_at.to_le_bytes());
    bytes.extend(note.to_bytes());

    sk.sign(pk, &bytes)
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
