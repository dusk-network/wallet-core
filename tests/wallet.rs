// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use dusk_wallet_core::{tx, types, utils, MAX_LEN, RNG_SEED};
use rusk_abi::ContractId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasmer::{imports, Instance, Module, Store, Value};

#[test]
fn seed_works() {
    let mut wallet = Wallet::default();

    let seed = wallet.call("seed", json!({
        "passphrase": b"Taking a new step, uttering a new word, is what people fear most.".to_vec()
    })).take_memory();

    assert_eq!(seed.len(), RNG_SEED);
}

#[test]
fn balance_works() {
    let seed = [0xfa; RNG_SEED];
    let values = [10, 250, 15, 39, 55];
    let mut wallet = Wallet::default();

    let types::BalanceResponse { maximum, value } = wallet
        .call(
            "balance",
            json!({
                "notes": node::notes(&seed, values),
                "seed": seed.to_vec(),
            }),
        )
        .take_contents();

    assert_eq!(value, values.into_iter().sum::<u64>());
    assert_eq!(maximum, 359);
}

#[test]
fn public_spend_key_works() {
    let seed = [0xfa; RNG_SEED];

    let mut wallet = Wallet::default();

    let psk = wallet
        .call(
            "public_spend_key",
            json!({
                "seed": seed.to_vec(),
                "idx": 3
            }),
        )
        .take_memory();

    let psk = bs58::decode(psk).into_vec().unwrap();
    let mut psk_array = [0u8; PublicSpendKey::SIZE];

    psk_array.copy_from_slice(&psk);
    PublicSpendKey::from_bytes(&psk_array).unwrap();
}

#[test]
fn execute_works() {
    let seed = [0xfa; RNG_SEED];
    let rng_seed = [0xfb; RNG_SEED];
    let values = [10, 250, 15, 7500];

    let mut wallet = Wallet::default();

    let psk = wallet
        .call(
            "public_spend_key",
            json!({
                "seed": seed.to_vec(),
                "idx":5
            }),
        )
        .take_memory();
    let psk = String::from_utf8(psk).unwrap();

    let mut contract = ContractId::uninitialized();
    contract.as_bytes_mut().iter_mut().for_each(|b| *b = 0xfa);
    let contract = bs58::encode(contract.as_bytes()).into_string();

    let (inputs, openings) = node::notes_and_openings(&seed, values);
    let args = json!({
        "call": {
            "contract": contract,
            "method": "commit",
            "payload": b"We lost because we told ourselves we lost.".to_vec(),
        },
        "crossover": 25,
        "gas_limit": 100,
        "gas_price": 2,
        "inputs": inputs,
        "openings": openings,
        "output": {
            "note_type": "Transparent",
            "receiver": psk.clone(),
            "ref_id": 15,
            "value": 10,
        },
        "refund": psk,
        "rng_seed": rng_seed.to_vec(),
        "seed": seed.to_vec()
    });
    let types::ExecuteResponse { tx, unspent } =
        wallet.call("execute", args).take_contents();

    rkyv::from_bytes::<tx::UnprovenTransaction>(&tx).unwrap();
    rkyv::from_bytes::<Vec<phoenix_core::Note>>(&unspent).unwrap();
}

#[test]
fn merge_notes_works() {
    let seed = [0xfa; RNG_SEED];

    let notes1 = node::raw_notes(&seed, [10, 250, 15, 39, 55]);
    let notes2 = vec![notes1[1].clone(), notes1[3].clone()];
    let notes3: Vec<_> = node::raw_notes(&seed, [10, 250, 15, 39, 55])
        .into_iter()
        .chain([notes1[4].clone()])
        .collect();

    let notes_unmerged: Vec<_> = notes1
        .iter()
        .chain(notes2.iter())
        .chain(notes3.iter())
        .cloned()
        .collect();

    let mut notes_merged = notes_unmerged.clone();
    notes_merged.sort_by_key(|n| n.hash());
    notes_merged.dedup();

    assert_ne!(notes_unmerged, notes_merged);

    let notes1 = rkyv::to_bytes::<_, MAX_LEN>(&notes1).unwrap().into_vec();
    let notes2 = rkyv::to_bytes::<_, MAX_LEN>(&notes2).unwrap().into_vec();
    let notes3 = rkyv::to_bytes::<_, MAX_LEN>(&notes3).unwrap().into_vec();
    let notes = vec![notes1, notes2, notes3];

    let mut wallet = Wallet::default();

    let notes = wallet
        .call("merge_notes", json!({ "notes": notes }))
        .take_memory();

    let notes = rkyv::from_bytes::<Vec<phoenix_core::Note>>(&notes).unwrap();

    assert_eq!(notes, notes_merged);
}

