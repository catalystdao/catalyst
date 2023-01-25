# Run Tests
To run all the tests, from the EVM directory, use:
```
brownie test tests/catalyst
```

## Flags
The following flags can be used to run specific collections/configurations of the tests:
| Flag                           | Description|
| ----                           | ---------- |
| `--config CONFIG_FILE_NAME`    | Use a specific config file ('default' used if not specified).  |
| `--volatile`                   | Run tests for volatile pools.  |
| `--amplified`                  | Run tests for amplified pools  |
| `--amplification VALUE` | Override the amplification constant that is specified on the specified config file. (May only be set if amplified tests are set to be run.)  |
| `--pool PARAM_TYPE` | Specify how to parametrize the pool fixture:<ul><li>`'all'`: Go through all the pools.</li><li>`POOL_INDEX`: Only the specified pool.</li></ul>  |
| `--pool-1 PARAM_TYPE` | Specify how to parametrize the pool_1 fixture:<ul><li>`'all'`: Go through all the pools.</li><li>`POOL_INDEX`: Only the specified pool.</li></ul>  |
| `--pool-2 PARAM_TYPE` | Specify how to parametrize the pool_2 fixture:<ul><li>`'all'`: Go through all the pools (skips the `pool_1`, i.e. avoid combinations with the same pool)</li><li>`'next'`: Use the *next* pool defined after the current `pool_1` (by index)</li><li>`POOL_INDEX`: Only the specified pool.</li></ul>  |
| `--unit`                       | Run unit tests.  |
| `--integration`                | Run integration tests.  |
| `--filter FILTER`              | Filter the collected tests according to the provided filter (filtered by string inclusion): [file-name][::[test-name]] |

**NOTE:** If neither `--volatile` nor `--amplified` are specified, both sets will be tested. Similarly, if neither `--unit` nor `--integration` are specified, both sets will be tested.

<br/>

# Tests Structure
## Folder structure overview
```
├── fixtures
|   ├── accounts.py
|   ├── contracts.py
|   ├── pools.py
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
Both the *volatile* and *amplified* tests run on Catalyst pools which are configured according to the specification saved to `./config/default.json` within each main test directory (`test_volatile/` and `test_amplified/`).
- To select a different config file, use the `--config` flag together with the name of the config file (omit the `.json` extension). The config files must be located within the specified `./config` folders.
- The `volatile` and `amplified` tests load **different** config files. Each within its own directory.
- Each config file defines:
    - The token definitions to use (**minimum 4 tokens must be specified**)
    - The pool definitions to use (**if only one pool is provided, tests involving two pools will not run**)
    - The amplification value (**only for amplified tests**)

<br/>

# Fixtures
All helper fixtures are defined within the `fixtures/` folder.
- `accounts.py`: Accounts with given roles (e.g. *deployer*).
- `contracts.py`: The *deployed* contracts that are used by the tests (e.g. *swap_factory*).
- `pools.py`: The fixtures related to Catalyst pools. 
- `tokens.py`: The tokens and token helpers that are used by the tests.
## Parametrized Fixtures
The `raw_config` fixture is parametrized over the different test configurations to run the tests against. Usually, this means a volatile and an amplified test configuration, but may be different because of the user selected test settings (e.g. using only the `--volatile` flag). The `raw_config` fixture exposes the **full** unprocessed config data for each configuration. Further fixtures handle the data from this fixture further:
| Fixture           | Description|
| ----              | ---------- |
| `tokens_config`   | The 'tokens' definitions contained in `raw_config` (verified and processed). |
| `tokens`          | An array of deployed tokens, as defined on `tokens_config`. |
| `group_config`    | The 'pools' configuration contained in `raw_config` (verified and processed). |
| `group_pools`     | An array of deployed pools, as defined on `group_config`. |
| `group_tokens`    | An array of the tokens contained by each of the pools of `group_pools`. |
| `swap_pool_type`  | Identifies the current pool type, either `"volatile"` or `"amplified"`. |
| `amplification`   | The amplification value at which to the pools have been initialized. Returns None for volatile pools. |

Additionally, the `raw_config` fixture gets parametrized with further data according to the two following scenarios:
### Single Pool Tests
For tests involving a single pool, an extra field `pool_index` within `raw_config` gets parametrized over all the pools available on the config file. Using this parameter, further fixtures are defined:
| Fixture           | Description|
| ----              | ---------- |
| `pool_index`        | The index used within the current parametrization. Goes through all the available pool indexes. |
| `pool`              | The deployed pool from `group_pools` at index `pool_index`. |
| `pool_config`       | The configuration data of the pool `pool`. |
| `pool_tokens`       | The tokens contained by the pool `pool`. |
### Dual Pool Tests
For tests involving two pools, two extra fields `pool_1_index` and `pool_2_index` within `raw_config` get parametrized according to the test settings. Further fixtures are defined for these two parameters. These are the same as those defined for *Single Pool Tests* (described shortly above), but with `pool_1_` and `pool_2_` prepended to the fixture names.
