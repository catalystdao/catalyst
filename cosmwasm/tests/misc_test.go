package cosmwasmtesting

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/strangelove-ventures/interchaintest/v4"
	"github.com/strangelove-ventures/interchaintest/v4/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v4/ibc"
	"github.com/strangelove-ventures/interchaintest/v4/testreporter"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zaptest"
)

func SetupContract(t *testing.T, ctx context.Context, chain *cosmos.CosmosChain, keyname string, fileLoc string, message string) (codeId, contract string) {
	codeId, err := chain.StoreContract(ctx, keyname, fileLoc)
	if err != nil {
		t.Fatal(err)
	}

	contractAddr, err := chain.InstantiateContract(ctx, keyname, codeId, message, true)
	if err != nil {
		t.Fatal(err)
	}

	return codeId, contractAddr
}

// func getTransaction(chain *cosmos.CosmosChain, txHash string) (*types.TxResponse, error) {
// 	// Retry because sometimes the tx is not committed to state yet.
// 	var txResp *types.TxResponse
// 	err := retry.Do(func() error {
// 		var err error
// 		txResp, err = authTx.QueryTx(chain.getFullNode().CliContext(), txHash)
// 		return err
// 	},
// 		// retry for total of 3 seconds
// 		retry.Attempts(15),
// 		retry.Delay(200*time.Millisecond),
// 		retry.DelayType(retry.FixedDelay),
// 		retry.LastErrorOnly(true),
// 	)
// 	return txResp, err
// }

