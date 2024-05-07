// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

use dusk_bytes::Serializable;
use dusk_jubjub::JubJubScalar;
use dusk_pki::PublicSpendKey;
use dusk_wallet_core::{
    tx,
    types::{self, CrossoverType as WasmCrossover},
    utils, MAX_KEY, MAX_LEN, RNG_SEED,
};
use phoenix_core::Crossover;

use rusk_abi::ContractId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasmtime::{Engine, Instance, Module, Store, Val};

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
                "notes": node::notes(&seed, values).0,
                "seed": seed.to_vec(),
            }),
        )
        .take_contents();

    assert_eq!(value, values.into_iter().sum::<u64>());
    assert_eq!(maximum, 359);
}

#[test]
fn execute_works() {
    let seed = [0xfa; RNG_SEED];
    let rng_seed = [0xfb; 32];
    let values = [10, 250, 15, 7500];

    let mut wallet = Wallet::default();

    let types::PublicSpendKeysResponse { keys } = wallet
        .call(
            "public_spend_keys",
            json!({
                "seed": seed.to_vec(),
            }),
        )
        .take_contents();
    let psk = &keys[0];

    let mut contract: ContractId = ContractId::uninitialized();
    contract.as_bytes_mut().iter_mut().for_each(|b| *b = 0xfa);
    let contract = bs58::encode(contract.as_bytes()).into_string();

    let (inputs, openings) = node::notes_and_openings(&seed, values);
    let crossover = Crossover::default();
    let blinder = JubJubScalar::default();

    let crossover = WasmCrossover {
        blinder: rkyv::to_bytes::<JubJubScalar, MAX_LEN>(&blinder)
            .unwrap()
            .to_vec(),
        crossover: rkyv::to_bytes::<Crossover, MAX_LEN>(&crossover)
            .unwrap()
            .to_vec(),
        value: 0,
    };

    let args = json!({
        "call": {
            "contract": contract,
            "method": "commit",
            "payload": b"We lost because we told ourselves we lost.".to_vec(),
        },
        "crossover": crossover,
        "gas_limit": 100,
        "gas_price": 2,
        "inputs": inputs,
        "sender_index": 0,
        "openings": openings,
        "output": {
            "note_type": "Obfuscated",
            "receiver": &keys[1],
            "ref_id": 15,
            "value": 10,
        },
        "refund": psk,
        "rng_seed": rng_seed.to_vec(),
        "seed": seed.to_vec()
    });

    let types::ExecuteResponse { tx } =
        wallet.call("execute", args).take_contents();

    rkyv::from_bytes::<tx::UnprovenTransaction>(&tx).unwrap();
}

