#!/bin/bash
cd contracts
for contract_name in "swap-pool-factory" "swap-pool-volatile" "catalyst-ibc-interface"; do
    cd $contract_name
    RUSTFLAGS='-C link-arg=-s' cargo build --release --lib --target=wasm32-unknown-unknown --locked
    cd ..
done
cd ..