func TestLearn(t *testing.T) {

	t.Parallel()

	ctx := context.Background()

	// Chain Factory

	cf := interchaintest.NewBuiltinChainFactory(zaptest.NewLogger(t), []*interchaintest.ChainSpec{
		{Name: "juno", Version: "v15.0.0", ChainConfig: ibc.ChainConfig{
			GasPrices: "0.0uatom",
		}},
		{Name: "juno", Version: "v15.0.0", ChainConfig: ibc.ChainConfig{
			GasPrices: "0.0uatom",
		}},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)
	junoA, junoB := chains[0], chains[1]

	// Relayer Factory
	client, network := interchaintest.DockerSetup(t)
	r := interchaintest.NewBuiltinRelayerFactory(ibc.CosmosRly, zaptest.NewLogger(t)).Build(
		t, client, network)

	// Prep Interchain
	const ibcPath = "junoA-junoB-demo"
	ic := interchaintest.NewInterchain().
		AddChain(junoA).
		AddChain(junoB).
		AddRelayer(r, "relayer").
		AddLink(interchaintest.InterchainLink{
			Chain1:  junoA,
			Chain2:  junoB,
			Relayer: r,
			Path:    ibcPath,
		})

	// Log location
	f, err := interchaintest.CreateLogFile(fmt.Sprintf("%d.json", time.Now().Unix()))
	require.NoError(t, err)
	// Reporter/logs
	rep := testreporter.NewReporter(f)
	eRep := rep.RelayerExecReporter(t)

	// Build interchain
	require.NoError(t, ic.Build(ctx, eRep, interchaintest.InterchainBuildOptions{
		TestName:          t.Name(),
		Client:            client,
		NetworkID:         network,
		BlockDatabaseFile: interchaintest.DefaultBlockDatabaseFilepath(),

		SkipPathCreation: false},
	),
	)

	// Create and Fund User Wallets
	fundAmount := int64(10_000_000)
	users := interchaintest.GetAndFundTestUsers(t, ctx, "default", fundAmount, junoA, junoB)
	junoAUser := users[0]
	junoBUser := users[1]
	junoAUserAddr := junoAUser.Bech32Address(junoA.Config().Bech32Prefix)
	junoBUserAddr := junoBUser.Bech32Address(junoB.Config().Bech32Prefix)

	junoAUserBalInitial, err := junoA.GetBalance(ctx, junoAUserAddr, junoA.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, fundAmount, junoAUserBalInitial)

	junoBUserBalInitial, err := junoB.GetBalance(ctx, junoBUserAddr, junoB.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, fundAmount, junoBUserBalInitial)

	// Get Channel ID
	junoAChannelInfo, err := r.GetChannels(ctx, eRep, junoA.Config().ChainID)
	require.NoError(t, err)
	junoAChannelID := junoAChannelInfo[0].ChannelID

	junoBChannelInfo, err := r.GetChannels(ctx, eRep, junoB.Config().ChainID)
	require.NoError(t, err)
	junoBChannelID := junoBChannelInfo[0].ChannelID

	_ = junoBUser
	_ = junoAChannelID
	_ = junoBChannelID

	junoAChain := junoA.(*cosmos.CosmosChain)
	// junoBChain := junoB.(*cosmos.CosmosChain)

	// Deploy and instantiate factory
	msg := `{"default_governance_fee_share":"0"}`
	_, factoryContractAddr := SetupContract(t, ctx, junoAChain, junoAUserAddr, "contracts/catalyst_factory.wasm", msg)
	t.Log("Factory address: ", factoryContractAddr)

	// Deploy and instantiate interface
	msg = `{}`
	_, interfaceContractAddr := SetupContract(t, ctx, junoAChain, junoAUserAddr, "contracts/catalyst_ibc_interface.wasm", msg)
	t.Log("Interface Address: ", interfaceContractAddr)


	// Deploy test tokens
	tokenNames := [3]string{"TokenA", "TokenB", "TokenC"};
	tokenSymbols := [3]string{"TTA", "TTB", "TTC"};
	var tokenAddresses [3]string;

	for i := 0; i < len(tokenNames); i++ {

		// Deploy test tokens
		msg := fmt.Sprintf(`{"name":"%s","symbol":"%s","decimals":6,"initial_balances":[{"address":"%s","amount":"100000000"}],"mint":{"minter":"%s"}}`,tokenNames[i], tokenSymbols[i], junoAUserAddr, junoAUserAddr)
		_, tokenAddresses[i] = SetupContract(t, ctx, junoAChain, junoAUserAddr, "contracts/cw20_base.wasm", msg)
		t.Log("Token Address: ", tokenNames[i], tokenAddresses[i])

	}


	// Deploy vault
	vaultCodeId, err := junoAChain.StoreContract(ctx, junoAUserAddr, "contracts/catalyst_vault_volatile.wasm")
	if err != nil {
		t.Fatal(err)
	}
	t.Log(vaultCodeId)

	

	// Instantiate vault via factory

	// Set token allowance
	msg = fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"10000"}}`, factoryContractAddr)
	_, err = junoAChain.ExecuteContract(ctx, junoAUserAddr, tokenAddresses[0], msg)
	if err != nil {
		t.Fatal(err)
	}

	msg = fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"10000"}}`, factoryContractAddr)
	_, err = junoAChain.ExecuteContract(ctx, junoAUserAddr, tokenAddresses[1], msg)
	if err != nil {
		t.Fatal(err)
	}

	// Instantiate vault
	msg = fmt.Sprintf(`{"deploy_vault":{"vault_code_id":%s,"assets":["%s","%s"],"assets_balances":["10000","10000"],"weights":["1","1"],"amplification":"1000000000000000000","vault_fee":"0","name":"POOL","symbol":"POOL","chain_interface":"%s"}}`, vaultCodeId, tokenAddresses[0], tokenAddresses[1], interfaceContractAddr)
	t.Log("Deploy vault msg", msg)
	hash, err := junoAChain.ExecuteContract(ctx, junoAUserAddr, factoryContractAddr, msg)

	t.Log(hash)
	if err != nil {
		t.Fatal(err)
	}




	// codeId, err := junoBChain.StoreContract(ctx, junoBUser.Bech32Address(junoB.Config().Bech32Prefix), "contracts/catalyst_factory.wasm")
	// if err != nil {
	// 	t.Fatal(err)
	// }

	// _ = codeId;

	// contractAddr, err := junoAChain.InstantiateContract(ctx, keyname, codeId, message, true)
	// if err != nil {
	// 	t.Fatal(err)
	// }


	// // // // Send Transaction
	// // // amountToSend := int64(1_000_000)
	// // // dstAddress := junoBUser.FormattedAddress()
	// // // transfer := ibc.WalletAmount{
	// // // 	Address: dstAddress,
	// // // 	Denom:   junoA.Config().Denom,
	// // // 	Amount:  amountToSend,
	// // // }
	// // // tx, err := junoA.SendIBCTransfer(ctx, junoAChannelID, junoAUser.KeyName(), transfer, ibc.TransferOptions{})
	// // // require.NoError(t, err)
	// // // require.NoError(t, tx.Validate())

	// // // // relay MsgRecvPacket to junoB, then MsgAcknowledgement back to junoA
	// // // require.NoError(t, r.Flush(ctx, eRep, ibcPath, junoAChannelID))

	// // // // test source wallet has decreased funds
	// // // expectedBal := junoAUserBalInitial - amountToSend
	// // // junoAUserBalNew, err := junoA.GetBalance(ctx, junoAUser.FormattedAddress(), junoA.Config().Denom)
	// // // require.NoError(t, err)
	// // // require.Equal(t, expectedBal, junoAUserBalNew)

	// // // // Trace IBC Denom
	// // // srcDenomTrace := transfertypes.ParseDenomTrace(transfertypes.GetPrefixedDenom("transfer", osmoChannelID, junoA.Config().Denom))
	// // // dstIbcDenom := srcDenomTrace.IBCDenom()

	// // // // Test destination wallet has increased funds
	// // // osmosUserBalNew, err := junoB.GetBalance(ctx, junoBUser.FormattedAddress(), dstIbcDenom)
	// // // require.NoError(t, err)
	// // // require.Equal(t, amountToSend, osmosUserBalNew)


	// // Deploy contracts
	// junoBChain := junoB.(*cosmos.CosmosChain)
	// junoBFactoryCodeId, err := junoBChain.StoreContract(ctx, junoBUser.Address, "../target/wasm32-unknown-unknown/release/catalyst_factory.wasm")

	// _ = junoBFactoryCodeId
	// // _ = err
	// if err != nil {
	// 	t.Fatal(err)
	// }
	// // require.NoError(t, err);

	// // t.Logf("junoA Catalyst factory code id %s", junoAFactoryCodeId)

	
}
