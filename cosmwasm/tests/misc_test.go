package cosmwasmtesting

import (
	"context"
	"fmt"
	"testing"
)

func TestLearn(t *testing.T) {

	t.Parallel()

	ctx := context.Background()

	// Initialize the chains
	env := InitializeTestEnv(t, ctx)

	junoAEnv := env.Chains[0]
	junoAChain := junoAEnv.Chain
	junoAGovernanceAddr := junoAEnv.UsersAddresses[0]

	junoBEnv := env.Chains[1]

	_ = junoBEnv

	factoryContractAddr := DeployAndInstantiateFactory(t, ctx, junoAChain, junoAGovernanceAddr)
	interfaceContractAddr := DeployAndInstantiateInterfacePoA(t, ctx, junoAChain, junoAGovernanceAddr)
	testTokens := DeployAndInstantiateTestTokens(t, ctx, junoAChain, junoAGovernanceAddr)
	vaultCodeId := StoreVolatileVault(t, ctx, junoAChain, junoAGovernanceAddr)

	vaultAddress := DeployVault(
		t,
		ctx,
		junoAChain,
		junoAGovernanceAddr,
		testTokens,
		factoryContractAddr,
		interfaceContractAddr,
		vaultCodeId,
	)

	_ = vaultAddress

	junoASwapperAddr := junoAEnv.UsersAddresses[1]
	amount := 100

	// Transfer tokens to swapper
	transferMsg := fmt.Sprintf(`{"transfer":{"recipient":"%s","amount":"%d"}}`, junoASwapperAddr, amount)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, testTokens[0], transferMsg)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, testTokens[1], transferMsg)

	// Set token allowance
	setAllowanceMsg := fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"%d"}}`, vaultAddress, amount)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, testTokens[0], setAllowanceMsg)
	_, _ = ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, testTokens[1], setAllowanceMsg)

	// Local swap
	localSwapMsg := fmt.Sprintf(`{"local_swap":{"from_asset":"%s","to_asset":"%s","amount":"%d","min_out":"0"}}`, testTokens[0], testTokens[1], amount)

	stdout, _ := ExecuteCmd(t, ctx, junoAChain, junoAGovernanceAddr, vaultAddress, localSwapMsg)

	t.Log(string(stdout[:]))
	
}
