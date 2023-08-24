// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

use dusk_wallet_core::{
    tx, utils, BalanceArgs, BalanceResponse, ExecuteArgs, ExecuteResponse,
    FilterNotesArgs, FilterNotesResponse, MergeNotesArgs, MergeNotesResponse,
    NullifiersArgs, NullifiersResponse, ViewKeysArgs, ViewKeysResponse,
    MAX_LEN,
};
use std::collections::HashMap;
use wasmer::{imports, Function, Instance, Memory, Module, Store, Value};

#[test]
fn balance_works() {
    let seed = [0xfa; utils::RNG_SEED];
    let values = [10, 250, 15, 39, 55];

    let notes = node::notes(&seed, values);

    let args = BalanceArgs { seed, notes };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let mut wallet = Wallet::default();

    let len = Value::I32(args.len() as i32);
    let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

    wallet.memory_write(ptr, &args);

    let ptr = Value::I32(ptr as i32);
    let ptr = wallet.call("balance", &[ptr, len])[0].unwrap_i32() as u64;
    let balance = wallet.memory_read(ptr, BalanceResponse::LEN);
    let balance = rkyv::from_bytes::<BalanceResponse>(&balance)
        .expect("failed to deserialize balance");

    let ptr = Value::I32(ptr as i32);
    let len = Value::I32(BalanceResponse::LEN as i32);
    wallet.call("free_mem", &[ptr, len]);

    assert!(balance.success);
    assert_eq!(balance.value, values.into_iter().sum::<u64>());
    assert_eq!(balance.maximum, 359);
}

#[test]
fn execute_works() {
    let seed = [0xfa; utils::RNG_SEED];
    let rng_seed = [0xfb; utils::RNG_SEED];
    let values = [10, 250, 15, 7500];

    let (inputs, openings) = node::notes_and_openings(&seed, values);
    let refund = node::psk(&seed);
    let output = node::output(&seed, 133);
    let crossover = 35;
    let gas_limit = 100;
    let gas_price = 2;
    let call = node::empty_call_data();
    let args = ExecuteArgs {
        seed,
        rng_seed,
        inputs,
        openings,
        refund,
        output,
        crossover,
        gas_limit,
        gas_price,
        call,
    };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let mut wallet = Wallet::default();

    let len = Value::I32(args.len() as i32);
    let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

    wallet.memory_write(ptr, &args);

    let ptr = Value::I32(ptr as i32);
    let ptr = wallet.call("execute", &[ptr, len])[0].unwrap_i32() as u64;
    let execute = wallet.memory_read(ptr, ExecuteResponse::LEN);
    let execute = rkyv::from_bytes::<ExecuteResponse>(&execute)
        .expect("failed to deserialize execute");

    let ptr = Value::I32(ptr as i32);
    let len = Value::I32(ExecuteResponse::LEN as i32);
    wallet.call("free_mem", &[ptr, len]);

    let unspent =
        wallet.memory_read(execute.unspent_ptr, execute.unspent_len as usize);
    let _unspent: Vec<phoenix_core::Note> =
        rkyv::from_bytes(&unspent).expect("failed to deserialize notes");

    let ptr = Value::I32(execute.unspent_ptr as i32);
    let len = Value::I32(execute.unspent_len as i32);
    wallet.call("free_mem", &[ptr, len]);

    let tx = wallet.memory_read(execute.tx_ptr, execute.tx_len as usize);
    let _tx: tx::UnprovenTransaction =
        rkyv::from_bytes(&tx).expect("failed to deserialize tx");

    let ptr = Value::I32(execute.tx_ptr as i32);
    let len = Value::I32(execute.tx_len as i32);
    wallet.call("free_mem", &[ptr, len]);

    assert!(execute.success);
}

#[test]
fn merge_notes_works() {
    let seed = [0xfa; utils::RNG_SEED];

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

    let args = MergeNotesArgs { notes };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let mut wallet = Wallet::default();

    let len = Value::I32(args.len() as i32);
    let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

    wallet.memory_write(ptr, &args);

    let ptr = Value::I32(ptr as i32);
    let ptr = wallet.call("merge_notes", &[ptr, len])[0].unwrap_i32() as u64;

    let notes = wallet.memory_read(ptr, MergeNotesResponse::LEN);
    let notes = rkyv::from_bytes::<MergeNotesResponse>(&notes)
        .expect("failed to deserialize merged notes");

    let ptr = Value::I32(ptr as i32);
    let len = Value::I32(MergeNotesResponse::LEN as i32);
    wallet.call("free_mem", &[ptr, len]);

    let merged = wallet.memory_read(notes.notes_ptr, notes.notes_len as usize);
    let merged: Vec<phoenix_core::Note> =
        rkyv::from_bytes(&merged).expect("failed to deserialize notes");

    assert!(notes.success);
    assert_eq!(merged, notes_merged);
}

