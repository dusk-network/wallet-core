PROJECT := $(shell sed -n '0,/name\s*=\s*"\(.*\)"/s/name\s*=\s*"\(.*\)"/\1/p' Cargo.toml)
VERSION := $(shell sed -n '0,/version\s*=\s*"\(.*\)"/s/version\s*=\s*"\(.*\)"/\1/p' Cargo.toml)
FLAGS := RUSTFLAGS="$(RUSTFLAGS) --remap-path-prefix $(HOME)= -C link-args=-zstack-size=65536"
WASM := "target/wasm32-unknown-unknown/release/$(shell sed -n '0,/name\s*=\s*"\(.*\)"/s/name\s*=\s*"\(.*\)"/\1/p' Cargo.toml | sed 's/-/_/g').wasm"
NPM_WASM := "mod.wasm"
PACKAGE := "assets/$(PROJECT)-$(VERSION).wasm"

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

test: wasm ## Run the wasmer tests
	@cargo test

wasm: ## Build the WASM files
	@$(FLAGS) cargo build --release \
		--target wasm32-unknown-unknown \
		--color=always \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=panic_immediate_abort

package: wasm ## Prepare the WASM package
	@wasm-opt -O3 $(WASM) -o $(NPM_WASM)
	@cp $(NPM_WASM) $(PACKAGE)
	@echo "Package created: $(PACKAGE)"

.PHONY: test wasm package help
