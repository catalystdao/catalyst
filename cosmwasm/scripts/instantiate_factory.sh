#!/bin/bash

if [[ $# -ne 2 ]]; then
    echo "Expected two arguments: <wallet_name> <factory_code_id>"
    return
fi

WALLET_NAME=$1
FACTORY_CODE_ID=$2

if [[ -z "$PASSWORD" ]]; then
    echo -n wasmd keyring password:
    read -s PASSWORD_LOCAL
    echo
    echo
else
    PASSWORD_LOCAL=$PASSWORD
fi

echo Instantiate factory
echo $PASSWORD_LOCAL | wasmd tx wasm instantiate $FACTORY_CODE_ID '{"default_governance_fee_share":"0"}' --from $WALLET_NAME $TX_FLAG --label factory -b block --no-admin -y &> /dev/null
export FACTORY_ADDRESS=$(wasmd query wasm list-contract-by-code $FACTORY_CODE_ID --output json | jq -r '.contracts[-1]')
echo -n "Factory address: "
echo $FACTORY_ADDRESS

PASSWORD_LOCAL=""
