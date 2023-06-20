#!/bin/bash
cd contracts
for contract_name in "catalyst-factory" "catalyst-vault-volatile" "catalyst-vault-amplified" "catalyst-ibc-interface" "catalyst-ibc-interface-poa"; do
    cd $contract_name
    RUSTFLAGS='-C link-arg=-s' cargo build --release --lib --target=wasm32-unknown-unknown --locked
    cd ..
done
cd ..