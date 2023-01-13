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
There are 3 fixtures which get parametrized according to the loaded config file:
| Fixture           | Description|
| ----              | ---------- |
| `raw_config`      | Exposes the **full** config config file. (i.e. tests with fixtures that depend on `raw_config` will run only once.) |
| `raw_pool_config` | Parametrizes **each** pool definition within the loaded config file. (i.e. tests with fixtures that depend on `raw_pool_config` will run once for every pool that is defined on the config file.) |
| `swap_pool_type`  | Identifies the type of pool being used for the tests. Either `"volatile"` or `"amplified"`|
## Other Fixtures
All helper fixtures are defined within the `fixtures/` folder.

- `accounts.py`: Accounts with given roles (e.g. *deployer*).
- `contracts.py`: The *deployed* contracts that are used by the tests (e.g. *swap_factory*).
- `pools.py`: Fixtures related to Catalyst pools. There are several **important fixtures** to note:
    - Based on `raw_config`:
        - `group_config`: The description of the pools contained in `raw_config` (verified and processed).
        - `group_pools`: An array of deployed pools, as defined on `group_config`.
        - `group_tokens`: An array of the deployed tokens contained by each of the pools of `group_pools`.
    - Based on `raw_pool_config`:
        - `pool_config`: The description of a pool contained in `raw_pool_config` (verified and processed).
        - `pool`: A deployed pool, as defined on `pool_config`.
        - `pool_tokens`: An array of the deployed tokens contained by the pool of `pool`.
    - `deploy_pool`: Exposes a factory to deploy Catalyst pools.
- `tokens.py`: The tokens and token helpers that are used by the tests.
    - `tokens_config`: The description of the tokens contained in `raw_config` (verified and processed).
    - `tokens`: An array of deployed tokens, as defined on `tokens_config`
    - `create_token`: Exposes a factory to deploy tokens.
