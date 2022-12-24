import { Program, web3, workspace } from "@project-serum/anchor";
import { FixedPointMath } from "../../../target/types/fixed_point_math";
import { assert } from "chai";
import { splitBigIntToNumbers, joinNumbersToBigInt, calcRelativeBigIntsError } from "./math-utils";

const deployedProgramData = deployProgram();

export interface FixedPointMathDeployedProgram {
    program: Program<FixedPointMath>;
    calculationDataKeypair: web3.Keypair;
}

export async function deployProgram(): Promise<FixedPointMathDeployedProgram> {
    const calculationDataKeypair = web3.Keypair.generate();

    const program = workspace.FixedPointMath as Program<FixedPointMath>;
    const deployer = program.provider.wallet;

    await program.rpc.initialize({
        accounts: {
            calculationData: calculationDataKeypair.publicKey,
            deployer: deployer.publicKey,
            systemProgram: web3.SystemProgram.programId
        },
        signers: [calculationDataKeypair]
    });

    return {
        program,
        calculationDataKeypair
    };
}

export async function handle_math_request(method_name: string, args: bigint[]): Promise<bigint | undefined> {
    const { program, calculationDataKeypair } = await deployedProgramData;

    try {
        await program.rpc[method_name](
            ...args.map(arg => splitBigIntToNumbers(arg, 64)),
            {
                accounts: {
                    calculationData: calculationDataKeypair.publicKey,
                }
            }
        )
    }
    catch {
        return undefined;
    }

    const calculation_data_state = await program.account.calculationData.fetch(calculationDataKeypair.publicKey, 'recent');

    return joinNumbersToBigInt(calculation_data_state.result, 64);
}

export async function testMathRequest(
    methodName: string,
    args: bigint[],
    expectedResult: bigint,
    maxError: number,
    onSuccessChecks?: (result: bigint) => void,
    onFailureChecks?: () => void
): Promise<void> {

    let result = await handle_math_request(methodName, args)

    if (result === undefined) {
        assert(expectedResult === undefined, `Function failed for an expected valid output ${expectedResult}.`);
        onFailureChecks?.();
    }
    else {
        assert(expectedResult !== undefined, `Function did not fail for an expected invalid output, output: ${result}.`)
        onSuccessChecks?.(result);

        let error = calcRelativeBigIntsError(result, expectedResult);
        assert(
            error <= maxError,
            `Function output ${result} does not match expected result ${expectedResult}, error ${error}`
        );
    }
}