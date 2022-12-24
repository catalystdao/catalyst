# @version =0.3.3

from vyper.interfaces import ERC20

event PoolDeployed:
    deployer : indexed(address)  # msg.sender
    pool_address : indexed(address)  # The forwarder for the pool template
    _chaininterface : indexed(address)  # Which cross chain messaging service is used? #! TODO CHECK SECURITY
    _k : uint256  # amplification
    _assets : address[NUMASSETS]  # List of the 3 assets


NUMASSETS: constant(uint256) = 3
poolTemplate: public(address)
amplifiedPoolTemplate: public(address)
IsCreatedByFactory: public(HashMap[address, HashMap[address, bool]])
ONE: constant(uint256) = 2**64

interface ISwapPool:
    def setup(chaininterface : address, init_assets : address[NUMASSETS], weights : uint256[NUMASSETS], k : uint256, name : String[16], symbol: String[8], setupMaster : address): nonpayable


@external
def __init__( _poolTemplate : address, _amplifiedPoolTemplate : address):
    self.poolTemplate = _poolTemplate
    self.amplifiedPoolTemplate = _amplifiedPoolTemplate


@external
def deploy_swappool(_chaininterface : address, init_assets : address[NUMASSETS], init_balances : uint256[3], weights : uint256[NUMASSETS], k : uint256, name: String[16], symbol: String[8]) -> address:
    sp : address = empty(address)
    if k == ONE:
        sp = create_forwarder_to(self.poolTemplate)
    else:
        sp = create_forwarder_to(self.amplifiedPoolTemplate)
    self.IsCreatedByFactory[_chaininterface][sp] = True
    
    # The pool expects the balance0s to exist in the pool when setup is called.
    for i_asset in range(NUMASSETS):
        if init_assets[i_asset] == ZERO_ADDRESS:
            break
        ERC20(init_assets[i_asset]).transferFrom(msg.sender, sp, init_balances[i_asset])
    
    ISwapPool(sp).setup(_chaininterface, init_assets, weights, k, name, symbol, msg.sender)
    
    log PoolDeployed(msg.sender, sp, _chaininterface, k, init_assets)
    return sp
    