# Run Tests
To run all the tests, from the EVM directory, use:
```
brownie test tests/catalyst
```

## Flags
The following flags can be used to run specific collections/configurations of the tests:
| Flag                        | Description                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--config CONFIG_FILE_NAME` | Use a specific config file ('default' used if not specified).                                                                                                                                                                                                                                                                                                                                           |
| `--volatile`                | Run tests for volatile vaults.                                                                                                                                                                                                                                                                                                                                                                          |
| `--amplified`               | Run tests for amplified vaults                                                                                                                                                                                                                                                                                                                                                                          |
| `--amplification VALUE`     | Override the amplification constant that is specified on the specified config file. (May only be set if amplified tests are set to be run.)                                                                                                                                                                                                                                                             |
| `--vault PARAM_TYPE`        | Specify how to parametrize the `vault` fixture:<ul><li>`'all'`: Go through all the vaults.</li><li>`VAULT_INDEX`: Only the specified vault.</li></ul>Defaults to `'all'` if **not** running with `--fast`, otherwise it defaults to `0`.                                                                                                                                                                |
| `--vault-1 PARAM_TYPE`      | Specify how to parametrize the `vault_1` fixture:<ul><li>`'all'`: Go through all the vaults.</li><li>`VAULT_INDEX`: Only the specified vault.</li></ul>Defaults to `0`.                                                                                                                                                                                                                                 |  |
| `--vault-2 PARAM_TYPE`      | Specify how to parametrize the `vault_2` fixture:<ul><li>`'all'`: Go through all the vaults (skips the `vault_1`, i.e. avoid combinations with the same vault)</li><li>`'next'`: Use the *next* vault defined after the current `vault_1` (by index)</li><li>`VAULT_INDEX`: Only the specified vault.</li></ul>Defaults to `'all'` if **not** running with `--fast`, otherwise it defaults to `'next'`. |
| `--unit`                    | Run unit tests.                                                                                                                                                                                                                                                                                                                                                                                         |
| `--integration`             | Run integration tests.                                                                                                                                                                                                                                                                                                                                                                                  |
| `--filter FILTER`           | Filter the collected tests according to the provided filter (filtered by string inclusion): [file-name][::[test-name]]. More than one filter may be specified.                                                                                                                                                                                                                                          |
| `--fast`                    | Do not test the specified strategies of the `@given` parametrized tests, only test the `@example` values.                                                                                                                                                                                                                                                                                               |

**NOTE:** If neither `--volatile` nor `--amplified` are specified, both sets will be tested. Similarly, if neither `--unit` nor `--integration` are specified, both sets will be tested.

<br/>

# Tests Structure
## Folder structure overview
```
├── fixtures
|   ├── accounts.py
|   ├── contracts.py
|   ├── vaults.py
|   └── tokens.py
|
├── test_amplified/
|    ├── configs/
|    |   └── default.json
|    |   └── *.json
|    ├── integration/
|    |   └── test_*.py
|    └── unit/
|        └── test_*.py
|
├── test_common/
|    ├── integration/
|    |   └── test_*.py
|    └── unit/
|        └── test_*.py
|
└── test_volatile/
     ├── configs/
     |   └── default.json
     |   └── *.json
     ├── integration/
     |   └── test_*.py
     └── unit/
        └── test_*.py
```

## Test Config Files
Both the *volatile* and *amplified* tests run on Catalyst vaults which are configured according to the specification saved to `./config/default.json` within each main test directory (`test_volatile/` and `test_amplified/`).
- To select a different config file, use the `--config` flag together with the name of the config file (omit the `.json` extension). The config files must be located within the specified `./config` folders.
- The `volatile` and `amplified` tests load **different** config files. Each within its own directory.
- Each config file defines:
    - The token definitions to use (**minimum 4 tokens must be specified**)
    - The vault definitions to use (**if only one vault is provided, tests involving two vaults will not run**)
    - The amplification value (**only for amplified tests**)

<br/>

# Fixtures
All helper fixtures are defined within the `fixtures/` folder.
- `accounts.py`: Accounts with given roles (e.g. *deployer*).
- `contracts.py`: The *deployed* contracts that are used by the tests (e.g. *swap_factory*).
- `vaults.py`: The fixtures related to Catalyst vaults. 
- `tokens.py`: The tokens and token helpers that are used by the tests.
## Parametrized Fixtures
The `raw_config` fixture is parametrized over the different test configurations to run the tests against. Usually, this means a volatile and an amplified test configuration, but may be different because of the user selected test settings (e.g. using only the `--volatile` flag). The `raw_config` fixture exposes the **full** unprocessed config data for each configuration. Further fixtures handle the data from this fixture further:
| Fixture           | Description                                                                                              |
| ----------------- | -------------------------------------------------------------------------------------------------------- |
| `tokens_config`   | The 'tokens' definitions contained in `raw_config` (verified and processed).                             |
| `tokens`          | An array of deployed tokens, as defined on `tokens_config`.                                              |
| `group_config`    | The 'vaults' configuration contained in `raw_config` (verified and processed).                           |
| `group_vaults`    | An array of deployed vaults, as defined on `group_config`.                                               |
| `group_tokens`    | An array of the tokens contained by each of the vaults of `group_vaults`.                                |
| `swap_vault_type` | Identifies the current vault type, either `"volatile"` or `"amplified"`.                                 |
| `amplification`   | The amplification value with which the vaults have been initialized. Returns 10**18 for volatile vaults. |

Additionally, the `raw_config` fixture gets parametrized with further data according to the two following scenarios:
### Single Vault Tests
For tests involving a single vault, an extra field `vault_index` within `raw_config` gets parametrized over all the vaults available on the config file. Using this parameter, further fixtures are defined:
| Fixture        | Description                                                                                      |
| -------------- | ------------------------------------------------------------------------------------------------ |
| `vault_index`  | The index used within the current parametrization. Goes through all the available vault indexes. |
| `vault`        | The deployed vault from `group_vaults` at index `vault_index`.                                   |
| `vault_config` | The configuration data of the vault `vault`.                                                     |
| `vault_tokens` | The tokens contained by the vault `vault`.                                                       |
### Dual Vault Tests
For tests involving two vaults, two extra fields `vault_1_index` and `vault_2_index` within `raw_config` get parametrized according to the test settings. Further fixtures are defined for these two parameters. These are the same as those defined for *Single Vault Tests* (described shortly above), but with `vault_1_` and `vault_2_` prepended to the fixture names.

<br/>

# Markers
| Marker           | Description                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------ |
| `no_vault_param` | Don't parametrize the `vault` fixture more than once. (i.e. only use the first vault definition) |