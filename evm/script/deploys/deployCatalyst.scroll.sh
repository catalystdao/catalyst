set -o allexport
source .scroll.env
set +o allexport

forge script FundAddresses --fork-url=$RPC_URL --broadcast

forge create WrappedGas9 --constructor-args "Wrapped ETH", $WGAS --rpc-url $RPC_URL --private-key $WGAS_DEPLOYER

forge script DeployTokens --fork-url=$RPC_URL --broadcast

forge script DeployInterfaces --fork-url=$RPC_URL --broadcast

forge script DeployCatalyst --fork-url=$RPC_URL --broadcast

forge script DeployVaults --fork-url=$RPC_URL --broadcast