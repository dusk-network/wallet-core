# ⛔ [DEPRECATED] Wallet Core

This repository has been archived and is no longer actively maintained.

The development of Wallet Core has been moved to the [Rusk Monorepo](https://github.com/dusk-network/rusk/tree/master/wallet-core). Please refer to that repository for the latest code, updates, and contributions.

---

![status](https://github.com/dusk-network/wallet-core/workflows/Dusk%20CI/badge.svg)
[![codecov](https://codecov.io/gh/dusk-network/wallet-core/branch/main/graph/badge.svg?token=9W3J09AWZG)](https://codecov.io/gh/dusk-network/wallet-core)
[![documentation](https://img.shields.io/badge/docs-wallet-blue?logo=rust)](https://docs.rs/dusk-wallet-core/)

A WASM library to provide business logic for Dusk wallet implementations.

Check the available methods under the [FFI](src/ffi.rs) module.

Every function expects a fat pointer to its arguments already allocated to the WASM memory. For the arguments definition, check the [JSON Schema](assets/schema.json). It will consume this pointer region and free it after execution. The return of the function will also be in accordance to the schema, and the user will have to free the memory himself after fetching the data.

For maximum compatibility, every WASM function returns a `i64` with the status of the operation and an embedded pointer. The structure of the bytes in big-endian is as follows:

[(pointer) x 4bytes (length) x 3bytes (status) x 1bit]

The pointer will be a maximum `u32` number, and the length a `u24` number. The status of the operation is the least significant bit of the number, and will be `0` if the operation is successful.

Here is an algorithm to split the result into meaningful parts:

```rust,ignore
let ptr = (result >> 32) as u64;
let len = ((result << 32) >> 48) as u64;
let success = ((result << 63) >> 63) == 0;
```

For an example usage, check the [wallet-cli](https://github.com/dusk-network/wallet-cli) implementation that consumes this library.

## Requirements

- [Rust 1.71.0](https://www.rust-lang.org/)
- [target.wasm32-unknown-unknown](https://github.com/rustwasm/)
- [binaryen](https://github.com/WebAssembly/binaryen) to generate packages

## Build

To build a distributable package:

```sh
make package
```

## Test

To run the tests, there is an automated Makefile script

```sh
make test
```
