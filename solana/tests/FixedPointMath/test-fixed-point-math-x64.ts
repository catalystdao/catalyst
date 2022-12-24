import { setProvider, AnchorProvider } from "@project-serum/anchor";
import { testMathRequest } from "./utils/utils";
import {
  mul_x64,
  div_x64,
  log2_x64,
  ln_x64,
  pow2_x64,
  inv_pow2_x64,
  exp_x64,
  inv_exp_x64,
  pow_x64,
  inv_pow_x64
} from "./utils/math-utils";
import { generate_tuples, TEST_VALUES, SMALL_TEST_VALUES } from "./utils/test-ranges";

var itParam = require('mocha-param');


const MAX_U256 = 2n**256n-1n;


describe("FixedPointMath", () => {

  setProvider(AnchorProvider.env());


  itParam(
    "Test multiplication ${value}",
    generate_tuples(TEST_VALUES, TEST_VALUES),
    async ([a, b]: [bigint, bigint]) => {

      const expectedResult = mul_x64(a, b);
      const expectedValid  = expectedResult <= MAX_U256;
      const maxRelError    = 0;

      await testMathRequest('mulX64', [a, b], expectedValid ? expectedResult : undefined, maxRelError);
    
    }
  );


  itParam(
    "Test division ${value}",
    generate_tuples(TEST_VALUES, TEST_VALUES),
    // [[2n**(256n-64n), 2n**64n]], //IMPORTANT: Falsifying example, comment line above and uncomment this line to try
    async ([a, b]: [bigint, bigint]) => {  

      const expectedResult = div_x64(a, b);
      // const expectedValid  = b !== 0n && expectedResult <= MAX_U256; //TODO Desired range
      const expectedValid  =
        b !== 0n && 
        a < 2n**(256n-64n) && 
        expectedResult <= MAX_U256;
      const maxRelError    = 0;

      await testMathRequest('divX64', [a, b], expectedValid ? expectedResult : undefined, maxRelError)
    
    }
  );


  itParam(
    "Test log2 ${value}",
    TEST_VALUES,
    // [2n**64n+2n**60n],
    async (a: bigint) => {

      const expectedResult = log2_x64(a);
      const expectedValid  = a >= 2n**64n; // Input must be larger or equal to one (else result is a negative value)
      const maxRelError    = 1e-10;

      await testMathRequest('log2X64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test ln ${value}",
    TEST_VALUES,
    async (a: bigint) => {

      const expectedResult = ln_x64(a);
      const expectedValid  = a >= 2n**64n;
      const maxRelError    = 1e-10;

      await testMathRequest('lnX64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test power of 2 ${value}",
    SMALL_TEST_VALUES,
    async (a: bigint) => {

      const expectedValid  = a < 192n*2n**64n;
      const expectedResult = !expectedValid ? 0n : pow2_x64(a);
      const maxRelError    = 1e-5;

      await testMathRequest('pow2X64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test inverse power of 2 ${value}",
    TEST_VALUES,
    async (a: bigint) => {

      const expectedValid  = (a >> 64n) < 41n;
      const expectedResult = inv_pow2_x64(a);
      const maxRelError    = 1e-5;

      await testMathRequest('invPow2X64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test power ${value}",
    generate_tuples(
      TEST_VALUES,
      [0n, 1n, 2n**64n, 2n**64n*3n, 2n**64n*150n]
    ),
    async ([a, p]: [bigint, bigint]) => {

      const expectedResult = pow_x64(a, p);
      const expectedValid  = a >= 2n**64n &&
                             (mul_x64(log2_x64(a), p) >> 64n) < 192n &&
                             expectedResult <= MAX_U256;
      const maxRelError    = 1e-5;

      await testMathRequest('powX64', [a, p], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test inverse power ${value}",
    generate_tuples(
      TEST_VALUES,
      [0n, 1n, 2n**64n, 2n**64n*3n, 2n**64n*150n]
    ),
    async ([a, p]: [bigint, bigint]) => {

      const expectedResult = inv_pow_x64(a, p);
      const expectedValid  = a >= 2n**64n && 
                             (mul_x64(log2_x64(a), p) >> 64n) < 41 && //TODO is this condition checked with the test arguments?
                             expectedResult <= MAX_U256;
      const maxRelError    = 1e-5;

      await testMathRequest('invPowX64', [a, p], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test exponential ${value}",
    [
      ...SMALL_TEST_VALUES,
      80n*2n**64n,
      130n*2n**64n,
      134n*2n**64n,
      134n*2n**64n+1n,
      2n**(64n+8n), 2n**(64n+9n), 2n**(64n+10n)
    ],
    async (a: bigint) => {

      const expectedValid  = a < 134n*2n**64n; //TODO verify range
      const expectedResult = expectedValid ? exp_x64(a) : 0n;
      const maxRelError    = 1e-5;

      await testMathRequest('expX64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  itParam(
    "Test inverse exponential ${value}",
    SMALL_TEST_VALUES,
    async (a: bigint) => {

      const expectedValid  = a <= 2n**(64n+5n); //TODO verify range
      const expectedResult = !expectedValid ? 0n : inv_exp_x64(a);
      const maxRelError    = 1e-6;

      await testMathRequest('invExpX64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


  it(
    "Test safe power ${value}",
    async () => {
      throw new Error('Safe power test not implemented!');

      // const expectedValid  = true; //TODO range
      // const expectedResult = !expectedValid ? 0n : inv_exp_x64(a);
      // const maxRelError    = 1e-5;

      // await testMathRequest('powX64', [a], expectedValid ? expectedResult : undefined, maxRelError)

    }
  );


})
