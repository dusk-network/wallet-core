help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: wasm ## Run the wasmer tests
	@cargo test

wasm: ## Build the WASM files
	@RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536" \
		cargo build \
			--release \
			--color=always \
			-Z build-std=core,alloc,panic_abort \
			-Z build-std-features=panic_immediate_abort \
			--target wasm32-unknown-unknown

package: ## Prepare the WASM npm package
	wasm-opt -O4 \
		--output-target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm \
		-o mod.wasm

.PHONY: test wasm help
