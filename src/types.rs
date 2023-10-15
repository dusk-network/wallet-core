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
#[doc = " The arguments of the execute function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteArgs {
    #[doc = " A call to a contract method"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call: Option<ExecuteCall>,
    #[doc = " The [phoenix_core::Crossover] value"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crossover: Option<u64>,
    #[doc = " The gas limit of the transaction"]
    pub gas_limit: u64,
    #[doc = " The gas price per unit for the transaction"]
    pub gas_price: u64,
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to be used as inputs"]
    pub inputs: Vec<u8>,
    #[doc = " A rkyv serialized [Vec<tx::Opening>] to open the inputs to a Merkle root"]
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
#[doc = " The response of the execute function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ExecuteResponse {
    #[doc = " A rkyv serialized [crate::tx::UnspentTransaction]"]
    pub tx: Vec<u8>,
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] containing the notes that weren't used"]
    pub unspent: Vec<u8>,
}
#[doc = " The arguments of the filter_notes function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FilterNotesArgs {
    #[doc = " Boolean flags to be negative filtered"]
    pub flags: Vec<bool>,
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to be filtered"]
    pub notes: Vec<u8>,
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
#[doc = " The arguments of the nullifiers function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NullifiersArgs {
    #[doc = " A rkyv serialized [Vec<phoenix_core::Note>] to have nullifiers generated"]
    pub notes: Vec<u8>,
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " A note type variant"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum OutputType {
    Transparent,
    Obfuscated,
}
#[doc = " The arguments of the public_spend_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PublicSpendKeysArgs {
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
#[doc = " The response of the public_spend_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PublicSpendKeysResponse {
    #[doc = " The Base58 public spend keys of the wallet."]
    pub keys: Vec<String>,
}
#[doc = " The arguments of the rkyv_notes_array function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvNotesArray {
    #[doc = " Array of notes which are rkyv serialized"]
    pub notes: Vec<Vec<u8>>,
}
#[doc = " The arguments of the balance function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvTreeLeaf {
    #[doc = " Bytes that are rkyv serialized into a phoenix_core::transaction::TreeLeaf"]
    pub bytes: Vec<u8>,
}
#[doc = " The response of the public_spend_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RkyvTreeLeafResponse {
    #[doc = " The block height of the note."]
    pub block_height: u64,
    #[doc = " Bytes of note at the block_height"]
    pub note: Vec<u8>,
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
#[doc = " The arguments of the view_keys function"]
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ViewKeysArgs {
    #[doc = " Seed used to derive the keys of the wallet"]
    pub seed: Vec<u8>,
}