#[test]
fn filter_notes_works() {
    let seed = [0xfa; RNG_SEED];

    let notes = node::raw_notes(&seed, [10, 250, 15, 39, 55]);
    let flags = vec![true, true, false, true, false];
    let filtered = vec![notes[2].clone(), notes[4].clone()];
    let filtered = utils::sanitize_notes(filtered);

    let notes = rkyv::to_bytes::<_, MAX_LEN>(&notes).unwrap().into_vec();

    let mut wallet = Wallet::default();

    let notes = wallet
        .call("filter_notes", json!({ "flags": flags, "notes": notes }))
        .take_memory();

    let notes = rkyv::from_bytes::<Vec<phoenix_core::Note>>(&notes).unwrap();

    assert_eq!(notes, filtered);
}

#[test]
fn view_keys_works() {
    let seed = [0xfa; RNG_SEED];

    let mut wallet = Wallet::default();

    let vk = wallet
        .call(
            "view_keys",
            json!({
                "seed": seed.to_vec()
            }),
        )
        .take_memory();

    rkyv::from_bytes::<Vec<dusk_pki::ViewKey>>(&vk).unwrap();
}

#[test]
fn nullifiers_works() {
    let seed = [0xfa; RNG_SEED];

    let (notes, nullifiers): (Vec<_>, Vec<_>) =
        node::raw_notes_and_nulifiers(&seed, [10, 250, 15, 39, 55])
            .into_iter()
            .unzip();

    let notes = rkyv::to_bytes::<_, MAX_LEN>(&notes).unwrap().into_vec();

    let mut wallet = Wallet::default();

    let response = wallet
        .call(
            "nullifiers",
            json!({
                "seed": seed.to_vec(),
                "notes": notes
            }),
        )
        .take_memory();

    let response =
        rkyv::from_bytes::<Vec<dusk_jubjub::BlsScalar>>(&response).unwrap();

    assert_eq!(nullifiers, response);
}

/// A node interface. It will encapsulate all the phoenix core functionality.
mod node {
    use core::mem;

    use dusk_jubjub::{BlsScalar, JubJubScalar};
    use dusk_wallet_core::{key, tx, utils, MAX_KEY, MAX_LEN, RNG_SEED};
    use phoenix_core::Note;
    use rand::RngCore;

    pub fn raw_notes<Values>(seed: &[u8; RNG_SEED], values: Values) -> Vec<Note>
    where
        Values: IntoIterator<Item = u64>,
    {
        let rng = &mut utils::rng(seed);
        values
            .into_iter()
            .map(|value| {
                let obfuscated = (rng.next_u32() & 1) == 1;
                let idx = rng.next_u64() % MAX_KEY as u64;
                let psk = key::derive_ssk(seed, idx).public_spend_key();

                if obfuscated {
                    let blinder = JubJubScalar::random(rng);
                    Note::obfuscated(rng, &psk, value, blinder)
                } else {
                    Note::transparent(rng, &psk, value)
                }
            })
            .collect()
    }

    pub fn notes<Values>(seed: &[u8; RNG_SEED], values: Values) -> Vec<u8>
    where
        Values: IntoIterator<Item = u64>,
    {
        rkyv::to_bytes::<_, MAX_LEN>(&raw_notes(seed, values))
            .expect("failed to serialize notes")
            .into_vec()
    }

