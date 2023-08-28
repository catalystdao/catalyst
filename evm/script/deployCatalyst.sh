forge script FundAddresses --fork-url=http://127.0.0.1:8545 --broadcast

forge script DeployInterfaces --fork-url=http://127.0.0.1:8545 --broadcast

forge create WrappedGas9 --constructor-args "Wrapped ETH", "WETH" --rpc-url http://127.0.0.1:8545 --private-key $WGAS_DEPLOYER

forge script DeployCatalyst --fork-url=http://127.0.0.1:8545 --broadcast

forge script DeployTokens --fork-url=http://127.0.0.1:8545 --broadcast

# forge script DeployVaults --fork-url=http://127.0.0.1:8545 --broadcast