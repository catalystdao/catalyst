package cosmwasmtesting

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/cosmos/cosmos-sdk/crypto/keyring"
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


type TestChainConfig struct {
	Chain *cosmos.CosmosChain
	// ChannelID string
	Users []*ibc.Wallet
	UsersAddresses []string
	Relayer *ibc.Relayer
}

type ResponseAttribute struct {
	Key string
	Value string
}

type ResponseEvent struct {
	Type string
	Attributes []ResponseAttribute
}

type ResponseLog struct {
	Msg_index int
	Log string
	Events []ResponseEvent
}

type Response struct {
	Height string
	Txhash string
	Codespace string
	Code int
	Data string
	Logs []ResponseLog
}

type TestEnv struct {
	Chains []TestChainConfig
	Relayer ibc.Relayer
	IBCPath string
	RelayerReporter testreporter.RelayerExecReporter
}

func InitializeTestEnv(t *testing.T, ctx context.Context) TestEnv {

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
		// BlockDatabaseFile: interchaintest.DefaultBlockDatabaseFilepath(),

		SkipPathCreation: false},
	),
	)

	// Create and Fund User Wallets
	fundAmount := int64(10_000_000)
	users := interchaintest.GetAndFundTestUsers(t, ctx, "default", fundAmount, junoA, junoA, junoB, junoB)
	junoAUser1 := users[0]
	junoAUser2 := users[1]
	junoBUser1 := users[2]
	junoBUser2 := users[3]
	junoAUser1Addr := junoAUser1.Bech32Address(junoA.Config().Bech32Prefix)
	junoAUser2Addr := junoAUser2.Bech32Address(junoA.Config().Bech32Prefix)
	junoBUser1Addr := junoBUser1.Bech32Address(junoB.Config().Bech32Prefix)
	junoBUser2Addr := junoBUser2.Bech32Address(junoB.Config().Bech32Prefix)

	// junoAUserBalInitial, err := junoA.GetBalance(ctx, junoAUserAddr, junoA.Config().Denom)
	// require.NoError(t, err)
	// require.Equal(t, fundAmount, junoAUserBalInitial)

	// junoBUserBalInitial, err := junoB.GetBalance(ctx, junoBUserAddr, junoB.Config().Denom)
	// require.NoError(t, err)
	// require.Equal(t, fundAmount, junoBUserBalInitial)

	// Get Channel ID
	junoAChannelInfo, err := r.GetChannels(ctx, eRep, junoA.Config().ChainID)
	require.NoError(t, err)
	junoAChannelID := junoAChannelInfo[0].ChannelID

	junoBChannelInfo, err := r.GetChannels(ctx, eRep, junoB.Config().ChainID)
	require.NoError(t, err)
	junoBChannelID := junoBChannelInfo[0].ChannelID

	_ = junoAChannelID
	_ = junoBChannelID

	junoAConfig := TestChainConfig{
		Chain: junoA.(*cosmos.CosmosChain),
		// ChannelID: junoAChannelID,
		Users: []*ibc.Wallet{junoAUser1, junoAUser2},
		UsersAddresses: []string{junoAUser1Addr, junoAUser2Addr},
	}

	junoBConfig := TestChainConfig{
		Chain: junoB.(*cosmos.CosmosChain),
		// ChannelID: junoBChannelID,
		Users: []*ibc.Wallet{junoBUser1, junoBUser2},
		UsersAddresses: []string{junoBUser1Addr, junoBUser2Addr},
	}

	require.NoError(t, r.StartRelayer(ctx, eRep, ibcPath))

	return TestEnv{
		Chains: []TestChainConfig{junoAConfig, junoBConfig},
		Relayer: r,
		IBCPath: ibcPath,
		RelayerReporter: *eRep,
	}

}

func DeployAndInstantiateFactory(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
) string {

	msg := `{"default_governance_fee_share":"0"}`
	_, factoryContractAddr := SetupContract(t, ctx, chain, account, "contracts/catalyst_factory.wasm", msg)

	t.Log("Factory address: ", factoryContractAddr)

	return factoryContractAddr
}

func DeployAndInstantiateInterfacePoA(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
) string {

	msg := `{}`
	_, interfaceContractAddr := SetupContract(t, ctx, chain, account, "contracts/catalyst_ibc_interface_poa.wasm", msg)

	t.Log("Interface address: ", interfaceContractAddr)

	return interfaceContractAddr
}

