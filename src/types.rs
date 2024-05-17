// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Arguments and responses to the module requests

// THIS FILE IS AUTO GENERATED!!

#![allow(missing_docs)]

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[doc = " The arguments of the balance function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct BalanceArgs {
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>]; all notes should have their keys derived from "]
    #[doc = " `seed`"]
    pub notes: Vec<u8>,
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " The response of the balance function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct BalanceResponse {
    #[doc = " Maximum value per transaction"]
    pub maximum: u64,
    #[doc = " Total computed balance"]
    pub value: u64,
}
#[doc = " Response of check_note_ownership function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CheckNoteOwnershipResponse {
    #[doc = " The block heights of the notes in the same order the notes were returned seperated by comma"]
    pub block_heights: String,
    #[doc = " The last position of the note"]
    pub last_pos: u64,
    #[doc = " The raw owned note"]
    pub notes: Vec<Vec<u8>>,
    #[doc = " The nullifiers of the notes in the same order the notes were returned"]
    pub nullifiers: Vec<Vec<u8>>,
    #[doc = " The public spend keys of the notes in the same order the notes were returned"]
    pub public_spend_keys: Vec<String>,
}
#[doc = " The value of the Crossover and the blinder"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CrossoverType {
    #[doc = " The rkyv serialized blinder of the crossover"]
    pub blinder: Vec<u8>,
    #[doc = " The rkyv serialized bytes of the crossover struct"]
    pub crossover: Vec<u8>,
    #[doc = " The value of the crossover"]
    pub value: u64,
}
#[doc = " Arguments of the dusk_to_lux function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct DuskToLuxArgs {
    #[doc = " The amount of dusk to convert to lux"]
    pub dusk: u64,
}
#[doc = " Response of the dusk_to_lux function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct DuskToLuxResponse {
    #[doc = " The amount of lux that was converted from dusk"]
    pub lux: f64,
}
#[doc = " The arguments of the execute function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteArgs {
    #[doc = " A call to a contract method"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call: Option<ExecuteCall>,
    #[doc = " The crossover value"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crossover: Option<CrossoverType>,
    #[doc = " A rkyv serialized Fee"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<Vec<u8>>,
    #[doc = " The gas limit of the transaction"]
    pub gas_limit: u64,
    #[doc = " The gas price per unit for the transaction"]
    pub gas_price: u64,
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to be used as inputs"]
    pub inputs: Vec<u8>,
    #[doc = " A rkyv serialized [Vec<tx::Opening>] to open the inputs to a Merkle root, along with the "]
    #[doc = " positions of the notes the openings are of in a tuple (opening, position) rkyv serialized, "]
    #[doc = " see rkyv.rs/rkyv_openings_array"]
    pub openings: Vec<u8>,
    #[doc = " The transfer output note"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<ExecuteOutput>,
    #[doc = " The refund addressin Base58 format"]
    pub refund: String,
    #[doc = " Seed used to derive the entropy for the notes"]
    pub rng_seed: Vec<u8>,
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
    #[doc = " The index of the sender in the seed"]
    pub sender_index: u64,
}
#[doc = " A call to a contract method"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteCall {
    #[doc = " The id of the contract to call in Base58 format"]
    pub contract: String,
    #[doc = " The name of the method to be called"]
    pub method: String,
    #[doc = " The payload of the call"]
    pub payload: Vec<u8>,
}
#[doc = " The output of a transfer"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteOutput {
    #[doc = " The type of the note"]
    pub note_type: OutputType,
    #[doc = " The address of the receiver in Base58 format"]
    pub receiver: String,
    #[doc = " A reference id to be appended to the output"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u64>,
    #[doc = " The value of the output"]
    pub value: u64,
}
#[doc = " Response of the execute function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteResponse {
    #[doc = " The rkyv serialized unproven transaction"]
    pub tx: Vec<u8>,
}
#[doc = " The arguments of the filter_notes function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FilterNotesArgs {
    #[doc = " Boolean flags to be negative filtered"]
    pub flags: Vec<bool>,
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to be filtered"]
    pub notes: Vec<u8>,
}
#[doc = " Arguments of the filter_nullifier_note function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FilterNulifierNotesArgs {
    #[doc = " The existing nullifiers that are spent as a Vec<BlsScalar>"]
    pub existing_nullifiers: Vec<u8>,
    #[doc = " notes we want to check the nullifiers of as a Vec<Note>"]
    pub notes: Vec<u8>,
    #[doc = " The seed to generate the view keys from"]
    pub seed: Vec<u8>,
}
#[doc = " Arguments for get_allow_call_data function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetAllowCallDataArgs {
    #[doc = " Counter value from stakeinfo"]
    pub counter: u64,
    #[doc = " gas_limit"]
    pub gas_limit: u64,
    #[doc = " gas_price"]
    pub gas_price: u64,
    #[doc = " index of the owner of the stake"]
    pub owner_index: u64,
    #[doc = " pk in string of who to refund this tx to"]
    pub refund: String,
    #[doc = " random rng seed"]
    pub rng_seed: Vec<u8>,
    #[doc = " Seed of the wallet"]
    pub seed: Vec<u8>,
    #[doc = " index of the sender of the tx"]
    pub sender_index: u64,
}
#[doc = " Response of the get_allow_call_data function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetAllowCallDataResponse {
    #[doc = " Blinder used to make the crossover"]
    pub blinder: Vec<u8>,
    #[doc = " The id of the contract to call in Base58 format"]
    pub contract: String,
    #[doc = " Crossover of this tx"]
    pub crossover: Vec<u8>,
    #[doc = " The fee of the tx"]
    pub fee: Vec<u8>,
    #[doc = " The name of the method to be called"]
    pub method: String,
    #[doc = " The payload of the call"]
    pub payload: Vec<u8>,
}
#[doc = " arguments of the get_history function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetHistoryArgs {
    #[doc = " index of the key the notes belong to"]
    pub index: u64,
    #[doc = " The notes of the wallet"]
    pub notes: Vec<NoteInfoType>,
    #[doc = " Seed of the wallet"]
    pub seed: Vec<u8>,
    #[doc = " The tx data of the wallet"]
    pub tx_data: Vec<TxsDataType>,
}
#[doc = " Response of the get_history function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetHistoryResponse {
    #[doc = " The history of a address"]
    pub history: Vec<TransactionHistoryType>,
}
#[doc = " Retrieve the seed bytes from the mnemonic and passphrase"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetMnemonicSeedArgs {
    #[doc = " The mnemonic string"]
    pub mnemonic: String,
    #[doc = " The passphrase tied to that mnemonic"]
    pub passphrase: String,
}
#[doc = " Response of the get_mnemonic_seed function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetMnemonicSeedResponse {
    #[doc = " Seed bytes from the given passphrase and Mnemonic"]
    pub mnemonic_seed: Vec<u8>,
}
#[doc = " Get the call data for stakeing"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStakeCallDataArgs {
    #[doc = " The stake counter value"]
    pub counter: u64,
    #[doc = " The stct proof as recieved from the node"]
    pub proof: Vec<u8>,
    #[doc = " The seed to generate the sender keys from"]
    pub seed: Vec<u8>,
    #[doc = " Index of the address of the staker in the seed"]
    pub staker_index: u64,
    #[doc = " The amount of value to stake"]
    pub value: u64,
}
#[doc = " Response of the get_stake_call_data function, send this to the call_data in execute"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStakeCallDataResponse {
    #[doc = " The contract to call encoded in bs58 format"]
    pub contract: String,
    #[doc = " The method to call on the contract"]
    pub method: String,
    #[doc = " The payload of the call"]
    pub payload: Vec<u8>,
}
#[doc = " Args of the get_stake_info function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStakeInfoArgs {
    #[doc = " The stake info of the stake obtained from the node"]
    pub stake_info: Vec<u8>,
}
#[doc = " Response of the get_stake_info function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStakeInfoRespose {
    #[doc = " amount staked"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    #[doc = " Signature counter to prevent replay"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter: Option<u64>,
    #[doc = " eligiblity"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligiblity: Option<u64>,
    #[doc = " True if the key has been authorized to stake"]
    pub has_key: bool,
    #[doc = " Has the given address staked"]
    pub has_staked: bool,
    #[doc = " Reward for participating in concensus"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward: Option<u64>,
}
#[doc = " Args of the get_stake_pk_rkyv_serialized function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStakePKrkyvSerializedArgs {
    #[doc = " The index of the public key to get"]
    pub index: u64,
    #[doc = " The seed to generate the sender keys from"]
    pub seed: Vec<u8>,
}
#[doc = " Get the bytes for the stct proof to send to the node"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStctProofArgs {
    #[doc = " The gas limit of the transaction"]
    pub gas_limit: u64,
    #[doc = " The gas price of the transaction"]
    pub gas_price: u64,
    #[doc = " The refund address in base58 format"]
    pub refund: String,
    #[doc = " The rng seed to generate the entropy for the notes"]
    pub rng_seed: Vec<u8>,
    #[doc = " The seed to generate the sender keys from"]
    pub seed: Vec<u8>,
    #[doc = " index of the sender in the seed"]
    pub sender_index: u64,
    #[doc = " The amount of value to send"]
    pub value: u64,
}
#[doc = " Response of the get_stct_proof function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetStctProofResponse {
    #[doc = " The blinder of the stct proof"]
    pub blinder: Vec<u8>,
    #[doc = " The bytes of the stct proof to send to the node"]
    pub bytes: Vec<u8>,
    #[doc = " The crossover value of the stct proof"]
    pub crossover: Vec<u8>,
    #[doc = " The Fee of the crossover note"]
    pub fee: Vec<u8>,
    #[doc = " The signature of the stct proof"]
    pub signature: Vec<u8>,
}
#[doc = " Args of the get_unstake_call_data function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetUnstakeCallDataArgs {
    #[doc = " The counter of the unstake note"]
    pub counter: u64,
    #[doc = " The seed to generate the sender keys from"]
    pub seed: Vec<u8>,
    #[doc = " The index of the public key to get"]
    pub sender_index: u64,
    #[doc = " The unstake note"]
    pub unstake_note: Vec<u8>,
    #[doc = " The unstake proof"]
    pub unstake_proof: Vec<u8>,
}
#[doc = " Response of the get_wfct_proof function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct GetWfctProofResponse {
    #[doc = " JubJubScalar Blinder for tx"]
    pub blinder: Vec<u8>,
    #[doc = " The bytes of the wfct proof to send to the node"]
    pub bytes: Vec<u8>,
    #[doc = " Crossover of the tx"]
    pub crossover: Vec<u8>,
    #[doc = " The fee of the tx"]
    pub fee: Vec<u8>,
    #[doc = " The unstake note"]
    pub unstake_note: Vec<u8>,
}
#[doc = " The arguments of the merge_notes function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MergeNotesArgs {
    #[doc = " All serialized list of notes to be merged"]
    pub notes: Vec<Vec<u8>>,
}
#[doc = " The arguments of the mnemonic_new function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MnemonicNewArgs {
    #[doc = " Cryptographically secure [u8; 64]"]
    pub rng_seed: Vec<u8>,
}
#[doc = " Response of the new_mnemonic function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MnewmonicNewResponse {
    #[doc = " String from the generated mnemonic"]
    pub mnemonic_string: String,
}
#[doc = " Information about the note"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NoteInfoType {
    #[doc = " The block height of the note"]
    pub block_height: u64,
    #[doc = " Singular Note rkyv serialized"]
    pub note: Vec<u8>,
    #[doc = " Nullifier of a Singular Note rkyv serialized"]
    pub nullifier: Vec<u8>,
    #[doc = " public key belonging to that note"]
    pub pk: String,
    #[doc = " position of the note"]
    pub pos: u64,
}
#[doc = " The arguments of the nullifiers function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NullifiersArgs {
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to have nullifiers generated"]
    pub notes: Vec<u8>,
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " The type represents the Opening and the position of the note, the opening is of"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct OpeningType {
    #[doc = " The rkyv serialized opening"]
    pub opening: Vec<u8>,
    #[doc = " The position of the note the opening is of"]
    pub pos: u64,
}
#[doc = " A note type variant"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum OutputType {
    Transparent,
    Obfuscated,
}
#[doc = " Arguments of the prove_tx function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ProveTxArgs {
    #[doc = " The bytes of the proof of the tx"]
    pub proof: Vec<u8>,
    #[doc = " The unproven_tx bytes"]
    pub unproven_tx: Vec<u8>,
}
#[doc = " Response of the prove_tx function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ProveTxResponse {
    #[doc = " The bytes of the proven transaction ready to be sent to the node"]
    pub bytes: Vec<u8>,
    #[doc = " The hash of the proven transaction"]
    pub hash: String,
}
#[doc = " Type of the response of the check_note_validity function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PublicKeysAndNotesType {
    #[doc = " Array of notes which are rkyv serialized"]
    pub notes: Vec<u8>,
    #[doc = " The public key as a bs58 formated string"]
    pub public_key: String,
}
#[doc = " The arguments of the public_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PublicKeysArgs {
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " The response of the public_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PublicKeysResponse {
    #[doc = " The Base58 public keys of the wallet."]
    pub keys: Vec<String>,
}
#[doc = " Arguments of the rkyv_bls_scalar_array function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvBlsScalarArrayArgs {
    #[doc = " An array containing rkyv serialized bytes of each bls scalar"]
    pub bytes: Vec<Vec<u8>>,
}
#[doc = " The arguments of the rkyv_notes_array function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvNotesArray {
    #[doc = " Array of notes which are rkyv serialized"]
    pub notes: Vec<Vec<u8>>,
}
#[doc = " Arguments of the rkyv_openings_array function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvOpeningsArray {
    #[doc = " Vec containing the rkyv serialized bytes of each openings along with positions"]
    pub openings: Vec<OpeningType>,
}
#[doc = " The arguments of the balance function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvTreeLeaf {
    #[doc = " Bytes that are rkyv serialized into a phoenix_core::transaction::TreeLeaf"]
    pub bytes: Vec<u8>,
}
#[doc = " The arguments of the rkyv tree leaf function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvTreeLeafArgs {
    #[doc = " Bytes that are rkyv serialized into a phoenix_core::transaction::TreeLeaf"]
    pub bytes: Vec<u8>,
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " A serialized u64 using rkyv"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvU64 {
    #[doc = " A u64 rust string, representing a valid rust u64 (max: 18446744073709551615)"]
    pub value: u64,
}
#[doc = " The arguments of the seed function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct SeedArgs {
    #[doc = " An arbitrary sequence of bytes used to generate a secure seed"]
    pub passphrase: Vec<u8>,
}
#[doc = " The direction of the transaction"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum TransactionDirectionType {
    In,
    Out,
}
#[doc = " The type of the transaction history"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct TransactionHistoryType {
    #[doc = " The amount of the transaction"]
    pub amount: f64,
    #[doc = " The block height of the transaction"]
    pub block_height: u64,
    #[doc = " The direction of the transaction, in or out"]
    pub direction: TransactionDirectionType,
    #[doc = " The fee of the transaction"]
    pub fee: u64,
    #[doc = " The hash of the transaction"]
    pub id: String,
    #[doc = " The type of the transaction"]
    pub tx_type: String,
}
#[doc = " Metadata of the transaction, used in calculating history"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct TxDataType {
    #[doc = " The amount of gas spent in the transaction"]
    pub gas_spent: u64,
    #[doc = " The raw transaction bytes"]
    pub raw_tx: String,
}
#[doc = " Collection of transactions at a given block height"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct TxsDataType {
    #[doc = " The block height of the transactions"]
    pub block_height: u64,
    #[doc = " The transactions at the given block height"]
    pub txs: Vec<TxDataType>,
}
#[doc = " Arguments of the unproven_tx_to_bytes_response"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct UnprovenTxToBytesResponse {
    #[doc = " Serialied unproven_Tx ready to be sent to the network"]
    pub serialized: Vec<u8>,
}
#[doc = " Arguments of the unspent spent notes response"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct UnpsentSpentNotesResponse {
    #[doc = " The notes which are spent"]
    pub spent_notes: Vec<NoteInfoType>,
    #[doc = " The notes which are not spent yet"]
    pub unspent_notes: Vec<NoteInfoType>,
}
#[doc = " Arguents of the unspent_spent_notes function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct UnspentSpentNotesArgs {
    #[doc = " The Array<Number> of block heights of thte notes in the same order as the notes"]
    pub block_heights: Vec<f64>,
    #[doc = " The UInt8Array of rkyv serialized nullifiers recieved from the node"]
    pub existing_nullifiers: Vec<u8>,
    #[doc = " The Array<UInt8Array> of rkyv serialized notes"]
    pub notes: Vec<Vec<u8>>,
    #[doc = " The Array<UInt8Array> of rkyv serialized nullifiers of the note in the same order as the "]
    #[doc = " notes"]
    pub nullifiers_of_notes: Vec<Vec<u8>>,
    #[doc = " Array of bs58 encoded string to be sent with the response of the function"]
    pub pks: Vec<String>,
}
#[doc = " The arguments of the view_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ViewKeysArgs {
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
