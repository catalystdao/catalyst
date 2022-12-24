import * as anchor from "@project-serum/anchor";
import math, { create, all } from 'mathjs';

// Create a MathJS instance with support of BigNumbers that have 320 significant digits
const largeMathjs = create(all);
largeMathjs.config({ number: 'BigNumber', precision: 320 });

const PRECISION = 64;
const BIG_ONE   = largeMathjs.bignumber(2).pow(PRECISION);


/** Convert a native JS bigint to a math.BigNumber. */
function bigIntToBigNumber(n: bigint): math.BigNumber {
    return largeMathjs.bignumber(n.toString());
}

/** Convert a math.BigNumber to a native JS bigint. */
function bigNumberToBigInt(n: math.BigNumber) {
    return BigInt(n.floor().toFixed(0));
}

/** Convert a native JS bigint fixed point representation to a math.BigNumber */
function fixedPointBigIntToBigNumber(n: bigint): math.BigNumber {
    return largeMathjs.bignumber(n.toString()).div(BIG_ONE);
}

/** Convert a math.BigNumber to a native JS bigint fixed point representation. 
 *  Returns undefined if conversion fails (e.g. NaN) 
 */
function bigNumberToFixedPointBigInt(n: math.BigNumber): bigint | undefined {
    try {
        return BigInt(n.times(BIG_ONE).floor().toFixed(0))
    }
    catch {
        return undefined;
    }
}

/** Return the relative error between two bigint numbers (always positive). If the target is 0, 
 *  any non-zero error returns infinity.
 */
export function calcRelativeBigIntsError(value: bigint, target: bigint): number {
    if (target === 0n) return value === target ? 0 : Infinity;

    return (
        largeMathjs.bignumber(1).minus(
            bigIntToBigNumber(value).div(bigIntToBigNumber(target))
        ).abs()
    ).toNumber();
}

/** Split a bigint (256 bit int representation) to an array of anchor.BN numbers. Each anchor.BN number 
 *  represents a slice of ${splitSize} bits of the original bigint.
 */
export function splitBigIntToNumbers(num: bigint, splitSize: number): anchor.BN[] {
    let splitSizeBigInt = BigInt(splitSize);
    let n = 256n/splitSizeBigInt;

    let bitMask = 2n**splitSizeBigInt - 1n; // 0xFFFF... ${splitSize} bits

    let split: anchor.BN[] = [];
    for (let i = 0n; i < n; i++) {
        split.push(new anchor.BN(((num >> splitSizeBigInt*i) & bitMask) as any))
    }

    return split;
}

/** Join an array of anchor.BN numbers to a single bigint. Each anchor.BN represents a number of ${splitSize} bits. */
export function joinNumbersToBigInt(split: anchor.BN[], splitSize: number): bigint {
    let splitSizeBigInt = BigInt(splitSize);

    return split.reduce(
        (acc, next, i) => acc | (BigInt(next.toString()) << (splitSizeBigInt * BigInt(i))),
        0n
    );
}



// Accurate mathematical calculations for testing of the fixed point math library ***********************************************

export function mul_x64(a: bigint, b: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(a).times(fixedPointBigIntToBigNumber(b))
    );
}

export function div_x64(a: bigint, b: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(a).div(fixedPointBigIntToBigNumber(b))
    );
}

export function log2_x64(n: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(n).log(2)
    );
}

export function ln_x64(n: bigint): bigint | undefined {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(n).ln()
    );
}

export function pow2_x64(x: bigint): bigint | undefined {
    return bigNumberToFixedPointBigInt(
        largeMathjs.bignumber(2).pow(fixedPointBigIntToBigNumber(x))
    );
}

export function inv_pow2_x64(x: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        largeMathjs.bignumber(2).pow(fixedPointBigIntToBigNumber(x).times(-1))
    );
}

export function pow_x64(x: bigint, p: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(x).pow(fixedPointBigIntToBigNumber(p))
    );
}

export function inv_pow_x64(x: bigint, p: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(x).pow(fixedPointBigIntToBigNumber(p).times(-1))
    );
}

export function exp_x64(p: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(p).exp()
    )
}

export function inv_exp_x64(p: bigint): bigint {
    return bigNumberToFixedPointBigInt(
        fixedPointBigIntToBigNumber(p).times(-1).exp()
    )
}