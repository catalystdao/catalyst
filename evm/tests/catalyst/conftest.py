import json
from pathlib import Path
import pytest
from hypothesis import settings, Phase

from brownie.project.main import get_loaded_projects

from tests.catalyst.fixtures.vaults import MAX_VAULT_ASSETS


pytest_plugins = [
    "fixtures.accounts",
    "fixtures.contracts",
    "fixtures.vaults",
    "fixtures.tokens",
    "fixtures.modifiers",
]

_test_config = {"volatile": None, "amplified": None}

_run_unit_tests = False
_run_integration_tests = False

_parametrized_vaults = []
_parametrized_vault_pairs = []

_test_filters = None


# Enable test isolation
@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


def vault_type_checker(value):
    # vault should be either one of ["all"], or a specific vault index

    try:
        # Check if a vault index
        int(value)
    except:
        # If not a vault index, check if it's an allowed string option
        if not value in ["all"]:
            raise pytest.UsageError('--vault should be either "all" or a vault index')

    return value


def vault_1_type_checker(value):
    # Same as the vault_type_checker:
    # vault-1 should be either one of ["all"], or a specific vault index
    try:
        return vault_type_checker(value)
    except:
        # Rename the error
        raise pytest.UsageError('--vault-1 should be either "all" or a vault index')


def vault_2_type_checker(value):
    # vault-2 should be either one of ["next", "all"], or a specific vault index

    try:
        # Check if a vault index
        int(value)
    except:
        # If not a vault index, check if it's an allowed string option
        if not value in ["next", "all"]:
            raise pytest.UsageError(
                '--vault-2 should be either ("next" | "all") or a vault index'
            )

    return value


def pytest_addoption(parser):
    parser.addoption(
        "--config",
        default="default",
        help="Load the a specific test config definition.",
    )
    parser.addoption(
        "--volatile", action="store_true", help="Run only tests of the volatile vault."
    )
    parser.addoption(
        "--amplified",
        action="store_true",
        help="Run only tests of the amplified vault.",
    )
    parser.addoption(
        "--amplification",
        default=None,
        help="Override the config amplification constant.",
    )

    parser.addoption(
        "--vault",
        default=None,
        type=vault_type_checker,
        help="Specify how to parametrize the vault fixture. Defaults to 'all' if not running with --fast, otherwise it defaults to 0.",
    )
    parser.addoption(
        "--vault-1",
        default=None,
        type=vault_1_type_checker,
        help="Specify how to parametrize the vault-1 fixture. Defaults to 0.",
    )
    parser.addoption(
        "--vault-2",
        default=None,
        type=vault_2_type_checker,
        help="Specify how to parametrize the vault-2 fixture. Defaults to 'all' if not running with --fast, otherwise it defaults to 'next'.",
    )

    parser.addoption("--unit", action="store_true", help="Run only unit tests.")
    parser.addoption(
        "--integration", action="store_true", help="Run only integration tests."
    )

    parser.addoption(
        "--filter",
        default=None,
        action="append",
        help="Run only tests which match the provided filter ([filename][::[test-name]]). More than one filter may be specified.",
    )

    parser.addoption(
        "--fast",
        action="store_true",
        help="Do not test the specified strategies of the `@given` parametrized tests.",
    )


