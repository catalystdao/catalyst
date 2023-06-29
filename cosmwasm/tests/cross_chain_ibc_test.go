package cosmwasmtesting

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"

	"github.com/strangelove-ventures/interchaintest/v4"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zaptest"
)



type ContractResponse struct {
	IbcPortId string
}



func GetPortId(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	contract string,
) {
	
	// Get the portIds
	cmd := []string{chain.Config().Bin, "query", "wasm", "contract", contract,
		"--node", chain.GetRPCAddress(),
		"--home", chain.HomeDir(),
		"--chain-id", chain.Config().ChainID,
		"--output", "json",
	}

	stdout, _, err := chain.Exec(ctx, cmd, nil)
	require.NoError(t, err)

	t.Log("==============================")
	t.Log("==============================")
	t.Log("==============================")
	t.Log("==============================")
	t.Log(string(stdout))

	var contractResponse ContractResponse
	json.Unmarshal([]byte(stdout), &contractResponse)

	t.Log(contractResponse)
	// ibcData := response.Logs[0].Events[2].Attributes[9].Value
}

func QueryBalance(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
	token string,
) {

	//wasmd query wasm contract-state smart $VAULT_ADDRESS '{"balance":{"address":"'$SWAPPER_ADDRESS'"}}'

	queryMsg := fmt.Sprintf(`{"balance":{"address":"%s"}}`, account)
	cmd := []string{chain.Config().Bin, "query", "wasm", "contract-state", "smart", token, queryMsg,
		"--node", chain.GetRPCAddress(),
		"--home", chain.HomeDir(),
		"--chain-id", chain.Config().ChainID,
		"--output", "json",
	}

	stdout, _, err := chain.Exec(ctx, cmd, nil)
	require.NoError(t, err)

	t.Log("BALANCE ===================================================")
	t.Log(string(stdout))

	// return stdout, stderr

}

