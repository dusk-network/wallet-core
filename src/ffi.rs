// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The foreign function interface for the wallet.

use alloc::string::String;
use alloc::vec::Vec;

use core::mem;
use core::num::NonZeroU32;
use core::ptr;

use bls12_381_bls::PublicKey as StakePublicKey;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_plonk::prelude::Proof;
use jubjub_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Note, PublicKey, ViewKey};
use poseidon_merkle::Opening as PoseidonOpening;
use rand_core::{
    impls::{next_u32_via_fill, next_u64_via_fill},
    CryptoRng, RngCore,
};
use rusk_abi::ContractId;

use crate::tx::UnprovenTransaction;
use crate::{
    BalanceInfo, EnrichedNote, Error, ProverClient, StakeInfo, StateClient,
    Store, Transaction, Wallet, POSEIDON_TREE_DEPTH,
};

extern "C" {
    /// Retrieves the seed from the store.
    fn get_seed(seed: *mut [u8; 64]) -> u8;

    /// Fills a buffer with random numbers.
    fn fill_random(buf: *mut u8, buf_len: u32) -> u8;

    /// Asks the node to finds the notes for a specific view key.
    ///
    /// An implementor should allocate - see [`malloc`] - a buffer large enough
    /// to contain the serialized notes (and the corresponding block height) and
    /// write them all in sequence. A pointer to the first element of the
    /// buffer should then be written in `notes`, while the number of bytes
    /// written should be put in `notes_len`.
    ///
    /// E.g: note1, block_height, note2, block_height, etc...
    fn fetch_notes(
        vk: *const [u8; ViewKey::SIZE],
        notes: *mut *mut u8,
        notes_len: *mut u32,
    ) -> u8;

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        note: *const [u8; Note::SIZE],
        opening: *mut u8,
        opening_len: *mut u32,
    ) -> u8;

    /// Asks the node to find the nullifiers that are already in the state and
    /// returns them.
    ///
    /// The nullifiers are to be serialized in sequence and written to
    /// `existing_nullifiers` and their number should be written to
    /// `existing_nullifiers_len`.
    fn fetch_existing_nullifiers(
        nullifiers: *const u8,
        nullifiers_len: u32,
        existing_nullifiers: *mut u8,
        existing_nullifiers_len: *mut u32,
    ) -> u8;

    /// Fetches the current anchor.
    fn fetch_anchor(anchor: *mut [u8; BlsScalar::SIZE]) -> u8;

    /// Fetches the current stake for a key.
    ///
    /// The value, eligibility, reward and counter should be written in
    /// sequence, little endian, to the given buffer. If there is no value and
    /// eligibility, the first 16 bytes should be zero.
    fn fetch_stake(
        stake_pk: *const [u8; StakePublicKey::SIZE],
        stake: *mut [u8; StakeInfo::SIZE],
    ) -> u8;

    /// Request the node to prove the given unproven transaction.
    fn compute_proof_and_propagate(
        utx: *const u8,
        utx_len: u32,
        tx: *mut u8,
        tx_len: *mut u32,
    ) -> u8;

    /// Requests the node to prove STCT.
    fn request_stct_proof(
        inputs: *const [u8; STCT_INPUT_SIZE],
        proof: *mut [u8; Proof::SIZE],
    ) -> u8;

    /// Request the node to prove WFCT.
    fn request_wfct_proof(
        inputs: *const [u8; WFCT_INPUT_SIZE],
        proof: *mut [u8; Proof::SIZE],
    ) -> u8;
}

macro_rules! unwrap_or_bail {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return Error::<FfiStore, FfiStateClient, FfiProverClient>::from(e).into();
            }
        }
    };
}

type FfiWallet = Wallet<FfiStore, FfiStateClient, FfiProverClient>;
const WALLET: FfiWallet =
    Wallet::new(FfiStore, FfiStateClient, FfiProverClient);

/// Allocates memory with a given size.
#[no_mangle]
pub unsafe extern "C" fn malloc(cap: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(cap as usize);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

/// Free memory pointed to by the given `ptr`, and the given `cap`acity.
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut u8, cap: u32) {
    Vec::from_raw_parts(ptr, 0, cap as usize);
}