def pytest_configure(config):
    global _run_unit_tests
    global _run_integration_tests
    global _parametrized_vaults
    global _parametrized_vault_pairs
    global _test_filters

    # Note that if "--volatile" nor "--amplified" are specified, all tests will run
    run_all_tests = not config.getoption("--volatile") and not config.getoption(
        "--amplified"
    )
    run_vol_tests = run_all_tests or config.getoption("--volatile")
    run_amp_tests = run_all_tests or config.getoption("--amplified")

    if not run_amp_tests and config.getoption("--amplification") is not None:
        raise Exception(
            "--amplification cannot be specified when amplified tests are not set to run."
        )

    # Note that if "--unit" nor "--integration" are specified, all tests will run
    run_unit_and_integration = not config.getoption("--unit") and not config.getoption(
        "--integration"
    )
    _run_unit_tests = run_unit_and_integration or config.getoption("--unit")
    _run_integration_tests = run_unit_and_integration or config.getoption(
        "--integration"
    )

    # Load config files
    config_name = config.getoption("--config")
    project_path = get_loaded_projects()[0]._path

    if run_vol_tests:

        # Load volatile config file
        vol_config_path = project_path.joinpath(
            "tests", "catalyst", "test_volatile", "configs", config_name + ".json"
        )

        if not vol_config_path.is_file():
            raise Exception(
                f"Cannot file config file '{config_name}.json' for volatile tests."
            )

        with vol_config_path.open() as f:
            _test_config["volatile"] = json.load(f)
            verify_config(_test_config["volatile"], "volatile", config_name)

    if run_amp_tests:

        # Load amplified config file
        amp_config_path = project_path.joinpath(
            "tests", "catalyst", "test_amplified", "configs", config_name + ".json"
        )

        if not amp_config_path.is_file():
            raise Exception(
                f"Cannot file config file '{config_name}.json' for amplified tests."
            )

        with amp_config_path.open() as f:
            _test_config["amplified"] = json.load(f)
            verify_config(_test_config["amplified"], "amplified", config_name)

    # If both volatile and amplified configurations are loaded within the same test run, they must contain
    # the same number of vaults, otherwise user defined --vault-1 and --vault-2 may work for one of
    # the configurations but not for the other.
    if (
        run_vol_tests
        and run_amp_tests
        and len(_test_config["volatile"]["vaults"])
        != len(_test_config["amplified"]["vaults"])
    ):
        raise Exception(
            f"The number of vaults defined in {config_name}.json for both volatile and amplified definitions must match."
        )

    vault_count = len(
        (_test_config["volatile"] or _test_config["amplified"])["vaults"]
    )  # 'or' as 'volatile' config may be None (but in that case 'amplified' is not)

    # Compute the vault parametrization (indexes only)
    fast_option = config.getoption("--fast")
    vault_option = config.getoption("--vault")
    _parametrized_vaults = compute_parametrized_vaults(
        vault_option
        if vault_option is not None
        else (
            "all" if not fast_option else 0
        ),  # If --vault is not specified: use "all" if not running in --fast mode, otherwise use 0
        vault_count,
    )

    # Compute the vault-1/vault-2 combinations (indexes only)
    vault_1_option = config.getoption("--vault-1")
    vault_2_option = config.getoption("--vault-2")
    _parametrized_vault_pairs = compute_parametrized_vault_pairs(
        vault_1_option
        if vault_1_option is not None
        else 0,  # If --vault-1 is not specified: use 0
        vault_2_option
        if vault_2_option is not None
        else (
            "all" if not fast_option else "next"
        ),  # If --vault-2 is not specified: use "all" if not running in --fast mode, otherwise use "next"
        vault_count,
    )

    # Process filter config
    filter_config = config.getoption("--filter")
    if filter_config is not None:
        _test_filters = [
            filter_name.split("::", maxsplit=1) for filter_name in filter_config
        ]  # Convert filters into [file_name, test_name]. Note test_name might not be present (i.e. only [file_name])
        _test_filters = [
            filter_split + [None] * (2 - len(filter_split))
            for filter_split in _test_filters
        ]  # If a filter does not specify a test_name, set the value to None (i.e. always have [file_name, test_name])

    # Add custom pytest markers
    config.addinivalue_line(
        "markers",
        "no_vault_param: don't parametrize the 'vault' fixture more than once.",
    )

    # Configure hypothesis
    hypothesis_configure(config)


def hypothesis_configure(config):
    if config.getoption("--fast"):
        settings.register_profile("fast", phases=[Phase.explicit])
        settings.load_profile("fast")


def pytest_report_header(config):
    header_msgs = []

    if _test_config["volatile"] is not None:
        if len(_test_config["volatile"]["vaults"]) < 2:
            header_msgs.append(
                # Set text color to warning
                "\033[93m"
                + "WARNING: Tests involving 2 vaults will NOT RUN for volatile vault tests (need at least two vaults defined on the config file)."
                + "\033[0m"
            )

    if _test_config["amplified"] is not None:
        if len(_test_config["amplified"]["vaults"]) < 2:
            header_msgs.append(
                # Set text color to warning
                "\033[93m"
                + "WARNING: Tests involving 2 vaults will NOT RUN for amplified vault tests (need at least two vaults defined on the config file)."
                + "\033[0m"
            )

    if len(header_msgs) > 0:
        return "\n".join(header_msgs)