#[test]
fn filter_notes_works() {
    let seed = [0xfa; utils::RNG_SEED];

    let notes = node::raw_notes(&seed, [10, 250, 15, 39, 55]);
    let flags = vec![true, true, false, true, false];
    let filtered = vec![notes[2].clone(), notes[4].clone()];
    let filtered = utils::sanitize_notes(filtered);

    let notes = rkyv::to_bytes::<_, MAX_LEN>(&notes).unwrap().into_vec();
    let flags = rkyv::to_bytes::<_, MAX_LEN>(&flags).unwrap().into_vec();

    let args = FilterNotesArgs { notes, flags };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let mut wallet = Wallet::default();

    let len = Value::I32(args.len() as i32);
    let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

    wallet.memory_write(ptr, &args);

    let ptr = Value::I32(ptr as i32);
    let ptr = wallet.call("filter_notes", &[ptr, len])[0].unwrap_i32() as u64;

    let response = wallet.memory_read(ptr, FilterNotesResponse::LEN);
    let response = rkyv::from_bytes::<FilterNotesResponse>(&response)
        .expect("failed to deserialize filtered notes");

    assert!(response.success);

    let notes =
        wallet.memory_read(response.notes_ptr, response.notes_len as usize);
    let notes: Vec<phoenix_core::Note> =
        rkyv::from_bytes(&notes).expect("failed to deserialize notes");

    let ptr = Value::I32(ptr as i32);
    let len = Value::I32(FilterNotesResponse::LEN as i32);
    wallet.call("free_mem", &[ptr, len]);

    assert_eq!(notes, filtered);
}

#[test]
fn view_keys_works() {
    let seed = [0xfa; utils::RNG_SEED];

    let args = ViewKeysArgs { seed };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let keys = {
        let mut wallet = Wallet::default();

        let len = Value::I32(args.len() as i32);
        let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

        wallet.memory_write(ptr, &args);

        let ptr = Value::I32(ptr as i32);
        let ptr = wallet.call("view_keys", &[ptr, len])[0].unwrap_i32() as u64;

        let response = wallet.memory_read(ptr, ViewKeysResponse::LEN);
        let response = rkyv::from_bytes::<ViewKeysResponse>(&response)
            .expect("failed to deserialize view keys");

        assert!(response.success);

        let keys =
            wallet.memory_read(response.vks_ptr, response.vks_len as usize);
        let keys: Vec<dusk_pki::ViewKey> =
            rkyv::from_bytes(&keys).expect("failed to deserialize keys");
        keys
    };

    let keys_p = {
        let mut wallet = Wallet::default();

        let len = Value::I32(args.len() as i32);
        let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

        wallet.memory_write(ptr, &args);

        let ptr = Value::I32(ptr as i32);
        let ptr = wallet.call("view_keys", &[ptr, len])[0].unwrap_i32() as u64;

        let response = wallet.memory_read(ptr, ViewKeysResponse::LEN);
        let response = rkyv::from_bytes::<ViewKeysResponse>(&response)
            .expect("failed to deserialize view keys");

        assert!(response.success);

        let keys =
            wallet.memory_read(response.vks_ptr, response.vks_len as usize);
        let keys: Vec<dusk_pki::ViewKey> =
            rkyv::from_bytes(&keys).expect("failed to deserialize keys");
        keys
    };

    // assert keys generation is deterministic
    assert_eq!(keys, keys_p);
}

#[test]
fn nullifiers_works() {
    let seed = [0xfa; utils::RNG_SEED];

    let (notes, nullifiers): (Vec<_>, Vec<_>) =
        node::raw_notes_and_nulifiers(&seed, [10, 250, 15, 39, 55])
            .into_iter()
            .unzip();

    let notes = rkyv::to_bytes::<_, MAX_LEN>(&notes).unwrap().into_vec();
    let args = NullifiersArgs { seed, notes };
    let args =
        rkyv::to_bytes::<_, MAX_LEN>(&args).expect("failed to serialize args");

    let mut wallet = Wallet::default();

    let len = Value::I32(args.len() as i32);
    let ptr = wallet.call("malloc", &[len.clone()])[0].unwrap_i32() as u64;

    wallet.memory_write(ptr, &args);

    let ptr = Value::I32(ptr as i32);
    let ptr = wallet.call("nullifiers", &[ptr, len])[0].unwrap_i32() as u64;

    let response = wallet.memory_read(ptr, NullifiersResponse::LEN);
    let response = rkyv::from_bytes::<NullifiersResponse>(&response)
        .expect("failed to deserialize nullifiers");

    assert!(response.success);

    let response = wallet
        .memory_read(response.nullifiers_ptr, response.nullifiers_len as usize);
    let response: Vec<dusk_jubjub::BlsScalar> =
        rkyv::from_bytes(&response).expect("failed to deserialize nullifiers");

    assert_eq!(nullifiers, response);
}