/// Get the public spend key with the given index.
#[no_mangle]
pub unsafe extern "C" fn public_key(
    index: *const u64,
    pk: *mut [u8; PublicKey::SIZE],
) -> u8 {
    let key = unwrap_or_bail!(WALLET.public_key(*index)).to_bytes();
    ptr::copy_nonoverlapping(&key[0], &mut (*pk)[0], key.len());
    0
}

/// Execute a generic contract call
#[no_mangle]
pub unsafe extern "C" fn execute(
    contract_id: *const [u8; 32],
    call_name_ptr: *mut u8,
    call_name_len: *const u32,
    call_data_ptr: *mut u8,
    call_data_len: *const u32,
    sender_index: *const u64,
    refund: *const [u8; PublicKey::SIZE],
    gas_limit: *const u64,
    gas_price: *const u64,
) -> u8 {
    let contract_id = ContractId::from_bytes(*contract_id);

    // SAFETY: these buffers are expected to have been allocated with the
    // correct size. If this is not the case problems with the allocator
    // *may* happen.
    let call_name = Vec::from_raw_parts(
        call_name_ptr,
        call_name_len as usize,
        call_name_len as usize,
    );
    let call_name = unwrap_or_bail!(String::from_utf8(call_name));

    let call_data = Vec::from_raw_parts(
        call_data_ptr,
        call_data_len as usize,
        call_data_len as usize,
    );

    let refund = unwrap_or_bail!(PublicKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.execute(
        &mut FfiRng,
        contract_id,
        call_name,
        call_data,
        *sender_index,
        &refund,
        *gas_price,
        *gas_limit
    ));

    0
}

/// Creates a transfer transaction.
#[no_mangle]
pub unsafe extern "C" fn transfer(
    sender_index: *const u64,
    refund: *const [u8; PublicKey::SIZE],
    receiver: *const [u8; PublicKey::SIZE],
    value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    ref_id: Option<&u64>,
) -> u8 {
    let refund = unwrap_or_bail!(PublicKey::from_bytes(&*refund));
    let receiver = unwrap_or_bail!(PublicKey::from_bytes(&*receiver));

    let ref_id =
        BlsScalar::from(ref_id.copied().unwrap_or_else(|| FfiRng.next_u64()));

    unwrap_or_bail!(WALLET.transfer(
        &mut FfiRng,
        *sender_index,
        &refund,
        &receiver,
        *value,
        *gas_price,
        *gas_limit,
        ref_id
    ));

    0
}

/// Creates a stake transaction.
#[no_mangle]
pub unsafe extern "C" fn stake(
    sender_index: *const u64,
    staker_index: *const u64,
    refund: *const [u8; PublicKey::SIZE],
    value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
) -> u8 {
    let refund = unwrap_or_bail!(PublicKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.stake(
        &mut FfiRng,
        *sender_index,
        *staker_index,
        &refund,
        *value,
        *gas_price,
        *gas_limit
    ));

    0
}

/// Unstake the value previously staked using the [`stake`] function.
#[no_mangle]
pub unsafe extern "C" fn unstake(
    sender_index: *const u64,
    staker_index: *const u64,
    refund: *const [u8; PublicKey::SIZE],
    gas_limit: *const u64,
    gas_price: *const u64,
) -> u8 {
    let refund = unwrap_or_bail!(PublicKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.unstake(
        &mut FfiRng,
        *sender_index,
        *staker_index,
        &refund,
        *gas_price,
        *gas_limit
    ));

    0
}

/// Withdraw the rewards accumulated as a result of staking and taking part in
/// the consensus.
#[no_mangle]
pub unsafe extern "C" fn withdraw(
    sender_index: *const u64,
    staker_index: *const u64,
    refund: *const [u8; PublicKey::SIZE],
    gas_limit: *const u64,
    gas_price: *const u64,
) -> u8 {
    let refund = unwrap_or_bail!(PublicKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.withdraw(
        &mut FfiRng,
        *sender_index,
        *staker_index,
        &refund,
        *gas_price,
        *gas_limit
    ));

    0
}

