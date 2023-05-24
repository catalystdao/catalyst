from brownie import convert

MSG_SENDER = convert.to_address(convert.to_bytes(0x01, "bytes20"))
ADDRESS_THIS = convert.to_address(convert.to_bytes(0x02, "bytes20"))
BALANCE_THIS = 2**255