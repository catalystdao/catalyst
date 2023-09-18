// SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "openzeppelin-contracts/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "openzeppelin-contracts/contracts/token/ERC20/extensions/draft-ERC20Permit.sol";

contract Token is ERC20Permit, ERC20Burnable, Ownable {
    uint8 private _decimals;
    address private _minter;

    event MinterTransferred(
        address indexed previousMinter,
        address indexed newMinter
    );

    constructor(
        string memory name,
        string memory symbol,
        uint8 decimals_,
        uint256 initialSupply
    ) ERC20(name, symbol) ERC20Permit(name) {
        _mint(msg.sender, initialSupply * 10**decimals_);
        _setMinter(_msgSender());
        _decimals = decimals_;
    }

    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

    function minter() public view virtual returns (address) {
        return _minter;
    }

    modifier onlyMinter() {
        require(minter() == _msgSender(), "Ownable: caller is not the minter");
        _;
    }

    function transferMinter(address newMinter) public onlyOwner {
        _setMinter(newMinter);
    }

    function mint(address _to, uint256 _amount) public onlyMinter {
        _mint(_to, _amount);
    }

    function _setMinter(address newMinter) private {
        address oldMinter = _minter;
        _minter = newMinter;
        emit MinterTransferred(oldMinter, newMinter);
    }
}
