// const NODES_PROXY_URL = "http://nodes.dusk.network:8585";
import Base58 from "./b58.mjs";
import * as Asyncify from "./wasm/asyncify.mjs";
import * as allocator from "./wasm/alloc/allocator.mjs";
import { u64, BoxedBuffer } from "./wasm/alloc/types.mjs";

const _claim = Symbol("Address::Claim");
const _seed = Symbol("Seed::Value");
const _builder = Symbol("Wallet::Builder");
const _memory = Symbol("Env::memory");
const _exports = Symbol("Wallet::Exports");

export class Env {
  #memory = null;
  #seed = null;

  set [_seed](value) {
    this.#seed = value;
  }

  set [_memory](value) {
    this.#memory = value;
  }

  get memory() {
    return this.#memory;
  }

  get seed() {
    return this.#seed;
  }
}

// impl<S: Store, SC: StateClient, PC: ProverClient> From<Error<S, SC, PC>>
//     for u8
// {
//     fn from(e: Error<S, SC, PC>) -> Self {
//         match e {
//             Error::Store(_) => 255,
//             Error::Rng(_) => 254,
//             Error::Bytes(_) => 253,
//             Error::State(_) => 252,
//             Error::Prover(_) => 251,
//             Error::NotEnoughBalance => 250,
//             Error::NoteCombinationProblem => 249,
//             Error::Canon(_) => 248,
//             Error::Phoenix(_) => 247,
//             Error::AlreadyStaked { .. } => 246,
//             Error::NotStaked { .. } => 245,
//             Error::NoReward { .. } => 244,
//         }
//     }
// }

const ASYNC_EXPORTS = ["get_balance"];

const PublicSpendKey = (value) => new BoxedBuffer({ size: 64, value });

export class BalanceInfo {
  // The total value of the balance.
  total;
  // The maximum _spendable_ value in a single transaction. This is
  // different from `value` since there is a maximum number of notes one can
  // spend.
  spendable;

  constructor({ total, spendable }) {
    this.total = total;
    this.spendable = spendable;
  }

  static parse(buffer) {
    if (
      typeof buffer !== "undefined" &&
      (!(buffer instanceof Uint8Array) || buffer.length !== 16)
    ) {
      throw new TypeError(`The buffer must be a Uint8Array of size 16`);
    }

    let view = new DataView(buffer.buffer, buffer.byteOffset);
    let total = view.getBigUint64(0, true);
    let spendable = view.getBigUint64(8, true);

    return new this({ total, spendable });
  }
}

export class Seed {
  #value = null;

  constructor(value) {
    if (typeof value === "number") {
      let buffer = new Uint8Array(64);
      let view = new DataView(buffer.buffer);

      view.setUint32(0, value, true);

      this.#value = buffer;
      return;
    }

    if (!(value instanceof Uint8Array) || value.length !== 64) {
      throw new TypeError("Seed must be a Uint8Array of size 64");
    }

    this.#value = Uint8Array.from(value);
  }

  get value() {
    return this.#value;
  }
}

export class Gas {
  limit = NaN;
  price = NaN;

  constructor({ limit = 500_000_000, price = 1 }) {
    this.limit = limit;
    this.price = price;

    Object.freeze(this);
  }
}

export class Address {
  #value = null;
  #wallet = null;
  #index = NaN;

  constructor(string) {
    if (!string) {
      return;
    }

    let value = Base58.decode(string);

    if (value.length !== 64) {
      throw new ReferenceError("Invalid Address");
    }

    this.#value = value;
  }

  [_claim](wallet, index, value) {
    this.#wallet = wallet;
    this.#index = index;

    if (value) {
      this.#value = value;
    }

    return this;
  }

  toJSON() {
    return Base58.encode(this.#value);
  }

  toString() {
    return this.toJSON();
  }

  get owned() {
    return this.#wallet instanceof Wallet && typeof this.#index === "number";
  }

  async balance() {
    let output = new BoxedBuffer({ size: 16 });
    await this.#wallet[_exports].get_balance(u64(this.#index), output);
    return BalanceInfo.parse(output.value);
  }

  transfer(receiver, amount, gas) {}

  stake(amount, gas) {}

  unstake(gas) {}

  withdraw(refund, gas) {}

  getStakeInfo() {}
}

export class Wallet {
  #seed = null;
  #addresses = [];

  static async build({ seed, env, source }) {
    let init = "";

    if (source instanceof Response) {
      init = "instantiateStreaming";
    } else if (Object.getPrototypeOf(source) instanceof Uint8Array) {
      init = "instantiate";
    } else {
      throw ReferenceError(
        "Source should be either a Response or a bytes buffer"
      );
    }

    if (!(seed instanceof Seed)) {
      throw new TypeError("seed must be an instance of `Seed`");
    }

    if (!(env instanceof Env)) {
      throw new TypeError("env must be an instance of `Env`");
    }

    const { instance } = await Asyncify[init](source, { env }, ASYNC_EXPORTS);

    const {
      public_spend_key,
      transfer,
      stake,
      unstake,
      withdraw,
      get_balance,
      get_stake,
    } = instance.exports;

    let wallet = new Wallet(_builder);

    wallet.#seed = seed;

    env[_memory] = allocator.init(instance).memory;
    env[_seed] = seed;

    wallet[_exports] = {
      public_spend_key,
      transfer,
      stake,
      unstake,
      withdraw,
      get_balance,
      get_stake,
    };

    return wallet;
  }

  constructor(builder) {
    if (builder !== _builder) {
      throw new TypeError("Wallet is not a constructor");
    }
  }

  get seed() {
    return this.#seed;
  }

  get addresses() {
    return [...this.#addresses];
  }

  claim(address) {
    let index = this.#addresses.findIndex(
      (addr) => addr.toString() === address.toString()
    );

    if (index > -1) {
      address[_claim](this, index);
    }
  }

  generateAddress() {
    let psk = PublicSpendKey();

    this[_exports].public_spend_key(u64(this.#addresses.length), psk);

    let address = new Address();
    address[_claim](this, this.#addresses.length, psk.value);

    this.#addresses.push(address);

    return address;
  }
}
