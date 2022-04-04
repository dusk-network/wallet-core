WASM_OPT?=./lib/binaryen/bin/wasm-opt

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

install: ## Install the toolchain
	make installBinaryen | \
	npm install

installBinaryen:
	make installBinaryen`(uname -s)` \

installBinaryenLinux:
	wget https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-linux.tar.gz -nc -P ./bin | \
    mkdir -p ./lib/binaryen && tar -xvf ./bin/binaryen-version_105-x86_64-linux.tar.gz -C ./lib/binaryen --strip-components 1 

installBinaryenDarwin:
	wget https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-macos.tar.gz -nc -P ./bin | \
	mkdir -p ./lib/binaryen && tar -xvf ./bin/binaryen-version_105-x86_64-macos.tar.gz -C ./lib/binaryen --strip-components 1

wasm: ## Generate WASM
	@cargo rustc \
		--manifest-path=./Cargo.toml \
		--release \
		--target wasm32-unknown-unknown \
		-- -C link-args=-s

asyncify: ## Generate WASM with asyncify
	$(WASM_OPT) --asyncify -O4 \
		--pass-arg asyncify-import@env.compute_proof_and_propagate \
		--pass-arg asyncify-import@env.request_stct_proof \
		--pass-arg asyncify-import@env.request_wfct_proof \
		--pass-arg asyncify-import@env.fetch_anchor \
		--pass-arg asyncify-import@env.fetch_block_height \
		--pass-arg asyncify-import@env.fetch_stake \
		--pass-arg asyncify-import@env.fetch_notes \
		--pass-arg asyncify-import@env.fetch_existing_nullifiers \
		--pass-arg asyncify-import@env.fetch_opening \
		target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm \
		-o mod.wasm

all: ## Install, build and test
	make install
	make wasm
	make asyncify