func DeployAndInstantiateInterface(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
) string {

	msg := `{}`
	_, interfaceContractAddr := SetupContract(t, ctx, chain, account, "contracts/catalyst_ibc_interface.wasm", msg)

	t.Log("Interface address: ", interfaceContractAddr)

	return interfaceContractAddr
}

func DeployAndInstantiateTestTokens(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
) []string {
	
	// Deploy test tokens
	tokenNames := [2]string{"TokenA", "TokenB"};
	tokenSymbols := [2]string{"TTA", "TTB"};
	var tokenAddresses []string;

	for i := 0; i < len(tokenNames); i++ {

		// Deploy test tokens
		msg := fmt.Sprintf(`{"name":"%s","symbol":"%s","decimals":6,"initial_balances":[{"address":"%s","amount":"100000000"}],"mint":{"minter":"%s"}}`,tokenNames[i], tokenSymbols[i], account, account)
		_, tokenAddress := SetupContract(t, ctx, chain, account, "contracts/cw20_base.wasm", msg)

		tokenAddresses = append(tokenAddresses, tokenAddress)

		t.Log("Token Address: ", tokenNames[i], tokenAddresses[i])

	}

	return tokenAddresses
}

func StoreVolatileVault(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
) string {
	vaultCodeId, err := chain.StoreContract(ctx, account, "contracts/catalyst_vault_volatile.wasm")
	if err != nil {
		t.Fatal(err)
	}
	t.Log("Volatile vault code id: ", vaultCodeId)

	return vaultCodeId
}

func ExecuteCmd(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
	contractAddr string,
	msg string,
) (stdout []byte, stderr []byte) {

	cmd := []string{chain.Config().Bin, "tx", "wasm", "execute", contractAddr, msg,
		"--keyring-backend", keyring.BackendTest,
		"--node", chain.GetRPCAddress(),
		"--from", account,
		"--gas", "50000000",
		"--gas-prices", chain.Config().GasPrices,
		"--home", chain.HomeDir(),
		"--chain-id", chain.Config().ChainID,
		"--output", "json",
		"-b", "block",
		"-y",
	}

	stdout, stderr, err := chain.Exec(ctx, cmd, nil)
	require.NoError(t, err)

	return stdout, stderr
}

func DeployVault(
	t *testing.T,
	ctx context.Context,
	chain *cosmos.CosmosChain,
	account string,
	tokens []string,
	factoryContractAddr string,
	interfaceContractAddr string,
	vaultCodeId string,
) string {

	// Instantiate vault via factory

	// Set token allowance
	msg := fmt.Sprintf(`{"increase_allowance":{"spender":"%s","amount":"10000"}}`, factoryContractAddr)
	for i := 0; i < len(tokens); i++ {
		_, err := chain.ExecuteContract(ctx, account, tokens[i], msg)
		if err != nil {
			t.Fatal(err)
		}
	}

	// Instantiate vault
	msg = fmt.Sprintf(`{"deploy_vault":{"vault_code_id":%s,"assets":["%s","%s"],"assets_balances":["10000","10000"],"weights":["1","1"],"amplification":"1000000000000000000","vault_fee":"0","name":"POOL","symbol":"POOL","chain_interface":"%s"}}`, vaultCodeId, tokens[0], tokens[1], interfaceContractAddr)

	stdout, _ := ExecuteCmd(
		t,
		ctx,
		chain,
		account,
		factoryContractAddr,
		msg,
	)

	var response Response

	json.Unmarshal([]byte(stdout), &response)

	vault_address := response.Logs[0].Events[5].Attributes[4].Value

	t.Log("Vault address: ", vault_address)

	return vault_address

}

func EncodeAddress(
	t *testing.T,
	address string,
) string {

	addressBytes := []byte(address)


	addressLength := len(addressBytes)

	//TODO check addressLenght is less than 65 or return error

	encodedBytes := []byte{byte(addressLength)}
	encodedBytes = append(encodedBytes, make([]byte, 64 - addressLength)...)
	encodedBytes = append(encodedBytes, addressBytes...)

	encodedString := base64.StdEncoding.EncodeToString(encodedBytes)

	t.Log("Encoded address: ", encodedString)

	return encodedString
}

