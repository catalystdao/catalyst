import { ethers, Signer, Wallet, providers, Wordlist } from "ethers";
import { readdir, readFile, stat } from "node:fs/promises";
import { join } from "path";

import { defaultPath, HDNode } from "ethers/lib/utils";


export interface PolymerChainChainConfig {
    dispatcher: {address: string},
    accounts: {alias: string, mnemonic: {phrase: string, path: string, locale: string}, balance: number}[]
}

export function numberToBytes32(val: number): string {
    return ethers.utils.hexZeroPad(ethers.utils.hexlify(val), 32)
}

// Load deployed pool address from tmp
export async function getPolymerChainDevnetRootDir(): Promise<string | null> {

    const rootDir = '/tmp';

    let devnetFolders = await readdir(rootDir).then(
        folders => folders.filter(name => name.slice(0, 15) === 'polymer-devnet-')
    );

    if (devnetFolders.length == 0) {
        return null;
    }

    let devnetFoldersTime = await Promise.all(
        devnetFolders.map(folderName => stat(join(rootDir, folderName)).then(p => p.birthtime))
    )

    let devnetFolder = devnetFolders[
        devnetFoldersTime.indexOf(devnetFoldersTime.reduce((cTime, nTime) => nTime > cTime ? nTime : cTime))
    ];

    return join(rootDir, devnetFolder);
}

export async function loadPolymerChainConfig(rootDir: string): Promise<PolymerChainChainConfig | undefined> {

    let configPath = join(rootDir, 'chain-config.json');
    
    try {
        return JSON.parse(
            await readFile(configPath,'utf8')
        );
    }
    catch {
        return undefined;
    }

}

export function createWalletFromMnemonic(mnemonic: string, path?: string, wordlist?: Wordlist, provider?: providers.JsonRpcProvider): Signer {
    if (!path) { path = defaultPath; }
    return new Wallet(HDNode.fromMnemonic(mnemonic, undefined, wordlist).derivePath(path), provider);    
}
