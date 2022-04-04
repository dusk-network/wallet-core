import { binary_to_base58 } from "base58-js";
// import { getUint64BigInt } from "./utils.js";

const SENDER_INDEX = 1n;

/**
 * Gets a public key.
 *
 * @param {*} public_spend_key
 * @param {*} memory
 * @param {*} free
 * @param {*} malloc
 * @returns
 */
export async function getPublicKey(public_spend_key, memory, free, malloc) {
  const pointer = await malloc(64);
  await public_spend_key(SENDER_INDEX, pointer);
  let buffer = new Uint8Array(memory.buffer, pointer, 64);
  buffer = binary_to_base58(buffer);
  await free(pointer, 64);
  return buffer;
}

export async function getBalance(get_balance, memory, malloc, free) {
  // TODO
}

export function handleTransfer(transfer, memory, malloc, free, event) {
  // TODO
}

export function toStake(stake, memory, free, malloc, event) {
  // TODO
}

export function extendStake(extend_stake, memory, free, malloc, event) {
  // TODO
}

export function withdrawStake(withdraw_stake, memory, malloc, free, event) {
  // TODO
}

export function getStake(get_stake, memory, malloc, free) {
  // TODO
}

export default {
  getPublicKey,
  getBalance,
  handleTransfer,
  getStake,
  withdrawStake,
  extendStake,
  toStake,
};
