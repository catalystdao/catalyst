CHAIN_NAME="cronos"
WGAS="WCRO"
RPC_URL="http://127.0.0.1:8547"


forge script FundAddresses --fork-url=$RPC_URL --broadcast

forge script DeployInterfaces --fork-url=$RPC_URL --broadcast

forge create WrappedGas9 --constructor-args "Wrapped CRO", $WGAS --rpc-url $RPC_URL --private-key $WGAS_DEPLOYER

forge script DeployCatalyst --fork-url=$RPC_URL --broadcast

forge script DeployTokens --fork-url=$RPC_URL --broadcast

forge script DeployVaults --fork-url=$RPC_URL --broadcast