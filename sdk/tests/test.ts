import { ContractFactory, ethers, Signer, Wallet, providers, Wordlist, BigNumber } from "ethers";
import { join } from "path";

import { evm } from '@catalabs/sdk-core/';

const SwapPoolEVM = evm.SwapPoolEVM;


import { deployTestToken } from './test-token/test-token';
import { numberToBytes32, getPolymerChainDevnetRootDir, loadPolymerChainConfig, createWalletFromMnemonic } from "./utils";
import { formatBytes32String } from "ethers/lib/utils";
import { exec } from "child_process";

const provider_eth = new providers.JsonRpcProvider('http://localhost:10000');
const provider_bsc = new providers.JsonRpcProvider('http://localhost:10001');

async function deployPolymerChainInterface(
    swapPoolFactoryAddress: string,
    polymeraseDispatcherAddress: string,
    signer: Signer
): Promise<ethers.Contract> {
    return deployContract(
        evm.IBCInterfaceContract.abi,
        evm.IBCInterfaceContract.bytecode,
        signer,
        [
            swapPoolFactoryAddress,
            polymeraseDispatcherAddress,
            numberToBytes32(0)
        ]
    );
}

async function deploySwapPoolFactory(
    swapPoolAddress: string,
    swapPoolAmplifiedAddress: string,
    signer: Signer
): Promise<ethers.Contract> {
    return deployContract(
        evm.SwapPoolFactoryContract.abi,
        evm.SwapPoolFactoryContract.bytecode,
        signer,
        [swapPoolAddress, swapPoolAmplifiedAddress]
    );
}

async function deploySwapPool(signer: Signer): Promise<ethers.Contract> {
    return deployContract(evm.SwapPoolContract.abi, evm.SwapPoolContract.bytecode, signer);
}

async function deploySwapPoolAmplified(signer: Signer): Promise<ethers.Contract> {
    return deployContract(evm.SwapPoolAmplifiedContract.abi, evm.SwapPoolAmplifiedContract.bytecode, signer);
}

async function deployContract(
    contractInterface : ethers.ContractInterface,
    bytecode          : ethers.utils.BytesLike,
    signer            : Signer,
    contractArgs      : any[] = [],
    deployOverrides   : any = {}
): Promise<ethers.Contract> {
    return new ContractFactory(contractInterface, bytecode, signer).deploy(...contractArgs, deployOverrides);
}

async function initializeSwapPool(
    tokenAddresses: string[],
    tokenWeights: number[],
    tokenBalances: number[],
    name: string,
    symbol: string,
    swapPoolFactoryAddress: string,
    IBCInterfaceAddress: string,
    signer: Signer
): Promise<string> {

    if (
        tokenAddresses.length != tokenWeights.length ||
        tokenAddresses.length != tokenBalances.length
    ) {
        throw new Error("Invalid pool initializations parameters. Token list length does not match with the provided weights/balances.");
    }

    // Give token allowances to the swapPool
    for (let i = 0; i < tokenAddresses.length; i++) {
        const tokenContract = new ethers.Contract(  // TODO erc20 class?
            tokenAddresses[i],
            ["function approve(address spender, uint256 amount) returns (bool)"],
            signer
        );

        const response = await tokenContract.approve(swapPoolFactoryAddress, tokenBalances[i]);
        await response.wait();
    }

    // Invoke factory deploy
    const factoryContract = new ethers.Contract( //TODO factory class?
        swapPoolFactoryAddress,
        evm.SwapPoolFactoryContract.abi,
        signer
    );

    const deploy_response = await factoryContract.deploy_swappool(
        IBCInterfaceAddress,
        0,  // poolTemplateIndex => 0 ==    //TODO enum?
        tokenAddresses,
        tokenBalances,
        tokenWeights,
        BigNumber.from(2).pow(64), // amplification => 1
        name,
        symbol,
        {gasLimit: 20000000}
    )

    // Wait for tx to be mined
    const deploy_tx = await deploy_response.wait();

    // Get the deployed pool address from the 'PoolDeployed' Event
    const filtered_events = deploy_tx.events.filter((event: any) => event.event === "PoolDeployed");
    if (filtered_events.length > 1) {
        throw new Error("Found multiple 'PoolDeployed' events on swap pool deploy (expected only 1).");
    }

    return filtered_events[0].args.pool_address;

}

interface DeployCatalystResut {
    swapPoolAddress: string,
    swapPoolAmplifiedAddress: string,
    swapPoolFactoryAddress: string,
    IBCInterfaceAddress: string
}

async function deployCatalyst(
    dispatcherAddress : string,
    signer            : Signer
): Promise<DeployCatalystResut> {
    
    // Deploy swap pool contracts ( and amplified)
    const swapPoolContract          = await deploySwapPool(signer);
    const swapPoolAmplifiedContract = await deploySwapPoolAmplified(signer);

    const [swapPoolAddress, swapPoolAmplifiedAddress] = (
        await Promise.all([
            swapPoolContract.deployTransaction.wait(),
            swapPoolAmplifiedContract.deployTransaction.wait()
        ])
    ).map(tx => tx.contractAddress)


    // Deploy swap pool factory
    const swapPoolFactoryContract = await deploySwapPoolFactory(
        swapPoolAddress,
        swapPoolAmplifiedAddress,
        signer
    );

    const swapPoolFactoryAddress = (await swapPoolFactoryContract.deployTransaction.wait()).contractAddress;


    // Deploy Polymer Chain IBC interface
    const IBCInterface = await deployPolymerChainInterface(
        swapPoolFactoryAddress,
        dispatcherAddress,
        signer
    );

    const IBCInterfaceAddress = (await IBCInterface.deployTransaction.wait()).contractAddress;

    return {
        swapPoolAddress,
        swapPoolAmplifiedAddress,
        swapPoolFactoryAddress,
        IBCInterfaceAddress
    }
}