    pub fn notes_and_openings<Values>(
        seed: &[u8; RNG_SEED],
        values: Values,
    ) -> (Vec<u8>, Vec<u8>)
    where
        Values: IntoIterator<Item = u64>,
    {
        let values: Vec<_> = values.into_iter().collect();
        let len = values.len();
        let notes = notes(seed, values);
        let openings: Vec<_> = (0..len)
            .map(|_| unsafe { mem::zeroed::<tx::Opening>() })
            .collect();

        let openings = rkyv::to_bytes::<_, MAX_LEN>(&openings)
            .expect("failed to serialize openings")
            .into_vec();

        (notes, openings)
    }

    pub fn raw_notes_and_nulifiers<Values>(
        seed: &[u8; RNG_SEED],
        values: Values,
    ) -> Vec<(Note, BlsScalar)>
    where
        Values: IntoIterator<Item = u64>,
    {
        let rng = &mut utils::rng(seed);
        values
            .into_iter()
            .map(|value| {
                let obfuscated = (rng.next_u32() & 1) == 1;
                let idx = rng.next_u64() % MAX_KEY as u64;
                let ssk = key::derive_ssk(seed, idx);
                let psk = ssk.public_spend_key();

                let note = if obfuscated {
                    let blinder = JubJubScalar::random(rng);
                    Note::obfuscated(rng, &psk, value, blinder)
                } else {
                    Note::transparent(rng, &psk, value)
                };

                let nullifier = note.gen_nullifier(&ssk);
                (note, nullifier)
            })
            .collect()
    }
}

pub struct Wallet {
    pub store: Store,
    pub module: Module,
    pub instance: Instance,
}

pub struct CallResult<'a> {
    pub status: bool,
    pub val: u64,
    pub aux: u64,
    pub wallet: &'a mut Wallet,
}

impl<'a> CallResult<'a> {
    pub fn new(wallet: &'a mut Wallet, value: i64) -> Self {
        let (status, val, aux) = utils::decompose(value);
        Self {
            status,
            val,
            aux,
            wallet,
        }
    }

    pub fn take_memory(self) -> Vec<u8> {
        assert!(self.status);

        let mut bytes = vec![0u8; self.aux as usize];

        self.wallet
            .instance
            .exports
            .get_memory("memory")
            .unwrap()
            .view(&self.wallet.store)
            .read(self.val, &mut bytes)
            .unwrap();

        self.wallet
            .instance
            .exports
            .get_function("free_mem")
            .unwrap()
            .call(
                &mut self.wallet.store,
                &[Value::I32(self.val as i32), Value::I32(self.aux as i32)],
            )
            .unwrap();

        bytes
    }

    pub fn take_contents<T>(self) -> T
    where
        T: for<'b> Deserialize<'b>,
    {
        assert!(self.status);
        let bytes = self.take_memory();
        let json = String::from_utf8(bytes).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    pub fn take_val(self) -> u64 {
        assert!(self.status);
        self.val
    }
}

impl Wallet {
    pub fn call<T>(&mut self, f: &str, args: T) -> CallResult
    where
        T: Serialize,
    {
        let bytes = serde_json::to_string(&args).unwrap();
        let len = Value::I32(bytes.len() as i32);
        let ptr = self
            .instance
            .exports
            .get_function("malloc")
            .unwrap()
            .call(&mut self.store, &[len.clone()])
            .unwrap()[0]
            .unwrap_i32();

        self.instance
            .exports
            .get_memory("memory")
            .unwrap()
            .view(&self.store)
            .write(ptr as u64, bytes.as_bytes())
            .unwrap();

        let ptr = Value::I32(ptr);
        let result = self
            .instance
            .exports
            .get_function(f)
            .unwrap()
            .call(&mut self.store, &[ptr, len])
            .unwrap()[0]
            .unwrap_i64();

        CallResult::new(self, result)
    }
}

impl Default for Wallet {
    fn default() -> Self {
        const WALLET: &[u8] = include_bytes!(
            "../target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm"
        );

        let mut store = Store::default();
        let module =
            Module::new(&store, WALLET).expect("failed to create wasm module");

        let import_object = imports! {};
        let instance = Instance::new(&mut store, &module, &import_object)
            .expect("failed to instanciate the wasm module");

        Self {
            store,
            module,
            instance,
        }
    }
}