/// A node interface. It will encapsulate all the phoenix core functionality.
mod node {
    use core::mem;

    use dusk_jubjub::{BlsScalar, JubJubScalar};
    use dusk_wallet_core::{key, tx, utils, MAX_KEY, MAX_LEN};
    use phoenix_core::{Note, NoteType};
    use rand::RngCore;

    pub fn raw_notes<Values>(
        seed: &[u8; utils::RNG_SEED],
        values: Values,
    ) -> Vec<Note>
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

    pub fn raw_notes_and_nulifiers<Values>(
        seed: &[u8; utils::RNG_SEED],
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

    pub fn notes<Values>(
        seed: &[u8; utils::RNG_SEED],
        values: Values,
    ) -> Vec<u8>
    where
        Values: IntoIterator<Item = u64>,
    {
        rkyv::to_bytes::<_, MAX_LEN>(&raw_notes(seed, values))
            .expect("failed to serialize notes")
            .into_vec()
    }

    pub fn notes_and_openings<Values>(
        seed: &[u8; utils::RNG_SEED],
        values: Values,
    ) -> (Vec<u8>, Vec<u8>)
    where
        Values: IntoIterator<Item = u64>,
    {
        let rng = &mut utils::rng(seed);
        let notes: Vec<_> = values
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
            .collect();

        let openings: Vec<_> = (0..notes.len())
            .map(|_| unsafe { mem::zeroed::<tx::Opening>() })
            .collect();

        let notes = rkyv::to_bytes::<_, MAX_LEN>(&notes)
            .expect("failed to serialize notes")
            .into_vec();

        let openings = rkyv::to_bytes::<_, MAX_LEN>(&openings)
            .expect("failed to serialize openings")
            .into_vec();

        (notes, openings)
    }

    pub fn psk(seed: &[u8; utils::RNG_SEED]) -> Vec<u8> {
        let psk = key::derive_ssk(seed, 0).public_spend_key();
        rkyv::to_bytes::<_, MAX_LEN>(&psk)
            .expect("failed to serialize psk")
            .into_vec()
    }

    pub fn output(seed: &[u8; utils::RNG_SEED], value: u64) -> Vec<u8> {
        let rng = &mut utils::rng(seed);
        let obfuscated = (rng.next_u32() & 1) == 1;
        let r#type = if obfuscated {
            NoteType::Obfuscated
        } else {
            NoteType::Transparent
        };
        let receiver = key::derive_ssk(seed, 1).public_spend_key();
        let ref_id = rng.next_u64();
        let output = Some(tx::OutputValue {
            r#type,
            value,
            receiver,
            ref_id,
        });

        rkyv::to_bytes::<_, MAX_LEN>(&output)
            .expect("failed to serialize notes")
            .into_vec()
    }

    pub fn empty_call_data() -> Vec<u8> {
        let call: Option<tx::CallData> = None;
        rkyv::to_bytes::<_, MAX_LEN>(&call)
            .expect("failed to serialize call data")
            .into_vec()
    }
}

pub struct Wallet {
    pub store: Store,
    pub module: Module,
    pub memory: Memory,
    pub f: HashMap<&'static str, Function>,
}

impl Wallet {
    pub fn call(&mut self, f: &str, args: &[Value]) -> Box<[Value]> {
        self.f[f]
            .call(&mut self.store, args)
            .expect("failed to call module function")
    }

    pub fn memory_write(&mut self, ptr: u64, data: &[u8]) {
        self.memory
            .view(&self.store)
            .write(ptr, data)
            .expect("failed to write memory");
    }

    pub fn memory_read(&self, ptr: u64, len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        self.memory
            .view(&self.store)
            .read(ptr, &mut bytes)
            .expect("failed to read memory");
        bytes
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

        let memory = instance
            .exports
            .get_memory("memory")
            .expect("failed to get instance memory")
            .clone();

        fn add_function(
            map: &mut HashMap<&'static str, Function>,
            instance: &Instance,
            name: &'static str,
        ) {
            map.insert(
                name,
                instance
                    .exports
                    .get_function(name)
                    .expect("failed to import wasm function")
                    .clone(),
            );
        }

        let mut f = HashMap::new();

        add_function(&mut f, &instance, "malloc");
        add_function(&mut f, &instance, "free_mem");
        add_function(&mut f, &instance, "balance");
        add_function(&mut f, &instance, "execute");
        add_function(&mut f, &instance, "merge_notes");
        add_function(&mut f, &instance, "filter_notes");
        add_function(&mut f, &instance, "view_keys");
        add_function(&mut f, &instance, "nullifiers");

        Self {
            store,
            module,
            memory,
            f,
        }
    }
}
