import json
from pathlib import Path
import pytest

from brownie.project.main import get_loaded_projects


pytest_plugins = [
    "fixtures.accounts",
    "fixtures.contracts",
    "fixtures.pools",
    "fixtures.tokens"
]

_test_config = {
    "volatile"  : None,
    "amplified" : None
}

_run_unit_tests        = False
_run_integration_tests = False

_source_target_combinations_volatile  = []
_source_target_combinations_amplified = []


# Enable test isolation
@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


def source_pool_type_checker(value):
    # source-pool should be either one of ["all"], or a specific pool index

    try:
        # Check if a pool index
        int(value)
    except:
        # If not a pool index, check if it's an allowed string option
        if not value in ["all"]:
            raise pytest.UsageError("--source-pool should be either \"all\" or a pool index")
    
    return value


def target_pool_type_checker(value):
    # target-pool should be either one of ["next", "all"], or a specific pool index

    try:
        # Check if a pool index
        int(value)
    except:
        # If not a pool index, check if it's an allowed string option
        if not value in ["next", "all"]:
            raise pytest.UsageError("--target-pool should be either (\"next\" | \"all\") or a pool index")
    
    return value


def pytest_addoption(parser):
    parser.addoption("--config", default="default", help="Load the a specific test config definition.")
    parser.addoption("--volatile", action="store_true", help="Run only tests of the volatile pool.")
    parser.addoption("--amplified", action="store_true", help="Run only tests of the amplified pool.")
    parser.addoption("--amplification", default=None, help="Override the config amplification constant.")

    parser.addoption("--source-pool", default="0", type=source_pool_type_checker, help="Specify how to parametrize the source pool fixture.")
    parser.addoption("--target-pool", default="all", type=target_pool_type_checker, help="Specify how to parametrize the target pool fixture.")

    parser.addoption("--unit", action="store_true", help="Run only unit tests.")
    parser.addoption("--integration", action="store_true", help="Run only integration tests.")



def pytest_configure(config):
    global _run_unit_tests
    global _run_integration_tests
    global _source_target_combinations_volatile
    global _source_target_combinations_amplified

    # Note that if "--volatile" nor "--amplified" are specified, all tests will run
    run_all_tests = not config.getoption("--volatile") and not config.getoption("--amplified")
    run_vol_tests = run_all_tests or config.getoption("--volatile")
    run_amp_tests = run_all_tests or config.getoption("--amplified")
    
    if not run_amp_tests and config.getoption("--amplification") is not None:
        raise Exception("--amplification cannot be specified when amplified tests are not set to run.")

    
    # Note that if "--unit" nor "--integration" are specified, all tests will run
    run_unit_and_integration = not config.getoption("--unit") and not config.getoption("--integration")
    _run_unit_tests        = run_unit_and_integration or config.getoption("--unit")
    _run_integration_tests = run_unit_and_integration or config.getoption("--integration") 

    # Load config files
    config_name  = config.getoption("--config")
    project_path = get_loaded_projects()[0]._path
    
    if run_vol_tests:

        # Load volatile config file
        vol_config_path = project_path.joinpath(
            "tests", "catalyst", "test_volatile", "configs", config_name + ".json"
        )

        if not vol_config_path.is_file():
            raise Exception(f"Cannot file config file \'{config_name}.json\' for volatile tests.")
    
        with vol_config_path.open() as f:
            _test_config["volatile"] = json.load(f)
        
        # Compute the source-target pool combinations (indexes only)
        _source_target_combinations_volatile = compute_pool_combinations(
            config.getoption("--source-pool"),
            config.getoption("--target-pool"),
            len(_test_config["volatile"]["pools"])
        )
    
    
    if run_amp_tests:

        # Load amplified config file
        amp_config_path = project_path.joinpath(
            "tests", "catalyst", "test_amplified", "configs", config_name + ".json"
        )

        if not amp_config_path.is_file():
            raise Exception(f"Cannot file config file \'{config_name}.json\' for amplified tests.")
    
        with amp_config_path.open() as f:
            _test_config["amplified"] = json.load(f)
        
        # Compute the source-target pool combinations (indexes only)
        _source_target_combinations_amplified = compute_pool_combinations(
            config.getoption("--source-pool"),
            config.getoption("--target-pool"),
            len(_test_config["amplified"]["pools"])
        )


