import fs from "fs";
import {
    getPublicKey
  } from "../lib/wasm/exports.js";

const Module = {};
const wasmSource = fs.readFileSync(new URL("../mod.wasm", import.meta.url));

const imports = {
    env: {
      async fetch_anchor(anchorBuffer) {
        const ANCHOR_BUFFER_SIZE = 32;
        const { memory } = Module.exports;
        const anchor = new Uint8Array(memory.buffer, anchorBuffer, ANCHOR_BUFFER_SIZE);
        const client = new StateClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.GetAnchorRequest();
  
        await client.getAnchor(request, {}, (error, response) => {
          if (error) {
            console.error("Error in fetch_anchor", error);
            return 1; // Returning non-zero indicates an error.
          } else {
            console.log("response from client.getAnchor", response);
            anchor.set(response.anchor);
            return 0;
          }
        });
  
        return retCode;
      },
  
      fill_random(bufferPointer, bufferLength) {
        const { memory } = Module.exports;
        const bytes = new Uint8Array(memory.buffer, bufferPointer, bufferLength);
        crypto.getRandomValues(bytes);
        return 0;
      },
  
      // FIXME a value of zero entails that someone is only able to stake and
      //  withdraw only once for a particular key. This should be fixed once
      //  there is a way to query the node for the current block height.
      fetch_block_height(height) {
        console.log(height);
        return 0;
      },
  
      async fetch_existing_nullifiers(
        nullifiers,
        nullifiersLength,
        existingNullifiers,
        existingNullifiersLength,
      ) {
        const NULLIFIERS_BUFFER_SIZE = 4;
        const NULLIFIER_SIZE = 32;
        const NULLIFIERS_VIEW_SIZE = 4;
        const client = new StateClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.FindExistingNullifiersRequest();
  
        let { memory } = Module.exports;
        let nullifiersBuffer = new Uint8Array(memory.buffer, nullifiers, nullifiersLength);
        let existingNullifiersBuffer = new Uint8Array(
          memory.buffer,
          existingNullifiers,
          NULLIFIERS_BUFFER_SIZE,
        );
        let nullifiersLengthView = new DataView(
          memory.buffer,
          nullifiersLength,
          NULLIFIERS_VIEW_SIZE,
        );
        let nullifiersLengthViewLength = nullifiersLengthView.getUint32(0, true);
  
        const nullifiersList = new Array(nullifiersLengthViewLength);
  
        for (let i = 0; i < nullifiersLengthViewLength; i++) {
          nullifiersList.push(
            nullifiersBuffer.subarray(i * NULLIFIER_SIZE, (i + 1) * NULLIFIER_SIZE),
          );
        }
  
        request.setNullifiersList(nullifiersList);
  
        await client.FindExistingNullifiers(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.fetch_existing_nullifiers", error);
            return 1;
          } else {
            const nullifiers = response.nullifiers;
  
            nullifiers.forEach((nullifier, i) => {
              existingNullifiersBuffer.set(nullifier, i * NULLIFIER_SIZE);
            });
  
            const existingNullifiersLengthView = new DataView(
              existingNullifiersLength,
              0,
              NULLIFIERS_VIEW_SIZE,
            );
            existingNullifiersLengthView.setUint32(0, nullifiers.length, true);
  
            return 0;
          }
        });
      },
  
      async fetch_opening(notePointer, openingPointer, openingLengthPointer) {
        const OPENING_SIZE = 0x10000;
        const OPENING_VIEW_SIZE = 4;
        const { memory } = Module.exports;
        const note = new Uint8Array(memory.buffer, notePointer, NOTE_SIZE);
        const openingBuffer = new Uint8Array(memory.buffer, openingPointer, OPENING_SIZE);
        const client = new StateClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.GetOpeningRequest();
  
        request.setNote(note);
  
        await client.getOpening(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.getOpening", error);
            return 1;
          } else {
            console.log("response from client.getOpening", response);
  
            const opening = response.opening;
  
            openingBuffer.set(opening);
            const openingLengthView = new DataView(
              memory.buffer,
              openingLengthPointer,
              OPENING_VIEW_SIZE,
            );
            openingLengthView.setUint32(opening.length, 0, true);
  
            return 0;
          }
        });
      },
  
      get_seed(seedPointer) {
        const { memory } = Module.exports;
        const seedBuffer = new Uint8Array(memory.buffer, seedPointer, KEY_SIZE);
  
        if (seed) {
          console.log("getseed", seed);
          seedBuffer.set(seed);
          return 0;
        } else {
          console.error("error in get_seed, seed is a falsy");
          return 1;
        }
      },
  
      async fetch_notes(height, viewKeyPointer, notesPointer, notesLengthPointer) {
        const NOTES_VIEW_SIZE = 4;
        const NOTES_BUFFER_SIZE = 0x100000;
        const { memory } = Module.exports;
        const viewKey = new Uint8Array(memory.buffer, viewKeyPointer, KEY_SIZE);
        const client = new StateClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.GetNotesOwnedByRequest();
        const notesBuffer = new Uint8Array(memory.buffer, notesPointer, NOTES_BUFFER_SIZE);
        // const notesLengthBuffer = new Uint8Array(memory.buffer, notes_len_ptr, NOTES_BYTE_SIZE);
  
        request.setVk(viewKey).setHeight(Number(height));
  
        await client.getNotesOwnedBy(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.getNotesOwnedBy", error);
            return 1;
          } else {
            console.log("response from client.getNotesOwnedBy", response);
            const notesList = response.getNotesList();
  
            notesList.forEach((note, i) => {
              notesBuffer.set(note, i * NOTE_SIZE);
            });
  
            const notesLength = new DataView(memory.buffer, notesLengthPointer, NOTES_VIEW_SIZE);
            notesLength.setUint32(0, notesList.length, true);
  
            return 0;
          }
        });
      },
  
      async compute_proof_and_propagate(unprovenTransactionPointer, unprovenTransactionLength) {
        const { memory } = Module.exports;
        const unprovenTransaction = new Uint8Array(
          memory.buffer,
          unprovenTransactionPointer,
          unprovenTransactionLength,
        );
        const proverClient = new ProverClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.ExecuteProverRequest();
  
        request.setUtx(unprovenTransaction);
  
        let proverClientResponseSuccess = false;
        await proverClient.proveExecute(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.proveExecute", error);
            return 1;
          } else {
            console.log("response from client.proveExecute", response);
            unprovenTransaction.set(response);
            proverClientResponseSuccess = true;
          }
        });
  
        const networkClient = new NetworkClient(NODES_PROXY_URL, 0, 0);
        const message = new proto.rusk.PropagateMessage(unprovenTransaction);
  
        let networkClientResponseSuccess = false;
        await networkClient.propagate(message, {}, (error, response) => {
          if (error) {
            console.error("error in client.propagate", error);
            return 1;
          } else {
            console.log("response from client.propagate", response);
            networkClientResponseSuccess = true;
          }
        });
  
        return proverClientResponseSuccess && networkClientResponseSuccess ? 0 : 1;
      },
  
      async fetch_stake(payKeyPointer, stakePointer) {
        const PAY_KEY_SIZE = 32;
        const STAKE_SIZE = 8;
        const { memory } = Module.exports;
        const payKey = new Uint8Array(memory.buffer, payKeyPointer, PAY_KEY_SIZE);
        const stake = new Uint8Array(memory.buffer, stakePointer, STAKE_SIZE);
        const client = new StateClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.GetStakeRequest();
  
        request.setPk(payKey);
  
        await client.GetStake(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.stake", error);
            return 1;
          } else {
            console.log("response from client.stake", response);
            //var buffer = new ArrayBuffer(8);
            //var z = new Uint8Array(buffer, 1, 4);
            //put response array-buffer -> little endian into u8 array
            stake.set(getUint64BigInt(new DataView(memory.buffer), response.stake));
            payKey.set(getUint32BigInt(new DataView(memory.buffer), response.expiration));
            return 0;
          }
        });
      },
  
      async request_stct_proof(circuitInputsPointer, proofPointer) {
        const CIRCUIT_INPUTS_SIZE = 376;
        const { memory } = Module.exports;
        const circuitInputs = new Uint8Array(
          memory.buffer,
          circuitInputsPointer,
          CIRCUIT_INPUTS_SIZE,
        );
        const proof = new Uint8Array(memory.buffer, proofPointer, PROOF_SIZE);
        const client = new ProverClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.StctProverRequest();
  
        request.setCircuitInputs(circuitInputs);
  
        await client.proveStct(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.proveStct", error);
            return 1;
          } else {
            console.log("response from client.proveStct", response);
            proof.set(response);
            return 0;
          }
        });
      },
  
      async request_wfct_proof(circuitInputsPointer, proofPointer) {
        const CIRCUIT_INPUTS_SIZE = 72;
        const PROOF_SIZE = 1488;
        const { memory } = Module.exports;
        const circuitInputs = new Uint8Array(
          memory.buffer,
          circuitInputsPointer,
          CIRCUIT_INPUTS_SIZE,
        );
        const proof = new Uint8Array(memory.buffer, proofPointer, PROOF_SIZE);
        const client = new ProverClient(NODES_PROXY_URL, 0, 0);
        const request = new proto.rusk.WfctProverRequest();
  
        request.setCircuitInputs(circuitInputs);
  
        await client.proveWfct(request, {}, (error, response) => {
          if (error) {
            console.error("error in client.proveWfct", error);
            return 1;
          } else {
            //console.log("response from client.proveWfct", response);
            proof.set(response);
            return 0;
          }
        });
      },
  
      sig(pointer, length) {
        const { memory } = Module.exports;
        const messageBuffer = new Uint8Array(memory.buffer, pointer, length);
        const message = new TextDecoder().decode(messageBuffer);
        console.error("WASM Error", message);
      },
    },
  };

const buffer = new Uint8Array(wasmSource);
async function initWasm() {
    const wa = await WebAssembly.instantiate(buffer, imports);
    Module.exports = wa.instance.exports;
}

await initWasm();

let {
    public_spend_key,
    memory,
    free,
    malloc
  } = Module.exports;

console.log(getPublicKey(public_spend_key, memory, free, malloc));