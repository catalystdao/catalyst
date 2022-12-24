import { create, all, BigNumber } from 'mathjs';

const mathjs = create(all);
mathjs.config({
    number: 'BigNumber',
    precision: 320
});

function computeTwoTwoMinusI(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(mathjs.bignumber(2).pow(-(i+1))).times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}

function computeTwoMinusTwoMinusI(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(mathjs.bignumber(2).pow(-(i+1)).times(-1)).times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}

function computeExpPos(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(i).exp().times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}

function computeExpNeg(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(-(i+1)).exp().times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}


function computeInvExpPos(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(i).times(-1).exp().times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}

function computeInvExpNeg(elCount: number) {
    return [...Array(elCount).keys()]
        .map(i => {
            return mathjs.bignumber(2).pow(-(i+1)).times(-1).exp().times(mathjs.bignumber(2).pow(64))
        })
        .map(bn => bn.round())
}

function bigNumberArrToUint256StructArr(bNArr: BigNumber[]): string {
    return bNArr
        .map(bN => bigNumberToUint64Arr(bN))
        .map(numArr => `U256([${numArr.join(', ')}])`)
        .join(', ')
}

function bigNumberToUint64Arr(bN: BigNumber): number[] {
    let bigint = BigInt(bN.toFixed(0));
    return [0n, 1n, 2n, 3n]
        .map(i => {
            return (bigint >> (64n*i)) & 0xFFFFFFFFFFFFFFFFn
        })
        .map(bigintNum => Number(bigintNum))
}

function formatAndLogValues(bNArr: BigNumber[]): void {
    console.log("");
    console.log("Int Array");
    console.log(`[${bNArr.map(bN => bN.toFixed(0)).join(', ')}]`);
    console.log("");
    console.log("U256 Array");
    console.log(`[${bigNumberArrToUint256StructArr(bNArr)}]`)
}




let twoTwoMinusI      = computeTwoTwoMinusI(20);
let twoMinusTwoMinusI = computeTwoMinusTwoMinusI(20);

let expPos            = computeExpPos(9);
let expNeg            = computeExpNeg(20)

let invExpPos         = computeInvExpPos(8);
let invExpNeg         = computeInvExpNeg(20);


// Select here which lookup table to print out
formatAndLogValues(twoTwoMinusI);