def pytest_ignore_collect(path, config):

    project_path = get_loaded_projects()[0]._path
    test_path = Path(path)
    rel_test_path = test_path.relative_to(project_path).parts[
        2:
    ]  # Ignore the first two 'parts' of the test path, as the tests are under './tests/catalyst'

    if len(rel_test_path) == 0:
        return None  # Accept all tests on the root path (mostly for dev purposes, as no tests are planned to be there)

    if rel_test_path[0] == "test_volatile" and _test_config["volatile"] is None:
        return True

    if rel_test_path[0] == "test_amplified" and _test_config["amplified"] is None:
        return True

    if len(rel_test_path) == 1:
        return None  # Accept any other tests not catched by the conditions above (with path length == 1)

    if rel_test_path[1] == "unit" and not _run_unit_tests:
        return True

    if rel_test_path[1] == "integration" and not _run_integration_tests:
        return True

    # Filter tests by file name
    if test_path.is_file() and _test_filters is not None:
        file_name = rel_test_path[-1]
        if not any(
            [test_file_filter in file_name for test_file_filter, _ in _test_filters]
        ):  # Check that the test file name is not matched by any of the filters
            return True


def pytest_generate_tests(metafunc):

    project_path = get_loaded_projects()[0]._path
    test_path = Path(metafunc.definition.fspath)
    rel_test_path = test_path.relative_to(project_path).parts[
        2:
    ]  # Ignore the first two 'parts' of the test path, as the tests are under './tests/catalyst'

    configs = []

    path_ref = rel_test_path[0]
    if (path_ref == "test_volatile" or path_ref == "test_common") and _test_config[
        "volatile"
    ] is not None:
        configs.append(
            {
                **_test_config["volatile"],
                "swap_vault_type": "volatile",
            }
        )

    if (path_ref == "test_amplified" or path_ref == "test_common") and _test_config[
        "amplified"
    ] is not None:
        configs.append(
            {
                **_test_config["amplified"],
                "swap_vault_type": "amplified",
            }
        )

    if "raw_config" in metafunc.fixturenames:

        # For single-vault tests
        if "vault_index" in metafunc.fixturenames:
            no_vault_param = (
                next(metafunc.definition.iter_markers(name="no_vault_param"), None)
                is not None
            )

            parametrized_configs = [
                {**config, "vault_index": i}
                for config in configs
                for i in (
                    _parametrized_vaults
                    if not no_vault_param
                    else _parametrized_vaults[:1]
                )
            ]

        # For dual-vault tests (i.e 'vault_1_index' + 'vault_2_index' combos)
        elif "vault_1_index" in metafunc.fixturenames:
            parametrized_configs = [
                {**config, "vault_1_index": indexes[0], "vault_2_index": indexes[1]}
                for config in configs
                for indexes in _parametrized_vault_pairs
            ]

        else:
            parametrized_configs = [*configs]

        metafunc.parametrize(
            "raw_config", parametrized_configs, ids=raw_config_ids_fn, scope="session"
        )


def pytest_collection_modifyitems(session, config, items):

    filtered_items = []
    for item in items:

        test_name = item.originalname
        test_file_name = Path(item.location[0]).parts[-1]

        # Filter tests by test name
        # For a test to be DESELECTED, at least a filter has to be specified (_test_filters is not None) AND the test name + file name combo must NOT match any of the filters
        if _test_filters is not None and not any(
            [
                test_file_filter in test_file_name
                and (test_name_filter is None or test_name_filter in test_name)
                for test_file_filter, test_name_filter in _test_filters
            ]
        ):
            config.hook.pytest_deselected(items=[item])
            continue

        filtered_items.append(item)

    # Make hypothesis parametrized tests run first
    filtered_items.sort(
        key=lambda item: "is_hypothesis_test" not in item.obj.__dir__()
    )  # If condition is True => not a hypothesis test. List is sorted with False (0) before True (1)

    # Modify items inplace
    items[:] = filtered_items


def compute_parametrized_vaults(vault_param_type, vault_count):

    if vault_param_type == "all":
        return list(range(vault_count))

    try:
        p_idx = int(vault_param_type)
    except:
        raise Exception(
            "Unable to compute the parametrized vaults with the provided parameters."
        )

    if p_idx >= vault_count:
        raise Exception("The provided vault index exceeds the vault count.")

    return [p_idx]


def compute_parametrized_vault_pairs(
    vault_1_param_type, vault_2_param_type, vault_count
):

    if vault_count < 2:
        return []

    vault_1_indexes = []

    if vault_1_param_type == "all":
        vault_1_indexes = list(range(vault_count))
    else:
        try:
            p1_idx = int(vault_1_param_type)
        except:
            raise Exception(
                "Unable to compute the parametrized vault pairs with the provided parameters."
            )

        if p1_idx >= vault_count:
            raise Exception("The provided vault-1 index exceeds the vault count.")

        vault_1_indexes = [p1_idx]

    if vault_2_param_type == "all":
        return [
            (p1_idx, p2_idx)
            for p1_idx in vault_1_indexes
            for p2_idx in range(vault_count)
            if p1_idx != p2_idx
        ]
    elif vault_2_param_type == "next":
        return [(p1_idx, (p1_idx + 1) % vault_count) for p1_idx in vault_1_indexes]
    else:
        try:
            p2_idx = int(vault_2_param_type)
        except:
            raise Exception(
                "Unable to compute the parametrized vault pairs with the provided parameters."
            )

        if p2_idx >= vault_count:
            raise Exception("The provided vault-2 index exceeds the vault count.")

        return [(p1_idx, p2_idx) for p1_idx in vault_1_indexes]


