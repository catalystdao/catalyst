pool_information = {
    "default": {
        "amp": 10**18,
        "tokens": [0,1,2],
        "depositAmounts": [10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        "weights": [1,1,1],
        "poolName": "POOLNAME",
        "poolSymbol": "PS",
        "deployer": 1,
        "selfConnection": True
    },
    "2tokens": {
        "amp": 10**18,
        "tokens": [0, 1],
        "depositAmounts": [10 * 10**18, 100 * 10**18],
        "weights": [1,1],
        "poolName": "AnotherPoolName",
        "poolSymbol": "APN",
        "deployer": 1,
        "selfConnection": True,
    }, 
    "1token": {
        "amp": 10**18,
        "tokens": [2],
        "depositAmounts": [1000 * 10**6],
        "weights": [1],
        "poolName": "POOLNAME2",
        "poolSymbol": "PS2",
        "deployer": 1,
        "selfConnection": False
    },
    "ampdefault": {
        "amp": 2**62,
        "tokens": [0, 1, 2],
        "depositAmounts": [10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        "weights": [int(int(1000 * 10**18)/int(10 * 10**18)), 1, int(int(1000 * 10**18)/int(1000 * 10**6))],
        "poolName": "AMP1",
        "poolSymbol": "AP1",
        "deployer": 1,
        "selfConnection": True
    },
    "amp2tokens": {
        "amp": 2**62,
        "tokens": [0, 1],
        "depositAmounts": [10 * 10**18, 1000 * 10**18],
        "weights": [int(int(1000 * 10**18)/int(10 * 10**18)), 1],
        "poolName": "AMP2",
        "poolSymbol": "AP2",
        "deployer": 1,
        "selfConnection": True
    },
    "amp1tokens": {
        "amp": 2**62,
        "tokens": [2],
        "depositAmounts": [1000 * 10**6],
        "weights": [int(int(1000 * 10**18)/int(1000 * 10**6))],
        "poolName": "AMP1tkn",
        "poolSymbol": "A1T",
        "deployer": 1,
        "selfConnection": False
    }
}