/// Gets the balance of a secret spend key.
#[no_mangle]
pub unsafe extern "C" fn get_balance(
    sk_index: *const u64,
    balance: *mut [u8; BalanceInfo::SIZE],
) -> u8 {
    let b = unwrap_or_bail!(WALLET.get_balance(*sk_index)).to_bytes();
    ptr::copy_nonoverlapping(&b[0], &mut (*balance)[0], b.len());
    0
}

/// Gets the stake of a key. The value, eligibility, reward, and counter are
/// written in sequence to the given buffer. If there is no value and
/// eligibility the first 16 bytes will be zero.
#[no_mangle]
pub unsafe extern "C" fn get_stake(
    sk_index: *const u64,
    stake: *mut [u8; StakeInfo::SIZE],
) -> u8 {
    let s = unwrap_or_bail!(WALLET.get_stake(*sk_index)).to_bytes();
    ptr::copy_nonoverlapping(&s[0], &mut (*stake)[0], s.len());
    0
}

struct FfiStore;

impl Store for FfiStore {
    type Error = u8;

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        let mut seed = [0; 64];
        unsafe {
            let r = get_seed(&mut seed);
            if r != 0 {
                return Err(r);
            }
        }
        Ok(seed)
    }
}

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

struct FfiStateClient;

impl StateClient for FfiStateClient {
    type Error = u8;