def verify_config(config, type, config_name):

    error_descriptor = f"CONFIG ERR ({type}, {config_name}.json):"

    # Verify tokens
    assert "tokens" in config, "No tokens defined in config file."

    token_count = len(config["tokens"])
    assert (
        token_count >= 4
    ), f"{error_descriptor} At least 4 tokens must be defined on the test config file."

    for i, token_config in enumerate(config["tokens"]):
        assert "name" in token_config and isinstance(
            token_config["name"], str
        ), f"{error_descriptor} 'name' field missing or of wrong type for token definition at position {i}."

        assert "symbol" in token_config and isinstance(
            token_config["symbol"], str
        ), f"{error_descriptor} 'symbol' field missing or of wrong type for token definition at position {i}."

        assert "decimals" in token_config and isinstance(
            token_config["decimals"], int
        ), f"{error_descriptor} 'decimals' field missing or of wrong type for token definition at position {i}."

        assert "supply" in token_config and isinstance(
            token_config["supply"], int
        ), f"{error_descriptor} 'supply' field missing or of wrong type for token definition at position {i}."

    # Verify vaults
    assert "vaults" in config, "No vaults defined in config file."

    assert (
        len(config["vaults"]) >= 1
    ), f"{error_descriptor} At least 1 vault must be defined on the test config file"

    for i, vault_config in enumerate(config["vaults"]):
        assert (
            "tokens" in vault_config
            and len(vault_config["tokens"]) > 0
            and len(vault_config["tokens"]) <= MAX_VAULT_ASSETS
        ), f"{error_descriptor} 'tokens' field missing or of wrong length for vault definition at position {i}."

        assert "initBalances" in vault_config and len(
            vault_config["initBalances"]
        ) == len(
            vault_config["tokens"]
        ), f"{error_descriptor} 'initBalances' field missing or of wrong length for vault definition at position {i}."

        assert "weights" in vault_config and len(vault_config["weights"]) == len(
            vault_config["tokens"]
        ), f"{error_descriptor} 'weights' field missing or of wrong length for vault definition at position {i}."

        assert "name" in vault_config and isinstance(
            vault_config["name"], str
        ), f"{error_descriptor} 'name' field missing or of wrong type for vault definition at position {i}."

        assert "symbol" in vault_config and isinstance(
            vault_config["symbol"], str
        ), f"{error_descriptor} 'symbol' field missing or of wrong type for vault definition at position {i}."

    # Verify that the tokens within the vaults are valid and are not reused
    tokens_used = [
        token_idx
        for vault_config in config["vaults"]
        for token_idx in vault_config["tokens"]
    ]
    assert len(set(tokens_used)) == len(
        tokens_used
    ), f"{error_descriptor} Tokens are reused across the vault definitions."
    assert all(
        token_idx < token_count for token_idx in tokens_used
    ), f"{error_descriptor} Mismatch between the token indexes used by the vaults and the count of tokens defined."

    if type == "amplified":
        assert (
            "amplification" in config
        ), f"{error_descriptor} 'amplification' missing from amplified config file."


def raw_config_ids_fn(args):

    # Generates ids with the format:
    #  - No vault param.:       [amp/vol]
    #  - Single vault param.:   [amp/vol].pX        where X stands for the vault index
    #  - Dual vault param.:     [amp/vol].pX1.pX2   where X1/X2 stand for the vault_1/vault_2 indexes

    # NOTE: using the '.' as separator between the displayed arguments within the id, as any further
    # chained parametrizations of other fixtures will append the further parametrizations ids with dashes,
    # making the final test id difficult to understand.

    swap_vault_type_id = args["swap_vault_type"][:3]

    if "vault_index" in args:
        return f"{swap_vault_type_id}.p{args['vault_index']}"

    elif "vault_1_index" in args:
        return f"{swap_vault_type_id}.p{args['vault_1_index']}.p{args['vault_2_index']}"

    else:
        return f"{swap_vault_type_id}"
