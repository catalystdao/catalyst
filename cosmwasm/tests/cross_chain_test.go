package cosmwasmtesting

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
)

func TestCrossChain(t *testing.T) {

	t.Parallel()

	ctx := context.Background()

	// Initialize the chains
	env := InitializeTestEnv(t, ctx)


	// Juno A
	junoAEnv := env[0]
	junoAChain := junoAEnv.Chain
	junoAGovernanceAddr := junoAEnv.UsersAddresses[0]

	junoAFactoryContractAddr := DeployAndInstantiateFactory(t, ctx, junoAChain, junoAGovernanceAddr)
	junoAInterfaceContractAddr := DeployAndInstantiateInterfacePoA(t, ctx, junoAChain, junoAGovernanceAddr)
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

	junoAVaultAddressEncoded := EncodeAddress(t, junoAVaultAddress)

	// Juno B
	junoBEnv := env[0]
	junoBChain := junoBEnv.Chain
	junoBGovernanceAddr := junoBEnv.UsersAddresses[0]

	junoBFactoryContractAddr := DeployAndInstantiateFactory(t, ctx, junoBChain, junoBGovernanceAddr)
	junoBInterfaceContractAddr := DeployAndInstantiateInterfacePoA(t, ctx, junoBChain, junoBGovernanceAddr)
	junoBTestTokens := DeployAndInstantiateTestTokens(t, ctx, junoBChain, junoBGovernanceAddr)
	junoBVaultCodeId := StoreVolatileVault(t, ctx, junoBChain, junoBGovernanceAddr)

	junoBVaultAddress := DeployVault(
		t,
		ctx,
		junoBChain,
		junoBGovernanceAddr,
		junoBTestTokens,
		junoBFactoryContractAddr,
		junoBInterfaceContractAddr,
		junoBVaultCodeId,
	)

	_ = junoBVaultAddress

	junoBVaultAddressEncoded := EncodeAddress(t, junoBVaultAddress)


	// Connect Juno A and Juno B
	channel_id := "00000000000000000000000000000001"
	connectionMsg := fmt.Sprintf(`{"set_connection":{"channel_id":"%s","to_vault":"%s","state":true}}`, channel_id, junoBVaultAddressEncoded)
	stdout, _ := ExecuteCmd(
		t,
		ctx,
		junoAChain,
		junoAGovernanceAddr,
		junoAVaultAddress,
		connectionMsg,
	)

	t.Log(string(stdout))

	connectionMsg = fmt.Sprintf(`{"set_connection":{"channel_id":"%s","to_vault":"%s","state":true}}`, channel_id, junoAVaultAddressEncoded)
	stdout, _ = ExecuteCmd(
		t,
		ctx,
		junoBChain,
		junoBGovernanceAddr,
		junoBVaultAddress,
		connectionMsg,
	)

	t.Log(string(stdout))


	// Send Asset

	junoASwapperAddr := junoAEnv.UsersAddresses[1]
	junoBSwapperAddr := junoBEnv.UsersAddresses[1]
	junoBSwapperAddrEncoded := EncodeAddress(t, junoBSwapperAddr)
	amount := 100

	// Transfer token to swapper
	transferMsg := fmt.Sprintf(`{"transfer":{"recipient":"%s","amount":"%d"}}`, junoASwapperAddr, amount)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, junoATestTokens[0], transferMsg)

	// Set token allowance
	setAllowanceMsg := fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"%d"}}`, junoAVaultAddress, amount)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoASwapperAddr, junoATestTokens[0], setAllowanceMsg)

	sendAssetMsg := fmt.Sprintf(`{"send_asset":{"channel_id":"%s","to_vault":"%s","to_account":"%s","from_asset":"%s","to_asset_index":0,"amount":"%d","min_out":"0","fallback_account":"%s","calldata":""}}`, channel_id, junoBVaultAddressEncoded, junoBSwapperAddrEncoded, junoATestTokens[0], amount, junoASwapperAddr)

	stdout, _ = ExecuteCmd(t, ctx, junoAChain, junoASwapperAddr, junoAVaultAddress, sendAssetMsg)


	var response Response
	json.Unmarshal([]byte(stdout), &response)
	ibcData := response.Logs[0].Events[2].Attributes[9].Value



	// Complete swap
	receiveMsg := fmt.Sprintf(`{"i_b_c_packet_receive":{"data":"%s","channel_id":"%s"}}`, ibcData, channel_id)
	stdout, _ = ExecuteCmd(t, ctx, junoBChain, junoBGovernanceAddr, junoBInterfaceContractAddr, receiveMsg)

	

	json.Unmarshal([]byte(stdout), &response)
	t.Log("Receive response: ", response)
	ack := response.Logs[0].Events[3].Attributes[7].Value

	t.Log("ACK: ", ack)


	// Execute Ack
	ackMsg := fmt.Sprintf(`{"i_b_c_packet_ack":{"data":"%s","response":"%s","channel_id":"%s"}}`, ibcData, ack, channel_id)
	stdout, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, junoAInterfaceContractAddr, ackMsg)

	json.Unmarshal([]byte(stdout), &response)
	t.Log("Ack response: ", response)
	
}
