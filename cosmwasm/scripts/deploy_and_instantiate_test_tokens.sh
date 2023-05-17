
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

WALLET_ADDRESS_LOCAL=$(echo $PASSWORD_LOCAL | wasmd keys show $WALLET_NAME_LOCAL -a)



if [[ -z "$CW20_CODE_ID" ]]; then
    echo "Deploy CW20 contract"
    deploy_output=$(echo $PASSWORD_LOCAL | wasmd tx wasm store ./scripts/mock-artifacts/cw20_base.wasm --from $1 $TX_FLAG -y -b block --output json)
    echo -n "Code id: "
    export CW20_CODE_ID=$(echo $deploy_output | jq -r '.logs[0].events[-1].attributes[-1].value')
    echo $CW20_CODE_ID
else
    echo "Reusing CW20 contract $CW20_CODE_ID"
fi
echo



instantiate_token(){
    local TOKEN_NAME=$1
    local TOKEN_SYMBOL=$2

    echo "Instantiate Test Token '$TOKEN_NAME'"

    local INSTANTIATE_MESSAGE='{"name":"'$TOKEN_NAME'","symbol":"'$TOKEN_SYMBOL'","decimals":6,"initial_balances":[{"address":"'$WALLET_ADDRESS_LOCAL'","amount":"100000000"}],"mint":{"minter":"'$WALLET_ADDRESS_LOCAL'"}}'

    echo $PASSWORD_LOCAL | wasmd tx wasm instantiate $CW20_CODE_ID $INSTANTIATE_MESSAGE --from $WALLET_NAME $TX_FLAG --label $TOKEN_SYMBOL -b block --no-admin -y &> /dev/null

    TEST_TOKEN_ADDRESS=$(wasmd query wasm list-contract-by-code $CW20_CODE_ID --output json | jq -r '.contracts[-1]')

    echo "Token address: $TEST_TOKEN_ADDRESS"
    echo

}

instantiate_token "TokenA" "TKNA"
export TEST_TOKEN_1_ADDRESS=$TEST_TOKEN_ADDRESS

instantiate_token "TokenB" "TKNB"
export TEST_TOKEN_2_ADDRESS=$TEST_TOKEN_ADDRESS

instantiate_token "TokenC" "TKNC"
export TEST_TOKEN_3_ADDRESS=$TEST_TOKEN_ADDRESS


PASSWORD_LOCAL=""