    fn fetch_notes(
        &self,
        vk: &ViewKey,
    ) -> Result<Vec<EnrichedNote>, Self::Error> {
        let mut notes_ptr = ptr::null_mut();
        let mut notes_len = 0;

        let notes_buf = unsafe {
            let r = fetch_notes(&vk.to_bytes(), &mut notes_ptr, &mut notes_len);
            if r != 0 {
                return Err(r);
            }

            // SAFETY: the buffer is expected to have been allocated with the
            // correct size. If this is not the case problems with the allocator
            // *may* happen.
            Vec::from_raw_parts(
                notes_ptr,
                notes_len as usize,
                notes_len as usize,
            )
        };

        let num_notes = notes_len as usize / (Note::SIZE + u64::SIZE);
        let mut notes = Vec::with_capacity(num_notes);

        let mut buf = &notes_buf[..];
        for _ in 0..num_notes {
            let note = Note::from_reader(&mut buf).map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?;
            let block_height = u64::from_reader(&mut buf).map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?;
            notes.push((note, block_height));
        }

        Ok(notes)
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let mut scalar_buf = [0; BlsScalar::SIZE];
        unsafe {
            let r = fetch_anchor(&mut scalar_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let scalar: Option<BlsScalar> =
            BlsScalar::from_bytes(&scalar_buf).into();
        scalar.ok_or(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from(
                dusk_bytes::Error::InvalidData,
            )
            .into(),
        )
    }

    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        let nullifiers_len = nullifiers.len();
        let mut nullifiers_buf = vec![0u8; BlsScalar::SIZE * nullifiers_len];

        // If no nullifiers come in, then none of them exist in the state.
        if nullifiers_len == 0 {
            return Ok(vec![]);
        }

        let mut writer = &mut nullifiers_buf[..];

        for nullifier in nullifiers {
            writer.write(&nullifier.to_bytes()).map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?;
        }

        let mut existing_nullifiers_buf =
            vec![0u8; BlsScalar::SIZE * nullifiers_len];
        let mut existing_nullifiers_len = 0;

        unsafe {
            let r = fetch_existing_nullifiers(
                &nullifiers_buf[0],
                nullifiers_len as u32,
                &mut existing_nullifiers_buf[0],
                &mut existing_nullifiers_len,
            );
            if r != 0 {
                return Err(r);
            }
        };

        let mut existing_nullifiers =
            Vec::with_capacity(existing_nullifiers_len as usize);

        let mut reader = &existing_nullifiers_buf[..];
        for _ in 0..existing_nullifiers_len {
            existing_nullifiers.push(
                BlsScalar::from_reader(&mut reader).map_err(
                    Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
                )?,
            );
        }

        Ok(existing_nullifiers)
    }

    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>, Self::Error> {
        const OPENING_BUF_SIZE: usize = 3000;

        let mut opening_buf = Vec::with_capacity(OPENING_BUF_SIZE);
        let mut opening_len = 0;

        let note = note.to_bytes();
        unsafe {
            let r = fetch_opening(
                &note,
                opening_buf.as_mut_ptr(),
                &mut opening_len,
            );
            if r != 0 {
                return Err(r);
            }
        }

        let branch = rkyv::from_bytes(&opening_buf[..opening_len as usize])
            .map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?;

        Ok(branch)
    }

    fn fetch_stake(
        &self,
        stake_pk: &StakePublicKey,
    ) -> Result<StakeInfo, Self::Error> {
        let stake_pk = stake_pk.to_bytes();
        let mut stake_buf = [0u8; StakeInfo::SIZE];

        unsafe {
            let r = fetch_stake(&stake_pk, &mut stake_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let stake = StakeInfo::from_bytes(&stake_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(stake)
    }
}

struct FfiProverClient;

impl ProverClient for FfiProverClient {
    type Error = u8;

    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error> {
        let utx_bytes = utx.to_var_bytes();

        // A transaction is always smaller than an unproven transaction
        let mut tx_buf = vec![0; utx_bytes.len()];
        let mut tx_len = 0;

        unsafe {
            let r = compute_proof_and_propagate(
                &utx_bytes[0],
                utx_bytes.len() as u32,
                &mut tx_buf[0],
                &mut tx_len,
            );
            if r != 0 {
                return Err(r);
            }
        }

        let transaction = Transaction::from_slice(&tx_buf[..tx_len as usize])
            .map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(transaction)
    }

    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; STCT_INPUT_SIZE];

        let mut writer = &mut buf[..];
        writer.write(&fee.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&crossover.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&value.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&blinder.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&address.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&signature.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        let mut proof_buf = [0; Proof::SIZE];

        unsafe {
            let r = request_stct_proof(&buf, &mut proof_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let proof = Proof::from_bytes(&proof_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        Ok(proof)
    }

    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; WFCT_INPUT_SIZE];

        let mut writer = &mut buf[..];
        writer.write(&commitment.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&value.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&blinder.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        let mut proof_buf = [0; Proof::SIZE];

        unsafe {
            let r = request_wfct_proof(&buf, &mut proof_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let proof = Proof::from_bytes(&proof_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        Ok(proof)
    }
}

struct FfiRng;

impl CryptoRng for FfiRng {}

impl RngCore for FfiRng {
    fn next_u32(&mut self) -> u32 {
        next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).ok();
    }

    fn try_fill_bytes(
        &mut self,
        dest: &mut [u8],
    ) -> Result<(), rand_core::Error> {
        let buf = dest.as_mut_ptr();
        let len = dest.len();

        // SAFETY: this is unsafe since the passed function is not guaranteed to
        // be a CSPRNG running in a secure context. We therefore consider it the
        // responsibility of the user to pass a good generator.
        unsafe {
            match fill_random(buf, len as u32) {
                0 => Ok(()),
                v => {
                    let nzu = NonZeroU32::new(v as u32).unwrap();
                    Err(rand_core::Error::from(nzu))
                }
            }
        }
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<Error<S, SC, PC>>
    for u8
{
    fn from(e: Error<S, SC, PC>) -> Self {
        match e {
            Error::Store(_) => 255,
            Error::Rng(_) => 254,
            Error::Bytes(_) => 253,
            Error::State(_) => 252,
            Error::Prover(_) => 251,
            Error::NotEnoughBalance => 250,
            Error::NoteCombinationProblem => 249,
            Error::Rkyv => 248,
            Error::Phoenix(_) => 247,
            Error::AlreadyStaked { .. } => 246,
            Error::NotStaked { .. } => 245,
            Error::NoReward { .. } => 244,
            Error::Utf8(_) => 243,
        }
    }
}
