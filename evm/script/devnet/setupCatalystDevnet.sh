if ! command -v brownie &> /dev/null
then
    echo "BROWNIE could not be found. Try 'poetry shell'"
    exit
fi


echo "compiling contracts"
brownie compile

exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi


echo "Adding chain configs"
brownie networks add Ethereum polymerase-geth  host=http://127.0.0.1:10000 chainid=1337
brownie networks add 'Binance Smart Chain' polymerase-bsc-geth  host=http://127.0.0.1:10001 chainid=1234

echo "Deleting devnet eth stores"
rm -rf build/deployments/1337
echo "Deleting devnet bsc stores"
rm -rf build/deployments/1234
echo "Deleting devnet map"
rm build/deployments/map.json


echo "Deploying contract on ETH"
brownie run scripts/devnet/startCatalystETH.py --network polymerase-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi


echo "Deploying contract on BSC"
brownie run scripts/devnet/startCatalystBSC.py --network polymerase-bsc-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi

echo "Creating connection between pools from ETH to BSC"
brownie run scripts/devnet/connectCatalystETH.py --network polymerase-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi

echo "Creating connection between pools from BSC to ETH"
brownie run scripts/devnet/connectCatalystBSC.py --network polymerase-bsc-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi

# wait for channel creation
./waitForChannels.sh

./swapOnce.sh
