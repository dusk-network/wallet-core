import { u32, u64, Buffer64, BoxedBuffer } from "../wasm/alloc/types.mjs";
import * as crypto from "node:crypto";
import { Env } from "../wallet.mjs";
import Base58 from "../b58.mjs";
import * as expected from "./data.mjs";

const ViewKey = Buffer64;

export class TestEnv extends Env {
  constructor(assert) {
    super();
    this.assert = assert;
  }

  sig = (ptr, len) => {
    const { memory } = this;
    const messageBuffer = new Uint8Array(memory.buffer, ptr, len);
    const message = new TextDecoder().decode(messageBuffer);
    console.error("WASM Error:", message);
  };

  fetch_anchor = (anchor_buf) => {};

  fill_random = (ptr, len) => {
    let { memory } = this;
    let bytes = new Uint8Array(memory.buffer, ptr, len);

    crypto.getRandomValues(bytes);
  };

  fetch_existing_nullifiers = (
    nullifiers,
    nullifiers_len,
    existing_nullifiers,
    existing_nullifiers_len
  ) => {};

  fetch_opening = (note_ptr, opening_ptr, opening_len_ptr) => {};

  get_seed = (ptr) => {
    let { memory } = this;
    let { value } = this.seed;

    new Uint8Array(memory.buffer).set(value, ptr);
  };

  fetch_notes = async (vk_ptr, notes_ptr, notes_len_ptr) => {
    await new Promise((r) => setTimeout(r, 2000));
    const { assert } = this;

    let vk = ViewKey.from(vk_ptr);
    // get the pointer where to store the notes' pointer
    let ptr = u32.from(notes_ptr);
    // get the pointer where to store the notes' length
    let len = u64.from(notes_len_ptr);

    let encodedViewKey = Base58.encode(vk.value);

    assert.ok(encodedViewKey in expected.notes, "ViewKey matches");

    let notes = expected.notes[encodedViewKey];

    // Allocate enough space for the notes and write them in memory
    let value = Uint8Array.from(notes.flat());
    let size = value.length;
    let buffer = new BoxedBuffer({ size, value });

    // Store the notes' pointer
    ptr.value = buffer.ptr;
    // Store the notes' length
    len.value = size;
    buffer.forget();
  };

  compute_proof_and_propagate = (utx_ptr, utx_len) => {};

  fetch_stake = (pk_ptr, stake_ptr, expiration_ptr) => {};

  request_stct_proof = (circuit_inputs_ptr, proof_ptr) => {};

  request_wfct_proof = (circuit_inputs_ptr, proof_ptr) => {};
}