#[test]
fn merge_notes_works() {
    let seed = [0xfa; RNG_SEED];

    let notes1 = node::raw_notes(&seed, [10, 250, 15, 39, 55]);
    let notes2 = vec![notes1[1], notes1[3]];
    let notes3: Vec<_> = node::raw_notes(&seed, [10, 250, 15, 39, 55])
        .into_iter()
        .chain([notes1[4]])
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
    let notes4 = vec![];
    let notes = vec![notes1, notes2, notes3, notes4];

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
    let filtered = vec![notes[2], notes[4]];
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
fn public_spend_keys_works() {
    let seed = [0xfa; RNG_SEED];

    let mut wallet = Wallet::default();

    let types::PublicSpendKeysResponse { keys } = wallet
        .call(
            "public_spend_keys",
            json!({
                "seed": seed.to_vec(),
            }),
        )
        .take_contents();

    for key in &keys {
        let key = bs58::decode(key).into_vec().unwrap();
        let mut key_array = [0u8; PublicSpendKey::SIZE];
        key_array.copy_from_slice(&key);
        PublicSpendKey::from_bytes(&key_array).unwrap();
    }

    assert_eq!(keys.len(), MAX_KEY + 1);
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
    use dusk_wallet_core::{key, tx, MAX_KEY, MAX_LEN, RNG_SEED};
    use phoenix_core::Note;
    use rand::{rngs::StdRng, RngCore};
    use rand_core::SeedableRng;

    pub fn raw_notes<Values>(seed: &[u8; RNG_SEED], values: Values) -> Vec<Note>
    where
        Values: IntoIterator<Item = u64>,
    {
        let rng = &mut StdRng::from_entropy();
        values
            .into_iter()
            .map(|value| {
                let obfuscated = (rng.next_u32() & 1) == 1;
                let psk = key::derive_ssk(seed, 0).public_spend_key();

                if obfuscated {
                    let blinder = JubJubScalar::random(rng);
                    Note::obfuscated(rng, &psk, value, blinder)
                } else {
                    Note::transparent(rng, &psk, value)
                }
            })
            .collect()
    }

    pub fn notes<Values>(
        seed: &[u8; RNG_SEED],
        values: Values,
    ) -> (Vec<u8>, Vec<u8>)
    where
        Values: IntoIterator<Item = u64>,
    {
        let notes = raw_notes(seed, values);
        let len = notes.len();

        let openings: Vec<_> = (0..len)
            .zip(notes.clone())
            .map(|(_, note)| {
                let opening = unsafe { mem::zeroed::<tx::Opening>() };
                (opening, *note.pos())
            })
            .collect();

        (
            rkyv::to_bytes::<_, MAX_LEN>(&notes)
                .expect("failed to serialize notes")
                .into_vec(),
            rkyv::to_bytes::<Vec<(tx::Opening, u64)>, MAX_LEN>(&openings)
                .expect("failed to serialize openings")
                .into_vec(),
        )
    }

    pub fn notes_and_openings<Values>(
        seed: &[u8; RNG_SEED],
        values: Values,
    ) -> (Vec<u8>, Vec<u8>)
    where
        Values: IntoIterator<Item = u64>,
    {
        let values: Vec<_> = values.into_iter().collect();

        let (notes, openings) = notes(seed, values);

        (notes, openings)
    }

    pub fn raw_notes_and_nulifiers<Values>(
        seed: &[u8; RNG_SEED],
        values: Values,
    ) -> Vec<(Note, BlsScalar)>
    where
        Values: IntoIterator<Item = u64>,
    {
        let rng = &mut StdRng::from_entropy();
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
    pub store: Store<()>,
    pub module: Module,
    pub instance: Instance,
}

pub struct CallResult<'a> {
    pub status: bool,
    pub val: u32,
    pub aux: u32,
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
            .get_memory(&mut self.wallet.store, "memory")
            .expect("There should be one memory")
            .read(&mut self.wallet.store, self.val as usize, &mut bytes)
            .unwrap();

        self.wallet
            .instance
            .get_func(&mut self.wallet.store, "free_mem")
            .expect("free_mem should exist")
            .call(
                &mut self.wallet.store,
                &[Val::I32(self.val as i32), Val::I32(self.aux as i32)],
                &mut [],
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

    pub fn take_val(self) -> u32 {
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

        let len_params = [Val::I32(bytes.len() as i32)];
        let mut ptr_results = [Val::I32(0)];

        let allocate = self
            .instance
            .get_func(&mut self.store, "allocate")
            .expect("allocate should exist");

        allocate
            .call(&mut self.store, &len_params, &mut ptr_results)
            .unwrap();

        self.instance
            .get_memory(&mut self.store, "memory")
            .expect("There should be one memory")
            .write(
                &mut self.store,
                ptr_results[0].unwrap_i32() as usize,
                bytes.as_bytes(),
            )
            .expect("Writing to memory should succeed");

        let params = [ptr_results[0].clone(), len_params[0].clone()];
        let mut results = [Val::I64(0)];

        self.instance
            .get_func(&mut self.store, f)
            .expect("allocate should exist")
            .call(&mut self.store, &params, &mut results)
            .unwrap();

        CallResult::new(self, results[0].unwrap_i64())
    }
}

impl Default for Wallet {
    fn default() -> Self {
        const WALLET: &[u8] = include_bytes!("../assets/dusk_wallet_core.wasm");

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());

        let module =
            Module::new(&engine, WALLET).expect("failed to create wasm module");

        let instance = Instance::new(&mut store, &module, &[])
            .expect("failed to instantiate the wasm module");

        Self {
            store,
            module,
            instance,
        }
    }
}
