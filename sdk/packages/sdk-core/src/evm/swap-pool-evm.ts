import { Contract, providers } from "ethers";
import { SwapPoolBase } from "../swap-pool";
import { SwapPoolContract } from "./contracts/contracts";
import { Provider } from "@ethersproject/abstract-provider";
import { Signer } from "@ethersproject/abstract-signer";

export class SwapPoolEVM implements SwapPoolBase {

    private contract: Contract;

    constructor (
        private poolAddress: string,
        private signerOrProvider: Signer | Provider
    ) {
        this.contract = new Contract(this.poolAddress, SwapPoolContract.abi, this.signerOrProvider);
    }

    // * State query transactions ***********************************************************************************************
    async isInitialized(): Promise<boolean> {
        return this.contract.ready();
    }

    async getDecimals(): Promise<bigint> {
        return this.contract.decimals();
    }

    async getPoolTokenBalance(holder: string): Promise<bigint> {
        return this.contract.balanceOf(holder);
    }

    async getPoolTokenAllowance(holder: string, allowee: string): Promise<bigint> {
        return this.contract.allowance(holder, allowee);
    }

    async getPoolTokenSupply(): Promise<bigint> {
        return this.contract.totalSupply();
    }

    async getUnitCapacity(): Promise<bigint> {
        return this.contract.getUnitCapacity();
    }

    async getLiquidityUnitCapacity(): Promise<bigint> {
        return this.contract.getLiquidityUnitCapacity();
    }


    // TODO get connected pools


    // * State changing transactions ********************************************************************************************
    // Administration
    async setup(): Promise<void> {

    }

    async finishSetup(): Promise<void> {

    }

    async createConnection(): Promise<void> {

    }


    // Pool liqudity
    async deposit(): Promise<void> {

    }

    async withdraw(): Promise<void> {

    }


    async transferPoolTokens(): Promise<void> {

    }

    async transferPoolTokensFrom(): Promise<void> {

    }

    async approvePoolTokenAllowance(): Promise<void> {

    }


    // Asset Swaps
    async localSwap(): Promise<void> {

    }

    async crossChainSwap(): Promise<void> {

    }


    // Liquidity Swaps
    async crossChainLiquiditySwap(): Promise<void> {

    }


} 