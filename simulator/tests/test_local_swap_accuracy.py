# Assess the impact the input balance amount of a local swap has on its accuracy

from integer import Uint256
import matplotlib.pyplot as plt

from swap_calculation_helpers import (
    full_swap_binomial_approx_i,
    full_swap_f,
    full_swap_i,
    full_swap_uniform_approx_i,
    in_swap_uniform_approx_i,
    out_swap_f,
    out_swap_i_x64,
    out_swap_uniform_approx_i_x64,
)


def get_rel_error(val: float, target: float) -> float:

    if val is None:
        return -2

    if val == 0 and target == 0:
        return 0

    return 2 * (val - target) / (abs(val) + abs(target))


# TODO verify the following is correct
# Example values => DAI/USDC pool
#   => https://etherscan.io/address/0x5777d92f208679db4b9778590fa3cab3ac9e2168
#
#   DAI ~ 5 x 10^8 + 18 decimal places => 5 x 10^26
#   USDC ~ 4 x 10^8 + 6 decimal places => 4 x 10^14


def test_local_swap_accuracy_for_varying_source_asset():

    # ! TO EVALUATE THE FULL LOCALSWAP EQUATION, COMMENT THE W_A == W_B OPTIMISATION FROM THE SWAP IMPLEMENTATION

    # Use DAI range => ~10^8 + 18 decimal places (for both)
    asset_a_balance: int = 7 * 10**26
    asset_b_balance: int = 3 * 10**26

    steps = 10**3

    min_swap_value = 1 * 10**18  # 1 dollar

    max_swap_value = 10000 * 10**18  # 10k dollar
    # max_swap_value = 10000000*10**18   # uncomment for 'large' comparison

    # max_swap_value = int(0.005*asset_a_balance)

    asset_a_swap_values = range(
        min_swap_value, max_swap_value, int((max_swap_value - min_swap_value) / steps)
    )

    accuracies = []
    accuracies_uniform_calc = []
    accuracies_binom_2_calc = []

    for swap_value in asset_a_swap_values:

        result_i = full_swap_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=2**64,
        )

        result_i_uniform = full_swap_uniform_approx_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=2**64,
        )

        result_i_binom_2 = full_swap_binomial_approx_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=2**64,
            rounds=2,
        )

        result_f = full_swap_f(
            input=swap_value,
            source_asset_balance=asset_a_balance,
            source_asset_weight=1,
            target_asset_balance=asset_b_balance,
            target_asset_weight=1,
            amplification=1,
        )

        # Get error
        error = get_rel_error(result_i.value, result_f)
        assert (
            error <= 0
        ), "Error is positive (should be negative)!"  # Must always return less!
        accuracies.append(-error)  # Invert error for plotting

        error_uniform_calc = get_rel_error(result_i_uniform.value, result_f)
        assert (
            error_uniform_calc <= 0
        ), "Error is positive (should be negative)!"  # Must always return less!
        accuracies_uniform_calc.append(-error_uniform_calc)  # Invert error for plotting

        error_binom_2_calc = get_rel_error(result_i_binom_2.value, result_f)
        # ! TODO current binomial implementation DOES NOT GUARANTEE to always return less than it should
        # assert error_binom_2_calc <= 0, "Error is positive (should be negative)!"   # Must always return less!
        accuracies_binom_2_calc.append(-error_binom_2_calc)  # Invert error for plotting

    # Plot
    swap_percents = [balance / asset_a_balance * 100 for balance in asset_a_swap_values]

    fig_func, ax = plt.subplots()

    ax.set_yscale("log")

    ax.scatter(swap_percents, accuracies, s=5, label="Current")

    ax.scatter(
        swap_percents, accuracies_uniform_calc, s=5, c="orange", label="Uniform Approx."
    )

    ax.scatter(
        swap_percents, accuracies_binom_2_calc, s=5, c="green", label="Binom 2 Approx."
    )

    ax.set_title("LocalSwap accuracy", fontsize=20)
    ax.grid(True, which="major")
    ax.set_axisbelow(True)
    ax.set_xlabel("Input balance as % of same asset pool balance")
    ax.set_ylabel("Relative error (-2 to 2)")

    # Save plots
    ax.legend()
    plt.savefig("localswap_err_vs_new_large", dpi=300, bbox_inches="tight")

    plt.show()


