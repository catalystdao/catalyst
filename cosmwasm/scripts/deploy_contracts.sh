#!/bin/bash

if [[ $# -ne 1 ]]; then
    echo "Expected one argument: <wallet_name>"
    return
fi

WALLET_NAME_LOCAL=$1

if [[ -z "$PASSWORD" ]]; then
    echo -n wasmd keyring password:
    read -s PASSWORD_LOCAL
    echo
    echo
else
    PASSWORD_LOCAL=$PASSWORD
fi

echo Deploy factory
deploy_output=$(echo $PASSWORD_LOCAL | wasmd tx wasm store ./target/wasm32-unknown-unknown/release/catalyst_factory.wasm --from $WALLET_NAME_LOCAL $TX_FLAG -y -b block --output json)
export FACTORY_CODE_ID=$(echo $deploy_output | jq -r '.logs[0].events[-1].attributes[-1].value')
echo "Code id: $FACTORY_CODE_ID"
echo

echo Deploy volatile interface
deploy_output=$(echo $PASSWORD_LOCAL | wasmd tx wasm store ./target/wasm32-unknown-unknown/release/catalyst_ibc_interface.wasm --from $WALLET_NAME_LOCAL $TX_FLAG -y -b block --output json)
export INTERFACE_CODE_ID=$(echo $deploy_output | jq -r '.logs[0].events[-1].attributes[-1].value')
echo "Code id: $INTERFACE_CODE_ID"
echo

echo Deploy volatile vault
deploy_output=$(echo $PASSWORD_LOCAL | wasmd tx wasm store ./target/wasm32-unknown-unknown/release/catalyst_vault_volatile.wasm --from $WALLET_NAME_LOCAL $TX_FLAG -y -b block --output json)
export VOLATILE_CODE_ID=$(echo $deploy_output | jq -r '.logs[0].events[-1].attributes[-1].value')
echo "Code id: $VOLATILE_CODE_ID"
echo

echo Deploy PoA interface
deploy_output=$(echo $PASSWORD_LOCAL | wasmd tx wasm store ./target/wasm32-unknown-unknown/release/catalyst_ibc_interface_poa.wasm --from $WALLET_NAME_LOCAL $TX_FLAG -y -b block --output json)
export INTERFACE_CODE_ID=$(echo $deploy_output | jq -r '.logs[0].events[-1].attributes[-1].value')
echo "Code id: $INTERFACE_CODE_ID"
echo

PASSWORD_LOCAL=""
