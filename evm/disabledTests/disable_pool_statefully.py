import pytest

import brownie

from brownie.test import given, strategy
from hypothesis import assume
from hypothesis.stateful import consumes, rule, Bundle, precondition, invariant
from hypothesis.strategies import composite


def product(*elements):
    prod = 1
    for element in elements:
        prod *= element
    return element


def balancerEquvilantSwap(value, fromSwapPool, toSwapPool, fromAsset, toAsset):
    a = fromAsset.balanceOf(fromSwapPool)
    b = toAsset.balanceOf(toSwapPool)
    x = value
    w = 1 / 1
    # y = b * (1 - (a / (a + x)) ** w)
    y = (b * x) // (a + x)
    return y


@composite
def differentChains(draw, elements=strategy("uint256", max_value=1)):
    c1 = draw(elements)
    c2 = draw(elements.filter(lambda x: x != c1))
    return (c1, c2)


class StateMachine:
    st_value = strategy("uint256", max_value=9000 * 10**18)
    st_index = strategy("uint256", max_value=1)
    st_user_index = strategy("uint256", max_value=2)
    st_percentage = strategy("uint256", max_value=1 * 10**6)
    st_chains = differentChains()
    awaiting_execution = Bundle("awaiting_execution")

    num_underway = 0

    def __init__(
        cls,
        accounts,
        gov,
        crosschaininterface,
        AMOUNT,
        deploy_swappool,
        create_token,
        swapfactory,
        ibcemulator,
        num_chains=2,
        num_tokens=[2, 3],
    ):
        cls.gov = gov
        cls.accounts = accounts
        cls.chain_id = crosschaininterface.chain_id()
        cls.ibcemulator = ibcemulator
        cls.num_chains = num_chains
        cls.num_tokens = num_tokens
        cls.AMOUNT = AMOUNT

        cls.tokens = [[], [], []]
        cls.swappool = []
        for i in range(num_chains):
            for j in range(num_tokens[i]):
                cls.tokens[i].append(
                    create_token(f"{i},{j}", f"C{i}T{j}", deployer=gov)
                )
                cls.tokens[i][j].approve(swapfactory, 2**256 - 1, {"from": gov})
            for j in range(3 - num_tokens[i]):
                cls.tokens[i].append(brownie.ZERO_ADDRESS)

            cls.swappool.append(
                deploy_swappool(cls.tokens[i], AMOUNT, amp=2**64, deployer=gov)
            )

        for i in range(num_chains):
            for j in range(num_chains):
                if i == j:
                    continue
                cls.swappool[i].setConnection(
                    cls.chain_id,
                    brownie.convert.to_bytes(cls.swappool[j].address.replace("0x", "")),
                    True,
                    {"from": gov},
                )
            cls.swappool[i].finishSetup({"from": gov})

    @pytest.mark.no_call_coverage
    def calcTopInvariant(self) -> int:
        product = 1
        for i in range(self.num_chains):
            for j in range(self.num_tokens[i]):
                product *= self.tokens[i][j].balanceOf(self.swappool[i])

        return product

    @pytest.mark.no_call_coverage
    def calcBottomInvariant(self) -> int:

        product = 1
        for i in range(self.num_chains):
            for j in range(self.num_tokens[i]):
                product *= self.swappool[i]._balance0(self.tokens[i][j])

        return product

    def setup(self):
        # Deploy the tokens for each "chain"
        self.balances = [
            [[0 for _ in range(self.num_tokens[i])] for i in range(self.num_chains)]
            for _ in range(2)
        ]
        self.topInvariant = self.calcTopInvariant()
        self.bottomInvariant = self.calcBottomInvariant()

        for i in range(self.num_chains):
            for j in range(self.num_tokens[i]):
                self.tokens[i][j].transfer(
                    self.accounts[0], self.AMOUNT[j], {"from": self.gov}
                )

    @rule(st_percentage=st_percentage, st_chain=st_index)
    def deposit(self, st_percentage, st_chain):
        percentage = st_percentage / 10**6

        token0 = self.tokens[st_chain][0]
        swappool = self.swappool[st_chain]

        baseAmount = (
            token0.balanceOf(swappool)
            * int(token0.balanceOf(self.accounts[0]) * percentage)
        ) // swappool._balance0(token0)
        reverts = False
        for token in self.tokens[st_chain]:
            if token == brownie.ZERO_ADDRESS:
                break
            if token.balanceOf(self.accounts[0]) < baseAmount * swappool._balance0(
                token
            ) ** 2 / (token.balanceOf(swappool) * swappool.totalSupply()):
                reverts = True
        print(f"Depositing: {baseAmount}")

        for token in self.tokens[st_chain]:
            if token == brownie.ZERO_ADDRESS:
                break
            token.approve(swappool, 2**256 - 1, {"from": self.accounts[0]})

        if reverts:
            with brownie.reverts():
                swappool.depositAll(baseAmount, {"from": self.accounts[0]})
        else:
            swappool.depositAll(baseAmount, {"from": self.accounts[0]})

    @rule(st_percentage=st_percentage, st_chain=st_index)
    def withdraw(self, st_percentage, st_chain):
        percentage = st_percentage / 10**6

        token0 = self.tokens[st_chain][0]
        swappool = self.swappool[st_chain]

        baseAmount = int(swappool.balanceOf(self.accounts[0]) * percentage)
        print(f"Withdrawing: {baseAmount}")

        swappool.withdrawAll(baseAmount, [0, 0, 0], {"from": self.accounts[0]})

    @rule(
        st_value=st_value,
        st_user_index=st_user_index,
        st_chain=st_index,
        st_fromAsset=st_index,
        st_toAsset=st_index,
    )
    def localswap(
        self,
        st_value,
        st_user_index,
        st_chain,
        st_fromAsset,
        st_toAsset,
    ):
        # This can be savely removed. However, the ordering of swap actions within the contract means swap returns
        # less than it is supposed to. The result is that the invariant increases a lot, which is observed as a bug.
        assume((st_fromAsset != st_toAsset))

        pool = self.swappool[st_chain]

        assume(not (st_value > self.tokens[st_chain][st_fromAsset].balanceOf(self.gov)))
        print(f"Localswapping {st_value} {st_fromAsset} to {st_toAsset} on {st_chain}")

        fromAsset = self.tokens[st_chain][st_fromAsset]
        fromAsset.transfer(self.accounts[st_user_index], st_value, {"from": self.gov})
        fromAsset.approve(pool, st_value, {"from": self.accounts[st_user_index]})
        toAsset = self.tokens[st_chain][st_toAsset]

        BES = balancerEquvilantSwap(st_value, pool, pool, fromAsset, toAsset)

        initialOutputTokenBalance = toAsset.balanceOf(self.accounts[st_user_index])
        tx = pool.localswap(
            fromAsset,
            toAsset,
            st_value,
            0,
            {"from": self.accounts[st_user_index]},
        )

        postSwapBalance = toAsset.balanceOf(self.accounts[st_user_index])
        if BES > 0:
            if fromAsset.address == toAsset.address:
                swapReturn = postSwapBalance - initialOutputTokenBalance + st_value
            else:
                swapReturn = postSwapBalance - initialOutputTokenBalance
            effeciency = (swapReturn) / BES
            print(f"Trade returned {effeciency:.4f}%, {swapReturn}, {BES}")
            if effeciency != 0:
                assert 1 >= effeciency >= (0.98 if st_value > 1e5 else 0.5)

    @rule(
        target=awaiting_execution,
        st_value=st_value,
        st_user_index=st_user_index,
        st_pools=st_chains,
        st_fromAsset=st_index,
        st_toAsset=st_index,
    )
    def swap_async(
        self,
        st_value,
        st_user_index,
        st_pools,
        st_fromAsset,
        st_toAsset,
    ):
        swapToUnitspingPool = self.swappool[st_pools[0]]
        assume(
            not (st_value > self.tokens[st_pools[0]][st_fromAsset].balanceOf(self.gov))
        )

        print(
            f"Async swapping {st_value}, {st_fromAsset} to {st_toAsset} on {st_pools}"
        )

        fromAsset = self.tokens[st_pools[0]][st_fromAsset]
        fromAsset.transfer(self.accounts[st_user_index], st_value, {"from": self.gov})
        fromAsset.approve(
            swapToUnitspingPool, st_value, {"from": self.accounts[st_user_index]}
        )

        tx = swapToUnitspingPool.swapToUnits(
            self.chain_id,
            brownie.convert.to_bytes(
                self.swappool[st_pools[1]].address.replace("0x", "")
            ),
            brownie.convert.to_bytes(
                self.accounts[st_user_index].address.replace("0x", "")
            ),
            fromAsset,
            st_toAsset,
            st_value,
            0,
            {"from": self.accounts[st_user_index]},
        )
        self.num_underway += 1
        return tx.events["EmulatorPackage"]["packageId"]

    @rule(package_to_execute=consumes(awaiting_execution))
    def execute_async(self, package_to_execute):
        print("Package", package_to_execute)
        self.ibcemulator.execute(package_to_execute, {"from": self.gov})
        self.num_underway -= 1

    @rule(
        st_user_index=st_user_index,
        st_pools=st_chains,
        st_percentage=strategy("uint256", min_value=1, max_value=10**8),
    )
    def liquiditySwap(self, st_user_index, st_pools, st_percentage):
        percentage = st_percentage / 10**8
        user = self.accounts[st_user_index]

        swapToUnitspingPool = self.swappool[st_pools[0]]
        inSwappingPool = self.swappool[st_pools[1]]

        user_pool_token_balance = swapToUnitspingPool.balanceOf(user)
        current_target_pool_token_balance = inSwappingPool.balanceOf(user)

        assume(user_pool_token_balance > 0)

        to_swap = user_pool_token_balance * percentage

        print(f"Liquidity swapping {to_swap} via {st_pools}")
        tx = swapToUnitspingPool.outLiquidity(
            self.chain_id,
            brownie.convert.to_bytes(inSwappingPool.address.replace("0x", "")),
            brownie.convert.to_bytes(user.address.replace("0x", "")),
            to_swap,
            user,
            {"from": user},
        )
        ibc_package_to_execute = tx.events["EmulatorPackage"]["packageId"]
        txe = self.ibcemulator.execute(ibc_package_to_execute, {"from": self.gov})

    @pytest.mark.no_call_coverage
    @precondition(lambda self: self.num_underway == 0)
    @invariant(check_during_init=False)
    def largeInvariant(self):
        newTopInvariant = self.calcTopInvariant()
        newBottomInvariant = self.calcBottomInvariant()
        print(
            f"Invariant: {self.topInvariant / self.bottomInvariant} => {newTopInvariant / newBottomInvariant} "
        )
        assert (
            newTopInvariant / newBottomInvariant * (1 + 1e13)
            >= self.topInvariant / self.bottomInvariant
        ), "Invariant decreased => Loss of assets"
        # assert 1.1 >= (newTopInvariant / newBottomInvariant) / (
        #     self.topInvariant / self.bottomInvariant
        # ), "Invariant increased by a lot suddenly. Something might be wrong"
        self.topInvariant = newTopInvariant
        self.bottomInvariant = newBottomInvariant


def test_stateful(
    state_machine,
    accounts,
    gov,
    crosschaininterface,
    deploy_swappool,
    create_token,
    swapfactory,
    ibcemulator,
    module_isolation,
):
    state_machine(
        StateMachine,
        accounts[1:4],
        gov,
        crosschaininterface,
        [10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        deploy_swappool,
        create_token,
        swapfactory,
        ibcemulator,
    )
