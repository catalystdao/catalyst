from dataclasses import dataclass
from typing import Any, List

# Defaults

DEFAULT_POOL_DECIMALS     = 18
INITIAL_POOL_TOKEN_SUPPLY = 2**64
MAX_POOL_ASSETS           = 3

# Dataclasses

@dataclass
class SwapPoolInfo:
    swappool            : Any
    tokens              : List[Any]
    init_token_balances : List[int]
    token_weights       : List[int]
    amplification       : float
    pool_fee            : float
    governance_fee      : float
    name                : str
    symbol              : str
    deployer            : Any