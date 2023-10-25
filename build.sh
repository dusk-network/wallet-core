make wasm
cp target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm ./assets
wasm-opt -O3 ./assets/dusk_wallet_core.wasm -o "dusk-wallet-core-0.21.0.wasm"
cp ./dusk-wallet-core-0.21.0.wasm ../../Web/dusk-wallet-js/assets
