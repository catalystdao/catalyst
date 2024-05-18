// SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

import { ERC20 } from 'solady/tokens/ERC20.sol';

contract Token is ERC20 {

    string internal _name;
    string internal _symbol;
    uint8 internal _decimals;

    function name() public view override returns(string memory) {
        return _name;
    }

    function symbol() public view override returns(string memory) {
        return _symbol;
    }

    constructor(
        string memory name_,
        string memory symbol_,
        uint8 decimals_,
        uint256 initialSupply
    ) {
        _name = name_;
        _symbol = symbol_;
        _decimals = decimals_;
        _mint(msg.sender, initialSupply * 10**decimals_);
    }

    function decimals() public view override returns(uint8) {
        return _decimals;
    }

    function mint(address _to, uint256 _amount) public {
        _mint(_to, _amount);
    }
}
