# Catalust Tests

Tests are organised based on which feature they test:

*./CatalystFactory*

*./CatalystInterface*

*./CatalystRouter*

*./CatalystVault*

And then for larger tests of the system as a whole *./integration* 

Tests may be sub-organised based on the function which they test. For example *./CatalystFactory/DeployVault.t.sol* tests the `deployVault` function in the Catalyst Factory.

*./TestCommon.t.sol*
Contains frequently used code snippits to simplify testing and inherits from other `Common` contracts which exposes other often uses macros.

*./mocks*
Contains various mocks used throughout testing.


## Vault Testing

The majority of testing is for the Catalyst vaults, since this is where the majority of the contract risk exists. To test many different variations of vaults without having to repeat code several times, vault tests (which aren't integration tests) are written such that they can be inherited and the deployment configuration is then provided to the test.

For the simplest example, see *./CatalystVault/LocalSwap.t.sol*. This contract is inherited by *./CatalystVault/Volatile/VolatileLocalSwap.t.sol* where the test is run.

## 