def test_local_swap_accuracy_for_varying_target_asset():

    asset_a_balance: int = 5 * 10**26

    min_asset_b_balance: int = 4 * 10**7
    max_asset_b_balance: int = 4 * 10**9

    steps = 10**3

    asset_b_balances = range(
        min_asset_b_balance,
        max_asset_b_balance,
        int((max_asset_b_balance - min_asset_b_balance) / steps),
    )

    swap_value = 10000 * 10**18  # 10000 dollar

    accuracies = []

    for asset_b_balance in asset_b_balances:

        # Perform swaps
        result_i = full_swap_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=2**64,
        )
        result_f = full_swap_f(
            input=swap_value,
            source_asset_balance=asset_a_balance,
            source_asset_weight=1,
            target_asset_balance=asset_b_balance,
            target_asset_weight=1,
            amplification=1,
        )

        # Get error
        error = get_rel_error(result_i.value, result_f)
        assert error <= 0  # Must always return less!

        accuracies.append(-error)  # Invert error for plotting

    # Plot

    fig_func, ax = plt.subplots()

    ax.set_yscale("log")
    # ax.set_xscale('log')
    ax.scatter(asset_b_balances, accuracies, s=5)

    ax.set_title("LocalSwap accuracy", fontsize=20)
    ax.grid(True, which="major")
    ax.set_axisbelow(True)
    ax.set_xlabel("Input balance as % of target asset pool balance")
    ax.set_ylabel("Relative error (-2 to 2)")

    # Save plots
    # ax.legend()
    plt.savefig("localswap_err_fn_target_balance", dpi=300, bbox_inches="tight")

    plt.show()


def test_out_swap_accuracy_for_varying_source_asset():

    # Use DAI range => ~10^8 + 18 decimal places (for both)
    asset_a_balance: int = 7 * 10**26

    steps = 10**3

    min_swap_value = 1 * 10**18  # 1 dollar

    max_swap_value = 10000 * 10**18  # 10k dollar
    # max_swap_value = 10000000*10**18   # uncomment for 'large' comparison

    # max_swap_value = int(0.005*asset_a_balance)

    asset_a_swap_values = range(
        min_swap_value, max_swap_value, int((max_swap_value - min_swap_value) / steps)
    )

    accuracies = []
    accuracies_uniform_calc = []

    for swap_value in asset_a_swap_values:

        result_i_x64 = out_swap_i_x64(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            amplification_x64=2**64,
        )

        result_i_uniform_x64 = out_swap_uniform_approx_i_x64(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            amplification_x64=2**64,
        )

        result_f = out_swap_f(
            input=swap_value,
            source_asset_balance=asset_a_balance,
            source_asset_weight=1,
            amplification=1,
        )

        # Get error
        error = get_rel_error(result_i_x64.value / 2**64, result_f)
        assert error <= 0  # Must always return less!
        accuracies.append(-error)  # Invert error for plotting

        error_uniform_calc = get_rel_error(
            result_i_uniform_x64.value / 2**64, result_f
        )
        assert error_uniform_calc <= 0  # Must always return less!
        accuracies_uniform_calc.append(-error_uniform_calc)  # Invert error for plotting

    # Plot
    swap_percents = [balance / asset_a_balance * 100 for balance in asset_a_swap_values]

    fig_func, ax = plt.subplots()

    ax.set_yscale("log")

    ax.scatter(swap_percents, accuracies, s=5, label="Current")

    ax.scatter(
        swap_percents, accuracies_uniform_calc, s=5, c="orange", label="Uniform Approx."
    )

    ax.set_title("OutSwap accuracy", fontsize=20)
    ax.grid(True, which="major")
    ax.set_axisbelow(True)
    ax.set_xlabel("Input balance as % of same asset pool balance")
    ax.set_ylabel("Relative error (-2 to 2)")

    # Save plots
    ax.legend()
    plt.savefig("outswap_err_vs_new_large", dpi=300, bbox_inches="tight")

    plt.show()


