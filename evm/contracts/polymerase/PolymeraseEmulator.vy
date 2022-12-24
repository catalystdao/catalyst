# @version =0.3.3


interface IAcceptingContract:
    def receives(_header: Bytes[40], _data : Bytes[130]) -> bytes32: nonpayable

struct CrossChainTX:
    _data: Bytes[130]
    _target: bytes32
    _executed: bool
    _sender: address

event CrossChainTxEvent:
    _data: Bytes[130]
    _target: bytes32
    _sender: address

bytesAwaitingExecution: public(HashMap[uint256, CrossChainTX])
lastFilled: public(uint256) 

@payable
@external
def call_multichain(_chain : uint256, _target : bytes32, _data : Bytes[130]) -> bool:

    _toFill: uint256 = self.lastFilled + 1
    _executionContext: CrossChainTX = CrossChainTX(
        {_data: _data, _target: _target, _executed: False, _sender: msg.sender}
    )

    log CrossChainTxEvent(_data, _target, msg.sender)
 
    self.bytesAwaitingExecution[_toFill] = _executionContext
    self.lastFilled = _toFill

    return True


@external
def execute( _index : uint256) -> bytes32:
    _executionContext: CrossChainTX = self.bytesAwaitingExecution[_index]
    assert not _executionContext._executed

    _targetContract : address = convert(_executionContext._target, address)

    self.bytesAwaitingExecution[_index]._executed = True

    _header : Bytes[40] = concat(slice(convert(chain.id, bytes32), 32-8, 8), convert(_executionContext._sender, bytes32))
    return IAcceptingContract(_targetContract).receives(_header, _executionContext._data)