def pytest_ignore_collect(path, config):

    project_path  = get_loaded_projects()[0]._path
    test_path     = Path(path)
    rel_test_path = test_path.relative_to(project_path).parts[2:]   # Ignore the first two 'parts' of the test path, as the tests are under './tests/catalyst'


    if len(rel_test_path) == 0: return False    # Accept all tests on the root path (mostly for dev purposes, as no tests are planned to be there)


    if rel_test_path[0] == "test_volatile" and _test_config["volatile"] is None:
        return True

    if rel_test_path[0] == "test_amplified" and _test_config["amplified"] is None:
        return True

    if len(rel_test_path) == 1: return False    # Accept any other tests not catched by the conditions above (with path length == 1)


    if rel_test_path[1] == "unit" and not _run_unit_tests:
        return True
    
    if rel_test_path[1] == "integration" and not _run_integration_tests:
        return True



def pytest_generate_tests(metafunc):

    project_path  = get_loaded_projects()[0]._path
    test_path     = Path(metafunc.definition.fspath)
    rel_test_path = test_path.relative_to(project_path).parts[2:]   # Ignore the first two 'parts' of the test path, as the tests are under './tests/catalyst'

    if rel_test_path[0] == "test_volatile":
        config = _test_config["volatile"]
        swap_pool_type = "volatile"
        source_target_indexes = _source_target_combinations_volatile

    elif rel_test_path[0] == "test_amplified":
        config = _test_config["amplified"]
        swap_pool_type = "amplified"
        source_target_indexes = _source_target_combinations_amplified


    if "swap_pool_type" in metafunc.fixturenames:
        metafunc.parametrize("swap_pool_type", [swap_pool_type], scope="session")

    if "raw_config" in metafunc.fixturenames:
        metafunc.parametrize("raw_config", [config], indirect=True, scope="session")

    if "raw_pool_config" in metafunc.fixturenames:
        metafunc.parametrize("raw_pool_config", config["pools"], indirect=True, scope="session")
    
    if "source_target_indexes" in metafunc.fixturenames:
        metafunc.parametrize("source_target_indexes", source_target_indexes, scope="session")
    


# Main parametrized fixture to expose the entire test_config as selected by the user
@pytest.fixture(scope="session")
def raw_config(request):
    yield request.param


# Main parametrized fixture to expose each pool from test_config as selected by the user
@pytest.fixture(scope="session")
def raw_pool_config(request):
    yield request.param




def compute_pool_combinations(source_pool, target_pool, pool_count):
    
    if pool_count < 2:
        return []

    source_pool_indexes = []

    if source_pool == "all":
        source_pool_indexes = list(range(pool_count))
    else:
        try:
            s_idx = int(source_pool)
        except:
            raise Exception("Unable to compute pool combinations with the provided parameters.")

        if s_idx >= pool_count:
            raise Exception("The provided source pool index exceeds the pool count.")

        source_pool_indexes = [s_idx]


    if target_pool == "all":
        return [
            (s_idx, t_idx) \
            for s_idx in source_pool_indexes \
            for t_idx in range(pool_count) \
            if s_idx != t_idx
        ]
    elif target_pool == "next":
        return [(s_idx, (s_idx + 1) % pool_count) for s_idx in source_pool_indexes]
    else:
        try:
            t_idx = int(target_pool)
        except:
            raise Exception("Unable to compute pool combinations with the provided parameters.")

        if t_idx >= pool_count:
            raise Exception("The provided target pool index exceeds the pool count.")

        return [(s_idx, t_idx) for s_idx in source_pool_indexes]