func TestCrossChainIBC(t *testing.T) {

	t.Parallel()

	ctx := context.Background()

	// Initialize the chains
	// Initialize the chain factory
	validatorsCount := 1
	fullNodesCount := 1
	cf := interchaintest.NewBuiltinChainFactory(zaptest.NewLogger(t), []*interchaintest.ChainSpec{
		{Name: "juno", Version: "v15.0.0", ChainConfig: ibc.ChainConfig{
			GasPrices: "0.0uatom",
		}, NumValidators: &validatorsCount, NumFullNodes: &fullNodesCount,},
		{Name: "juno", Version: "v15.0.0", ChainConfig: ibc.ChainConfig{
			GasPrices: "0.0uatom",
		}, NumValidators: &validatorsCount,NumFullNodes: &fullNodesCount,},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)
	junoA, junoB := chains[0], chains[1]




	client, network := interchaintest.DockerSetup(t)
	_ = client
	_ = network




	// Create and Fund User Wallets
	fundAmount := int64(10_000_000)
	users := interchaintest.GetAndFundTestUsers(t, ctx, "default", fundAmount, junoA, junoA, junoB, junoB)
	junoAUser1 := users[0]
	junoAUser2 := users[1]
	// junoBUser1 := users[2]
	// junoBUser2 := users[3]
	junoAUser1Addr := junoAUser1.Bech32Address(junoA.Config().Bech32Prefix)
	junoAUser2Addr := junoAUser2.Bech32Address(junoA.Config().Bech32Prefix)
	// junoBUser1Addr := junoBUser1.Bech32Address(junoB.Config().Bech32Prefix)
	// junoBUser2Addr := junoBUser2.Bech32Address(junoB.Config().Bech32Prefix)

	// junoAUserBalInitial, err := junoA.GetBalance(ctx, junoAUserAddr, junoA.Config().Denom)
	// require.NoError(t, err)
	// require.Equal(t, fundAmount, junoAUserBalInitial)

	// junoBUserBalInitial, err := junoB.GetBalance(ctx, junoBUserAddr, junoB.Config().Denom)
	// require.NoError(t, err)
	// require.Equal(t, fundAmount, junoBUserBalInitial)


	junoAConfig := TestChainConfig{
		Chain: junoA.(*cosmos.CosmosChain),
		Users: []*ibc.Wallet{junoAUser1, junoAUser2},
		UsersAddresses: []string{junoAUser1Addr, junoAUser2Addr},
	}

	// junoBConfig := TestChainConfig{
	// 	Chain: junoB.(*cosmos.CosmosChain),
	// 	Users: []*ibc.Wallet{junoBUser1, junoBUser2},
	// 	UsersAddresses: []string{junoBUser1Addr, junoBUser2Addr},
	// }





	// Juno A
	junoAEnv := junoAConfig
	junoAChain := junoAEnv.Chain
	junoAGovernanceAddr := junoAEnv.UsersAddresses[0]

	junoAFactoryContractAddr := DeployAndInstantiateFactory(t, ctx, junoAChain, junoAGovernanceAddr)
	junoAInterfaceContractAddr := DeployAndInstantiateInterface(t, ctx, junoAChain, junoAGovernanceAddr)
	junoATestTokens := DeployAndInstantiateTestTokens(t, ctx, junoAChain, junoAGovernanceAddr)
	junoAVaultCodeId := StoreVolatileVault(t, ctx, junoAChain, junoAGovernanceAddr)


	junoAVaultAddress := DeployVault(
		t,
		ctx,
		junoAChain,
		junoAGovernanceAddr,
		junoATestTokens,
		junoAFactoryContractAddr,
		junoAInterfaceContractAddr,
		junoAVaultCodeId,
	)

	_ = junoAVaultAddress

	// junoAVaultAddressEncoded := EncodeAddress(t, junoAVaultAddress)

	// // Juno B
	// junoBEnv := junoBConfig
	// junoBChain := junoBEnv.Chain
	// junoBGovernanceAddr := junoBEnv.UsersAddresses[0]

	// junoBFactoryContractAddr := DeployAndInstantiateFactory(t, ctx, junoBChain, junoBGovernanceAddr)
	// junoBInterfaceContractAddr := DeployAndInstantiateInterface(t, ctx, junoBChain, junoBGovernanceAddr)
	// junoBTestTokens := DeployAndInstantiateTestTokens(t, ctx, junoBChain, junoBGovernanceAddr)
	// junoBVaultCodeId := StoreVolatileVault(t, ctx, junoBChain, junoBGovernanceAddr)

	// junoBVaultAddress := DeployVault(
	// 	t,
	// 	ctx,
	// 	junoBChain,
	// 	junoBGovernanceAddr,
	// 	junoBTestTokens,
	// 	junoBFactoryContractAddr,
	// 	junoBInterfaceContractAddr,
	// 	junoBVaultCodeId,
	// )

	// _ = junoBVaultAddress

	// junoBVaultAddressEncoded := EncodeAddress(t, junoBVaultAddress)





	// Setup IBC connection
	GetPortId(
		t,
		ctx,
		junoAChain,
		junoAInterfaceContractAddr,
	)

	

	// // Relayer Factory
	// r := interchaintest.NewBuiltinRelayerFactory(ibc.CosmosRly, zaptest.NewLogger(t)).Build(
	// 	t, client, network)

	// // Prep Interchain
	// const ibcPath = "junoA-junoB-demo"
	// ic := interchaintest.NewInterchain().
	// 	AddChain(junoA).
	// 	AddChain(junoB).
	// 	AddRelayer(r, "relayer").
	// 	AddLink(interchaintest.InterchainLink{
	// 		Chain1:  junoA,
	// 		Chain2:  junoB,
	// 		Relayer: r,
	// 		Path:    ibcPath,
	// 	})

	// // Log location
	// f, err := interchaintest.CreateLogFile(fmt.Sprintf("%d.json", time.Now().Unix()))
	// require.NoError(t, err)
	// // Reporter/logs
	// rep := testreporter.NewReporter(f)
	// eRep := rep.RelayerExecReporter(t)

	// // Build interchain
	// require.NoError(t, ic.Build(ctx, eRep, interchaintest.InterchainBuildOptions{
	// 	TestName:          t.Name(),
	// 	Client:            client,
	// 	NetworkID:         network,
	// 	// BlockDatabaseFile: interchaintest.DefaultBlockDatabaseFilepath(),

	// 	SkipPathCreation: false},
	// ),
	// )
	
	// // Get Channel ID
	// junoAChannelInfo, err := r.GetChannels(ctx, eRep, junoA.Config().ChainID)
	// require.NoError(t, err)
	// junoAChannelID := junoAChannelInfo[0].ChannelID

	// junoBChannelInfo, err := r.GetChannels(ctx, eRep, junoB.Config().ChainID)
	// require.NoError(t, err)
	// junoBChannelID := junoBChannelInfo[0].ChannelID
	// require.NoError(t, r.StartRelayer(ctx, eRep, ibcPath))


	// // Connect Juno A and Juno B
	// connectionMsg := fmt.Sprintf(`{"set_connection":{"channel_id":"%s","to_vault":"%s","state":true}}`, junoAChannelID, junoBVaultAddressEncoded)
	// stdout, _ := ExecuteCmd(
	// 	t,
	// 	ctx,
	// 	junoAChain,
	// 	junoAGovernanceAddr,
	// 	junoAVaultAddress,
	// 	connectionMsg,
	// )

	// t.Log(string(stdout))

	// connectionMsg = fmt.Sprintf(`{"set_connection":{"channel_id":"%s","to_vault":"%s","state":true}}`, junoBChannelID, junoAVaultAddressEncoded)
	// stdout, _ = ExecuteCmd(
	// 	t,
	// 	ctx,
	// 	junoBChain,
	// 	junoBGovernanceAddr,
	// 	junoBVaultAddress,
	// 	connectionMsg,
	// )

	// t.Log(string(stdout))


	// // Send Asset

	// junoASwapperAddr := junoAEnv.UsersAddresses[1]
	// junoBSwapperAddr := junoBEnv.UsersAddresses[1]
	// junoBSwapperAddrEncoded := EncodeAddress(t, junoBSwapperAddr)
	// amount := 100

	// QueryBalance(
	// 	t,
	// 	ctx,
	// 	junoBChain,
	// 	junoBSwapperAddr,
	// 	junoBTestTokens[0],
	// )

	// // Transfer token to swapper
	// transferMsg := fmt.Sprintf(`{"transfer":{"recipient":"%s","amount":"%d"}}`, junoASwapperAddr, amount)
	// _, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, junoATestTokens[0], transferMsg)

	// // Set token allowance
	// setAllowanceMsg := fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"%d"}}`, junoAVaultAddress, amount)
	// _, _ = ExecuteCmd(t, ctx, junoAChain, junoASwapperAddr, junoATestTokens[0], setAllowanceMsg)

	// sendAssetMsg := fmt.Sprintf(`{"send_asset":{"channel_id":"%s","to_vault":"%s","to_account":"%s","from_asset":"%s","to_asset_index":0,"amount":"%d","min_out":"0","fallback_account":"%s","calldata":""}}`, junoAChannelID, junoBVaultAddressEncoded, junoBSwapperAddrEncoded, junoATestTokens[0], amount, junoASwapperAddr)

	// stdout, _ = ExecuteCmd(t, ctx, junoAChain, junoASwapperAddr, junoAVaultAddress, sendAssetMsg)


	// var response Response
	// json.Unmarshal([]byte(stdout), &response)
	// // ibcData := response.Logs[0].Events[2].Attributes[9].Value
	// t.Log(response)
	
	
	// // require.NoError(t, env.Relayer.FlushPackets(ctx, &env.RelayerReporter, env.IBCPath, junoBEnv.ChannelID))
	// // require.NoError(t, env.Relayer.FlushAcknowledgements(ctx, &env.RelayerReporter, env.IBCPath, junoAEnv.ChannelID))


	// // ! No return observed at this point
	// // Query to account balance
	// for i:=0; i <= 100; i++ {
	// 	QueryBalance(
	// 		t,
	// 		ctx,
	// 		junoBChain,
	// 		junoBSwapperAddr,
	// 		junoBTestTokens[0],
	// 	)

	// 	if err := testutil.WaitForBlocks(ctx, 2, junoBChain); err != nil {
	// 		t.Fatal(err)
	// 	}
	// }





	


	// // Complete swap
	// receiveMsg := fmt.Sprintf(`{"i_b_c_packet_receive":{"data":"%s","channel_id":"%s"}}`, ibcData, channel_id)
	// stdout, _ = ExecuteCmd(t, ctx, junoBChain, junoBGovernanceAddr, junoBInterfaceContractAddr, receiveMsg)

	

	// json.Unmarshal([]byte(stdout), &response)
	// t.Log("Receive response: ", response)
	// ack := response.Logs[0].Events[3].Attributes[7].Value

	// t.Log("ACK: ", ack)


	// // Execute Ack
	// ackMsg := fmt.Sprintf(`{"i_b_c_packet_ack":{"data":"%s","response":"%s","channel_id":"%s"}}`, ibcData, ack, channel_id)
	// stdout, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, junoAInterfaceContractAddr, ackMsg)

	// json.Unmarshal([]byte(stdout), &response)
	// t.Log("Ack response: ", response)
	
}
