
export abstract class SwapPoolBase {

    // * State query transactions ***********************************************************************************************
    abstract isInitialized(): Promise<boolean>;
    abstract getDecimals(): Promise<bigint>;
    abstract getPoolTokenBalance(holder: string): Promise<bigint>;
    abstract getPoolTokenAllowance(holder: string, allowee: string): Promise<bigint>;
    abstract getPoolTokenSupply(): Promise<bigint>;
    abstract getUnitCapacity(): Promise<bigint>;
    abstract getLiquidityUnitCapacity(): Promise<bigint>;

    // TODO get connected pools


    // * State changing transactions ********************************************************************************************
    // Administration
    abstract setup(): Promise<void>;
    abstract finishSetup(): Promise<void>;
    abstract createConnection(): Promise<void>;

    // Pool liqudity
    abstract deposit(): Promise<void>;
    abstract withdraw(): Promise<void>;

    abstract transferPoolTokens(): Promise<void>;
    abstract transferPoolTokensFrom(): Promise<void>;
    abstract approvePoolTokenAllowance(): Promise<void>;

    // Asset Swaps
    abstract localSwap(): Promise<void>;
    abstract crossChainSwap(): Promise<void>;

    // Liquidity Swaps
    abstract crossChainLiquiditySwap(): Promise<void>;
}