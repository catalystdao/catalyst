import { Contract, ContractFactory, Signer } from "ethers";
import { TransactionReceipt } from "@ethersproject/abstract-provider"
import PolymerTokenJSON from './PolymerToken.json';

export async function deployTestToken(name: string, symbol: string, signer: Signer, decimals: number=18, supply: number=10000000): Promise<TransactionReceipt> {

    const contract = await new ContractFactory(
        PolymerTokenJSON.abi,
        PolymerTokenJSON.bytecode,
        signer
    ).deploy(name, symbol, decimals, supply);

    return contract.deployTransaction.wait();
}