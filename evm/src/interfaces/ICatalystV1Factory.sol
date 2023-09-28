pragma solidity ^0.8.10;

import { ICatalystV1FactoryEvents } from "./ICatalystV1FactoryEvents.sol";

interface ICatalystV1Factory is ICatalystV1FactoryEvents {
    function _defaultGovernanceFee() external view returns (uint256);
    function _governanceFeeDestination() external view returns (address);
    function deployVault(
        address vaultTemplate,
        address[] memory assets,
        uint256[] memory init_balances,
        uint256[] memory weights,
        uint256 amp,
        uint256 vaultFee,
        string memory name,
        string memory symbol,
        address chainInterface
    ) external returns (address);
    function isCreatedByFactory(address, address) external view returns (bool);
    function setDefaultGovernanceFee(uint256 fee) external;
    function setGovernanceFeeDestination(address feeDestination) external;
}