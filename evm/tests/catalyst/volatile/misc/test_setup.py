import brownie

from tests.catalyst.conftest import NUM_ASSETS  #TODO move to fixture

# # TODO do we want deployment of a pool with no tokens to fail? (currently it does not fail)
# def test_setup_no_tokens(deployer, deploy_swappool):
#     sp = deploy_swappool(
#         [],
#         [],
#         [],
#         2**64,
#         "",
#         "",
#         deployer=deployer,
#     )


# def test_setup_too_many_tokens(deployer, deploy_swappool, token_list):
#     asset_count = NUM_ASSETS + 1

#     with brownie.reverts():     #TODO add dev revert message
#         sp = deploy_swappool(
#             token_list[:asset_count],
#             [10**8]*asset_count,
#             [1]*asset_count,
#             2**64,
#             "",
#             "",
#             deployer=deployer,
#         )


# def test_setup_token_0_balance(deployer, deploy_swappool, token_list):
#     asset_count = NUM_ASSETS

#     with brownie.reverts():     #TODO add dev revert message
#         sp = deploy_swappool(
#             token_list[:asset_count],
#             [10**8]*(asset_count - 1) + [0],    # Set the initial balance for the last token to 0
#             [1]*asset_count,
#             2**64,
#             "",
#             "",
#             deployer=deployer,
#         )


def test_setup_call_twice(deploy_swappool, crosschaininterface, pool_data, token_list):

    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = [token_list[idx] for idx in pool_data.get("tokens")]
    depositAmounts = pool_data.get("depositAmounts")

    # Deploy the swap pool via the factory (internally calls setup on the swappool)
    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        2**64,
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
    )

    # Call setup again
    with brownie.reverts(dev_revert_msg="dev: Pool Already setup."):
        sp.setup(
            tokens,
            [1 for _ in tokens],
            2**64,
            0,
            "",
            "",
            crosschaininterface,
            deployer,
            {"from": deployer}
        )


def test_setup_invalid_amplification(deploy_swappool, pool_data, token_list):
    
    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = [token_list[idx] for idx in pool_data.get("tokens")]
    depositAmounts = pool_data.get("depositAmounts")

    # Deploy the swap pool via the factory (internally calls setup on the swappool)
    with brownie.reverts():     #TODO add dev revert message
        sp = deploy_swappool(
            tokens,
            depositAmounts,
            pool_data.get("weights"),
            2**64,
            pool_data.get("poolName"),
            pool_data.get("poolSymbol"),
            deployer=deployer,
            template_index=1    # Use amplified contract
        )



# Finish setup tests

def test_finish_setup_unauthorized(deploy_swappool, pool_data, token_list, molly):

    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = [token_list[idx] for idx in pool_data.get("tokens")]
    depositAmounts = pool_data.get("depositAmounts")

    # Deploy the swap pool via the factory (internally calls setup on the swappool)
    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        2**64,
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
    )

    sp.finishSetup({"from": molly})



def test_finish_setup_call_twice(deploy_swappool, pool_data, token_list):

    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = [token_list[idx] for idx in pool_data.get("tokens")]
    depositAmounts = pool_data.get("depositAmounts")

    # Deploy the swap pool via the factory (internally calls setup on the swappool)
    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        2**64,
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
    )

    sp.finishSetup({"from": deployer})

    with brownie.reverts():     #TODO add dev revert message
        sp.finishSetup({"from": deployer})



def test_finish_setup_only_local(deploy_swappool, pool_data, token_list, molly):

    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = [token_list[idx] for idx in pool_data.get("tokens")]
    depositAmounts = pool_data.get("depositAmounts")

    # Deploy the swap pool via the factory (internally calls setup on the swappool)
    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        2**64,
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
        only_local=True
    )

    sp.finishSetup({"from": deployer})

    assert sp._onlyLocal()