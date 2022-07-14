ASYNCIFY_METHODS := \
	asyncify-import@env.compute_proof_and_propagate\
	asyncify-import@env.request_stct_proof\
	asyncify-import@env.request_wfct_proof\
	asyncify-import@env.fetch_anchor\
	asyncify-import@env.fetch_stake\
	asyncify-import@env.fetch_notes\
	asyncify-import@env.fetch_existing_nullifiers\
	asyncify-import@env.fetch_opening

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

wasm: ## Generate the optimized WASM
	@cargo rustc \
		--manifest-path=./Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-Z build-std=std,panic_abort \
		-Z build-std-features=panic_immediate_abort \
		-- -C link-args=-s

asyncify: ## Apply asyncify
	wasm-opt --asyncify -O4 \
			--pass-arg "${ASYNCIFY_METHODS}" \
			target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm \
			-o ../js/wallet-core.wasm

all: wasm asyncify ## Generate the optimized WASM and apply asyncify

.PHONY: wasm asyncify all