async function main() {

    const polymerChainDevnetRootDir = await getPolymerChainDevnetRootDir();

    if (polymerChainDevnetRootDir == null) {
        throw Error("Unable to locate the polymer chain devnet root dir.");
    }

    // let ethConfig = await loadPolymerChainConfig(join(polymerChainDevnetRootDir, 'eth'));
    // if (ethConfig == undefined) throw Error("Unable to load the eth chain config file.")

    // let bscConfig = await loadPolymerChainConfig(join(polymerChainDevnetRootDir, 'bsc'));
    // if (bscConfig == undefined) throw Error("Unable to load the bsc chain config file.")

    // // Deploy Catalyst on ETH
    // const signerETH: Signer = createWalletFromMnemonic(
    //     ethConfig.accounts[0].mnemonic.phrase,
    //     undefined,
    //     undefined,
    //     provider_eth
    // );
    // const ethContracts = await deployCatalyst(ethConfig.dispatcher.address, signerETH);

    // // Deploy Catalyst on BSC
    // const signerBSC: Signer = createWalletFromMnemonic(
    //     bscConfig.accounts[0].mnemonic.phrase,
    //     undefined,
    //     undefined,
    //     provider_bsc
    // );
    // const bscContracts = await deployCatalyst(bscConfig.dispatcher.address, signerBSC);
    
    // // Create pool on ETH
    // // Deploy test tokens
    // const poolTokensEth = [
    //     (await deployTestToken('one', 'I', signerETH)).contractAddress,
    //     (await deployTestToken('two', 'II', signerETH)).contractAddress
    // ]

    // // Initialize swap pool
    // const poolAddressEth = await initializeSwapPool(
    //     poolTokensEth,
    //     [1, 1], 
    //     [1000, 1000],
    //     "eth_pool",
    //     "ep",
    //     ethContracts.swapPoolFactoryAddress,
    //     ethContracts.IBCInterfaceAddress,
    //     signerETH
    // )
    
    // // Create pool on BSC
    // // Deploy test tokens
    // const poolTokensBsc = [
    //     (await deployTestToken('one', 'I', signerBSC)).contractAddress,
    //     (await deployTestToken('two', 'II', signerBSC)).contractAddress
    // ]

    // // Initialize swap pool
    // const poolAddressBsc = await initializeSwapPool(
    //     poolTokensBsc,
    //     [1, 1], 
    //     [1000, 1000],
    //     "bsc_pool",
    //     "bp",
    //     bscContracts.swapPoolFactoryAddress,
    //     bscContracts.IBCInterfaceAddress,
    //     signerBSC
    // )


    // // Initialize IBC channel
    // const interfaceETHContract = new ethers.Contract(
    //     ethContracts.IBCInterfaceAddress,
    //     evm.IBCInterfaceContract.abi,
    //     signerETH
    // );

    // await (await interfaceETHContract.registerPort()).wait();
    // await (await interfaceETHContract.setChannelForChain(1234, formatBytes32String("channel-0"))).wait();


    // const interfaceBSCContract = new ethers.Contract(
    //     bscContracts.IBCInterfaceAddress,
    //     evm.IBCInterfaceContract.abi,
    //     signerBSC
    // );

    // await (await interfaceBSCContract.registerPort()).wait();
    // await (await interfaceBSCContract.setChannelForChain(1234, formatBytes32String("channel-0"))).wait();


    // // Connect catalyst
    // const swapPoolETHContract = new ethers.Contract(
    //     ethContracts.swapPoolAddress,
    //     evm.SwapPoolContract.abi,
    //     signerETH
    // );
    // // console.log(bscContracts.swapPoolAddress.replace("0x", ""))
    // await (await swapPoolETHContract.createConnectionWithChain(1234, bscContracts.swapPoolAddress.replace("0x", ""), true)).wait()

    // const swapPoolBSCContract = new ethers.Contract(
    //     bscContracts.swapPoolAddress,
    //     evm.SwapPoolContract.abi,
    //     signerBSC
    // );
    // await (await swapPoolBSCContract.createConnectionWithChain(1234, ethContracts.swapPoolAddress.replace("0x", ""), true)).wait()

    // wait for channels
    async function checkChannelState(rootDir: string): Promise<boolean> {
        return new Promise(r => {
            exec('polymerased --home ' + rootDir + ' --node http://127.0.0.1:11000 --chain-id polymerase --output json q ibc channel channels | jq \'.channels\[\] | select(.channel_id == "channel-0" and .state == "STATE_OPEN") | \[.\] | length\'', (err, stdout, stderr) => r(stdout === "1\n"));
        });
    }

    console.log(await checkChannelState(polymerChainDevnetRootDir))


}

main();

