#!/bin/sh

wasm-opt --asyncify -O4 \
  --pass-arg asyncify-import@env.compute_proof_and_propagate \
  --pass-arg asyncify-import@env.request_stct_proof \
  --pass-arg asyncify-import@env.request_wfct_proof \
  --pass-arg asyncify-import@env.fetch_anchor \
  --pass-arg asyncify-import@env.fetch_stake \
  --pass-arg asyncify-import@env.fetch_notes \
  --pass-arg asyncify-import@env.fetch_existing_nullifiers \
  --pass-arg asyncify-import@env.fetch_opening \
  target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm \
  -o mod.wasm
