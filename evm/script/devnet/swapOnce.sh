echo "Executing the swap from ETH to BSC"
brownie run scripts/devnet/swapETHToBSCCatalyst.py --network polymerase-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi


echo "Testing for polymerase tx on BSC"
brownie run scripts/devnet/checkForTX.py --network polymerase-bsc-geth
exit_status=$?
if [ "${exit_status}" -ne 0 ];
then
    exit ${exit_status}
fi