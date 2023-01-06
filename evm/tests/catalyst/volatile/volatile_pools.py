volatile_params = {
    "3 tokens": {
        "amp": 2**64,
        "tokens": [0,1,2],
        "depositAmounts": [10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        "weights": [1,1,1],
        "poolName": "POOLNAME",
        "poolSymbol": "PS",
        "deployer": 1,
        "selfConnection": True
    },
    "2 tokens": {
        "amp": 2**64,
        "tokens": [0, 1],
        "depositAmounts": [10 * 10**18, 100 * 10**18],
        "weights": [1,1,1],
        "poolName": "AnotherPoolName",
        "poolSymbol": "APN",
        "deployer": 1,
        "selfConnection": True
    }
}