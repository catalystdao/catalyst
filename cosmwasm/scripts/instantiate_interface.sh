#!/bin/bash

if [[ $# -ne 2 ]]; then
    echo "Expected two arguments: <wallet_name> <interface_code_id>"
    return
fi

WALLET_NAME=$1
INTERFACE_CODE_ID=$2

if [[ -z "$PASSWORD" ]]; then
    echo -n wasmd keyring password:
    read -s PASSWORD_LOCAL
    echo
    echo
else
    PASSWORD_LOCAL=$PASSWORD
fi

echo Instantiate Interface
echo $PASSWORD_LOCAL | wasmd tx wasm instantiate $INTERFACE_CODE_ID '{}' --from $WALLET_NAME $TX_FLAG --label factory -b block --no-admin -y &> /dev/null
export INTERFACE_ADDRESS=$(wasmd query wasm list-contract-by-code $INTERFACE_CODE_ID --output json | jq -r '.contracts[-1]')
echo -n "Interface address: "
echo $INTERFACE_ADDRESS

PASSWORD_LOCAL=""
