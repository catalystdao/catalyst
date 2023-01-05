import pytest

from brownie import accounts, convert


def _connect_pools(channelId, swappool1, swappool2):
    deployer1 = accounts.at(swappool1._setupMaster())
    
    swappool1.createConnection(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        True,
        {"from": deployer1},
    )
    
    deployer2 = accounts.at(swappool2._setupMaster())
    
    swappool2.createConnection(
        channelId,
        convert.to_bytes(swappool1.address.replace("0x", "")),
        True,
        {"from": deployer2},
    )


def _finish_setup(swappool1, swappool2):
    deployer1 = accounts.at(swappool1._setupMaster())
    
    swappool1.finishSetup({"from": deployer1})
    
    deployer2 = accounts.at(swappool2._setupMaster())
    
    swappool2.finishSetup({"from": deployer2})
    
        

@pytest.fixture(scope="module")
def connect_pools(channelId, swappool1, swappool2):
    _connect_pools(channelId, swappool1, swappool2)
    
    
@pytest.fixture(scope="module")
def finish_setup(swappool1, swappool2):
    _finish_setup(swappool1, swappool2)
    

@pytest.fixture(scope="module")
def connect_pools_amp(channelId, swappool1_amp, swappool2_amp):
    _connect_pools(channelId, swappool1_amp, swappool2_amp)
    
    
@pytest.fixture(scope="module")
def finish_setup_amp(swappool1_amp, swappool2_amp):
    _finish_setup(swappool1_amp, swappool2_amp)