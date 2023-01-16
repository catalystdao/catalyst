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
| `--amplification AMP_CONSTANT` | Override the amplification constant that is specified on the specified config file. (May only be set if amplified tests are set to be run.)  |
| `--source-pool PARAM_TYPE` | Specify how to parametrize the source pool fixture:<ul><li>`'all'`: Go through all the pools.</li><li>`POOL_INDEX`: Only the specified pool.</li></ul>  |
| `--target-pool PARAM_TYPE` | Specify how to parametrize the target pool fixture:<ul><li>`'all'`: Go through all the pools (skips the `source_pool`, i.e. avoid combinations with the same pool)</li><li>`'next'`: Use the *next* pool defined after the current `source_pool` (by index)</li><li>`POOL_INDEX`: Only the specified pool.</li></ul>  |
| `--unit`                       | Run unit tests.  |
| `--integration`                | Run integration tests.  |

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
    - The pool definitions to use (**minimum 2 pools must be specified**)
    - The amplification value (**only for amplified tests**)

<br/>

# Fixtures
## Parametrized Fixtures
There are 4 fixtures which get parametrized according to the specified test configuration and loaded config files:
| Fixture           | Description|
| ----              | ---------- |
| `raw_config`      | Exposes the **full** config config file. (i.e. tests with fixtures that depend on `raw_config` will run only once.) |
| `raw_pool_config` | Parametrizes **each** pool definition within the loaded config file. (i.e. tests with fixtures that depend on `raw_pool_config` will run once for every pool that is defined on the config file.) |
| `swap_pool_type`  | Identifies the type of pool being used for the tests. Either `"volatile"` or `"amplified"`|
| `source_target_indexes` | Paramterizes the source-target pool combinations (as tuples of indexes) |
## Other Fixtures
All helper fixtures are defined within the `fixtures/` folder.

- `accounts.py`: Accounts with given roles (e.g. *deployer*).
- `contracts.py`: The *deployed* contracts that are used by the tests (e.g. *swap_factory*).
- `pools.py`: Fixtures related to Catalyst pools. There are several **important fixtures** to note:
    - Based on `raw_config`:
        - `group_config`: The description of the pools contained in `raw_config` (verified and processed).
        - `group_pools`: An array of deployed pools, as defined on `group_config`.
        - `group_tokens`: An array of the deployed tokens contained by each of the pools of `group_pools`.
        - For every tuple of `source_target_indexes`:
            - `source_pool`: A pool from `group_pools` to act as a source pool. Selected according to `source_target_indexes`.
            - `source_pool_tokens`: The tokens handled by `source_pool`.
            - `target_pool`: A pool from `group_pools` to act as a target pool. Selected according to `source_target_indexes`.
            - `target_pool_tokens`: The tokens handled by `target_pool`.
    - Based on `raw_pool_config`:
        - `pool_config`: The description of a pool contained in `raw_pool_config` (verified and processed).
        - `pool`: A deployed pool, as defined on `pool_config`.
        - `pool_tokens`: An array of the deployed tokens contained by the pool of `pool`.
    - `deploy_pool`: Exposes a factory to deploy Catalyst pools.
- `tokens.py`: The tokens and token helpers that are used by the tests.
    - `tokens_config`: The description of the tokens contained in `raw_config` (verified and processed).
    - `tokens`: An array of deployed tokens, as defined on `tokens_config`
    - `create_token`: Exposes a factory to deploy tokens.
