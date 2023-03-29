abi = [
    {
        'inputs': [
            {
                'internalType': "address",
                'name': "IBCDispatcher_",
                'type': "address"
            }
        ],
        'name': "constructor",
        'stateMutability': "nonpayable",
        'type': "constructor"
    },
    {
        'inputs': [
            {
                'internalType': "bytes1",
                'name': "context",
                'type': "bytes1"
            }
        ],
        'name': "InvalidContext",
        'type': "error"
    },
    {
        'inputs': [
            {
                'internalType': "address",
                'name': "caller",
                'type': "address"
            }
        ],
        'name': "InvalidIBCCaller",
        'type': "error"
    },
    {
        'anonymous': False,
        'inputs': [
            {
                'indexed': True,
                'internalType': "address",
                'name': "previousOwner",
                'type': "address"
            },
            {
                'indexed': True,
                'internalType': "address",
                'name': "newOwner",
                'type': "address"
            }
        ],
        'name': "OwnershipTransferred",
        'type': "event"
    },
    {
        'inputs': [],
        'name': "IBC_DISPATCHER",
        'outputs': [
            {
                'internalType': "address",
                'name': "",
                'type': "address"
            }
        ],
        'stateMutability': "view",
        'type': "function"
    },
    {
        'inputs': [
            {
                'components': [
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "src",
                        'type': "tuple"
                    },
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "dest",
                        'type': "tuple"
                    },
                    {
                        'internalType': "uint64",
                        'name': "sequence",
                        'type': "uint64"
                    },
                    {
                        'internalType': "bytes",
                        'name': "data",
                        'type': "bytes"
                    },
                    {
                        'components': [
                            {
                                'internalType': "uint64",
                                'name': "block",
                                'type': "uint64"
                            },
                            {
                                'internalType': "uint64",
                                'name': "timestamp",
                                'type': "uint64"
                            }
                        ],
                        'internalType': "struct IbcTimeout",
                        'name': "timeout",
                        'type': "tuple"
                    }
                ],
                'internalType': "struct IbcPacket",
                'name': "packet",
                'type': "tuple"
            }
        ],
        'name': "onAcknowledgementPacket",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [
            {
                'components': [
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "src",
                        'type': "tuple"
                    },
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "dest",
                        'type': "tuple"
                    },
                    {
                        'internalType': "uint64",
                        'name': "sequence",
                        'type': "uint64"
                    },
                    {
                        'internalType': "bytes",
                        'name': "data",
                        'type': "bytes"
                    },
                    {
                        'components': [
                            {
                                'internalType': "uint64",
                                'name': "block",
                                'type': "uint64"
                            },
                            {
                                'internalType': "uint64",
                                'name': "timestamp",
                                'type': "uint64"
                            }
                        ],
                        'internalType': "struct IbcTimeout",
                        'name': "timeout",
                        'type': "tuple"
                    }
                ],
                'internalType': "struct IbcPacket",
                'name': "packet",
                'type': "tuple"
            }
        ],
        'name': "onRecvPacket",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [
            {
                'components': [
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "src",
                        'type': "tuple"
                    },
                    {
                        'components': [
                            {
                                'internalType': "bytes32",
                                'name': "portId",
                                'type': "bytes32"
                            },
                            {
                                'internalType': "bytes32",
                                'name': "channelId",
                                'type': "bytes32"
                            }
                        ],
                        'internalType': "struct IbcEndpoint",
                        'name': "dest",
                        'type': "tuple"
                    },
                    {
                        'internalType': "uint64",
                        'name': "sequence",
                        'type': "uint64"
                    },
                    {
                        'internalType': "bytes",
                        'name': "data",
                        'type': "bytes"
                    },
                    {
                        'components': [
                            {
                                'internalType': "uint64",
                                'name': "block",
                                'type': "uint64"
                            },
                            {
                                'internalType': "uint64",
                                'name': "timestamp",
                                'type': "uint64"
                            }
                        ],
                        'internalType': "struct IbcTimeout",
                        'name': "timeout",
                        'type': "tuple"
                    }
                ],
                'internalType': "struct IbcPacket",
                'name': "packet",
                'type': "tuple"
            }
        ],
        'name': "onTimeoutPacket",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [],
        'name': "owner",
        'outputs': [
            {
                'internalType': "address",
                'name': "",
                'type': "address"
            }
        ],
        'stateMutability': "view",
        'type': "function"
    },
    {
        'inputs': [],
        'name': "registerPort",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [],
        'name': "renounceOwnership",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [
            {
                'internalType': "bytes32",
                'name': "channelId",
                'type': "bytes32"
            },
            {
                'internalType': "bytes32",
                'name': "toPool",
                'type': "bytes32"
            },
            {
                'internalType': "bytes32",
                'name': "toAccount",
                'type': "bytes32"
            },
            {
                'internalType': "uint8",
                'name': "toAssetIndex",
                'type': "uint8"
            },
            {
                'internalType': "uint256",
                'name': "U",
                'type': "uint256"
            },
            {
                'internalType': "uint256",
                'name': "minOut",
                'type': "uint256"
            },
            {
                'components': [
                    {
                        'internalType': "uint256",
                        'name': "fromAmount",
                        'type': "uint256"
                    },
                    {
                        'internalType': "address",
                        'name': "fromAsset",
                        'type': "address"
                    },
                    {
                        'internalType': "bytes32",
                        'name': "swapHash",
                        'type': "bytes32"
                    },
                    {
                        'internalType': "uint32",
                        'name': "blockNumber",
                        'type': "uint32"
                    }
                ],
                'internalType': "struct AssetSwapMetadata",
                'name': "metadata",
                'type': "tuple"
            },
            {
                'internalType': "bytes",
                'name': "calldata_",
                'type': "bytes"
            }
        ],
        'name': "sendCrossChainAsset",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [
            {
                'internalType': "bytes32",
                'name': "channelId",
                'type': "bytes32"
            },
            {
                'internalType': "bytes32",
                'name': "toPool",
                'type': "bytes32"
            },
            {
                'internalType': "bytes32",
                'name': "toAccount",
                'type': "bytes32"
            },
            {
                'internalType': "uint256",
                'name': "U",
                'type': "uint256"
            },
            {
                'internalType': "uint256",
                'name': "minOut",
                'type': "uint256"
            },
            {
                'components': [
                    {
                        'internalType': "uint256",
                        'name': "fromAmount",
                        'type': "uint256"
                    },
                    {
                        'internalType': "bytes32",
                        'name': "swapHash",
                        'type': "bytes32"
                    },
                    {
                        'internalType': "uint32",
                        'name': "blockNumber",
                        'type': "uint32"
                    }
                ],
                'internalType': "struct LiquiditySwapMetadata",
                'name': "metadata",
                'type': "tuple"
            },
            {
                'internalType': "bytes",
                'name': "calldata_",
                'type': "bytes"
            }
        ],
        'name': "sendCrossChainLiquidity",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    },
    {
        'inputs': [
            {
                'internalType': "address",
                'name': "newOwner",
                'type': "address"
            }
        ],
        'name': "transferOwnership",
        'outputs': [],
        'stateMutability': "nonpayable",
        'type': "function"
    }
]