import test from "tape";

import { Wallet, Seed, Address } from "../wallet.mjs";
import { TestEnv } from "./env.mjs";

import { readFile } from "node:fs/promises";

const source = await readFile("./wallet-core.wasm");

const seed = new Seed(0xdead);

test("A test", async (assert) => {
  const env = new TestEnv(assert);

  const wallet = await Wallet.build({
    seed,
    env,
    source,
  });

  assert.deepEqual(wallet.addresses, [], "No addresses");

  let addresses = [wallet.generateAddress(), wallet.generateAddress()];

  assert.deepEqual(
    wallet.addresses,
    addresses,
    "The addresses in the wallets are the same and in the correct order"
  );

  assert.deepEqual(
    JSON.parse(JSON.stringify(wallet.addresses)),
    [
      "5vwNiRgPehSux1668xAyTPEnsqpWhQzLQBXNboyKALojpaLQvma1sNxruDos7ay2ZAFjptu84jwhFPo6DpYAaFy1",
      "fhpxiYrXPi9jdYdNfA2WYsYpp3v6oNJnrnjnu3JpgkgnY7JEPYKcvvBZeGfixtJ8WDZG4o2s5Ks65V8CRKKub7g",
    ],
    "Addresses are the expected one and serialization works"
  );

  assert.ok(addresses[0].owned, "The first address is owned");
  assert.ok(addresses[1].owned, "The second address is owned");

  let address = new Address(addresses[0].toString());

  assert.notOk(address.owned, "An address generated by string is not owned");

  wallet.claim(address);

  assert.ok(
    address.owned,
    "An address generated by a string can be claimed by a wallet if it's listed"
  );

  address = new Address(
    "4RyaodGmN8MyUDmpRrtRxJJhrVW2HsY2ycRUnRUXR97JCN1GHraQT9Ygb8yYo7oKzyZg2EXXCGkHBwoeNb96BKtQ"
  );

  wallet.claim(address);

  assert.notOk(
    address.owned,
    "An address generated by a string cannot be claimed by a wallet if it's not listed"
  );

  let balance = await addresses[0].balance();

  assert.equal(
    balance.total,
    100_000_000_000n,
    "Expected total balance for the first address"
  );

  assert.equal(
    balance.spendable,
    40_000_000_000n,
    "Expected spendable balance for the first address"
  );

  balance = await addresses[1].balance();

  assert.equal(
    balance.total,
    0n,
    "Expected total balance for the second address"
  );

  assert.equal(
    balance.spendable,
    0n,
    "Expected spendable balance for the second address"
  );

  assert.end();
});