def test_local_swap_accuracy_for_varying_source_asset_in_swap_approx_only():

    # Use DAI range => ~10^8 + 18 decimal places (for both)
    asset_a_balance: int = 7 * 10**26
    asset_b_balance: int = 3 * 10**26

    steps = 10**3

    min_swap_value = 1 * 10**18  # 1 dollar

    max_swap_value = 10000 * 10**18  # 10k dollar
    # max_swap_value = 10000000*10**18   # uncomment for 'large' comparison

    # max_swap_value = int(0.005*asset_a_balance)

    asset_a_swap_values = range(
        min_swap_value, max_swap_value, int((max_swap_value - min_swap_value) / steps)
    )

    accuracies = []
    accuracies_in_swap_uniform = []
    accuracies_uniform_calc = []

    for swap_value in asset_a_swap_values:

        result_i = full_swap_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=Uint256(2**64),
        )

        out_swap_result_i_x64 = out_swap_i_x64(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            amplification_x64=Uint256(2**64),
        )

        result_i_in_approx = in_swap_uniform_approx_i(
            units_x64=out_swap_result_i_x64,
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=Uint256(2**64),
        )

        result_i_uniform = full_swap_uniform_approx_i(
            input=Uint256(swap_value),
            source_asset_balance=Uint256(asset_a_balance),
            source_asset_weight=Uint256(1),
            target_asset_balance=Uint256(asset_b_balance),
            target_asset_weight=Uint256(1),
            amplification_x64=Uint256(2**64),
        )

        result_f = full_swap_f(
            input=swap_value,
            source_asset_balance=asset_a_balance,
            source_asset_weight=1,
            target_asset_balance=asset_b_balance,
            target_asset_weight=1,
            amplification=1,
        )

        # Get error
        error = get_rel_error(result_i.value, result_f)
        assert (
            error <= 0
        ), "Error is positive (should be negative)!"  # Must always return less!
        accuracies.append(-error)  # Invert error for plotting

        error_in_approx = get_rel_error(result_i_in_approx.value, result_f)
        assert (
            error_in_approx <= 0
        ), "Error is positive (should be negative)!"  # Must always return less!
        accuracies_in_swap_uniform.append(-error_in_approx)  # Invert error for plotting

        error_uniform_calc = get_rel_error(result_i_uniform.value, result_f)
        assert (
            error_uniform_calc <= 0
        ), "Error is positive (should be negative)!"  # Must always return less!
        accuracies_uniform_calc.append(-error_uniform_calc)  # Invert error for plotting

    # Plot
    swap_percents = [balance / asset_a_balance * 100 for balance in asset_a_swap_values]

    fig_func, ax = plt.subplots()

    ax.set_yscale("log")

    ax.scatter(swap_percents, accuracies, s=5, label="Current LocalSwap")

    ax.scatter(
        swap_percents,
        accuracies_uniform_calc,
        s=5,
        c="orange",
        label="Uniform Approx. of LocalSwap",
    )

    ax.scatter(
        swap_percents,
        accuracies_in_swap_uniform,
        s=5,
        c="red",
        label="Full swap with SwapFromUnits Approx.",
    )

    ax.set_title("Full Swap Accuracy", fontsize=20)
    ax.grid(True, which="major")
    ax.set_axisbelow(True)
    ax.set_xlabel("Input balance as % of same asset pool balance")
    ax.set_ylabel("Relative error (-2 to 2)")

    # Save plots
    ax.legend()
    plt.savefig("localswap_with_inswap_uniform_approx", dpi=300, bbox_inches="tight")

    plt.show()
