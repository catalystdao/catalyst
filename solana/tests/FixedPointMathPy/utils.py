from typing import List
from anchorpy import Context, Program, Provider

from solana.publickey import PublicKey
from solana.rpc.commitment import Commitment, Confirmed, Processed

from solders.signature import Signature
from solders.rpc.responses import GetSignatureStatusesResp

import fixed_point_math                 # type: ignore
from integer import Uint256             # type: ignore


U64_MAX  = 2**64-1
U256_MAX = 2**256-1


def u256_array_to_int(array: List[int]) -> int:
    return array[0] + (array[1]<<64) + (array[2]<<128) + (array[3]<<192)


def int_to_u256_array(value: int) -> List[int]:
    return [
        value & U64_MAX,
        (value >> 64) & U64_MAX,
        (value >> 128) & U64_MAX,
        (value >> 192) & U64_MAX,
    ]


async def confirm_transaction(provider: Provider, tx: Signature, commitment: Commitment = Confirmed) -> GetSignatureStatusesResp:

    confirmation = await provider.connection.confirm_transaction(tx, commitment=commitment)

    try:
        confirmation_error = None if confirmation.value[0] is None else confirmation.value[0].err
    except:
        raise Exception
        
    if confirmation_error is not None:
        raise Exception(confirmation_error)

    return confirmation


async def handle_math_request(program: Program, result_account: PublicKey, method_name: str, args: List[int]) -> int | None:

    tx_result = await program.rpc[method_name](
        *[int_to_u256_array(arg) for arg in args],
        ctx=Context(
            accounts={
                "calculation_data": result_account
            }
        )
    )

    try:
        await confirm_transaction(program.provider, tx_result, commitment=Processed)
    except:
        return None

    result_account_data = await program.account["CalculationData"].fetch(result_account, commitment=Processed)

    return u256_array_to_int(result_account_data.result)


async def verify_program_result(fn_name, args: List[int], math_program, result_account):

    program_result = await handle_math_request(math_program, result_account, fn_name, args)

    compare_fn = getattr(fixed_point_math, fn_name)

    try:
        expected_result = compare_fn(*[Uint256(arg) for arg in args])
    except:
        expected_result = None

    if program_result is None:
        assert expected_result is None
    else:
        assert expected_result is not None
        assert program_result == expected_result.value