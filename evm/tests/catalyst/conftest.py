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


# Enable test isolation
@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


def pytest_addoption(parser):
    parser.addoption("--config", default="default", help="Load the a specific test config definition.")
    parser.addoption("--vol-only", action="store_true", help="Run only tests of the volatile pool.")
    parser.addoption("--amp-only", action="store_true", help="Run only tests of the amplified pool.")
    parser.addoption("--amplification", default=None, help="Override the config amplification constant.")



def pytest_configure(config):

    # Verify the parser options
    if config.getoption("--vol-only") and config.getoption("--amp-only"):
        raise Exception("Can't specify both --vol-only and --amp-only at the same time.")
    
    if config.getoption("--vol-only") and config.getoption("--amplification") is not None:
        raise Exception("--amplification cannot be specified when running volatile-only tests (--vol-only)")

    # Note that if "--vol-only" nor "--amp-only" are specified, all tests will run
    run_vol_tests = not config.getoption("--amp-only")
    run_amp_tests = not config.getoption("--vol-only")

    # Load config files
    config_name  = config.getoption("--config")
    project_path = get_loaded_projects()[0]._path
    
    if run_vol_tests:

        vol_config_path = project_path.joinpath(
            "tests", "catalyst", "test_volatile", "configs", config_name + ".json"
        )

        if not vol_config_path.is_file():
            raise Exception(f"Cannot file config file \'{config_name}.json\' for volatile tests.")
    
        with vol_config_path.open() as f:
            _test_config["volatile"] = json.load(f)
    
    
    if run_amp_tests:

        amp_config_path = project_path.joinpath(
            "tests", "catalyst", "test_amplified", "configs", config_name + ".json"
        )

        if not amp_config_path.is_file():
            raise Exception(f"Cannot file config file \'{config_name}.json\' for amplified tests.")
    
        with amp_config_path.open() as f:
            _test_config["amplified"] = json.load(f)




def pytest_generate_tests(metafunc):

    project_path  = get_loaded_projects()[0]._path
    test_path     = Path(metafunc.definition.fspath)
    rel_test_path = test_path.relative_to(project_path).parts[2:]   # Ignore the first two 'parts' of the test path, as the tests are under './tests/catalyst'

    if rel_test_path[0] == "test_volatile":
        config = _test_config["volatile"]
        swap_pool_type = "volatile"

    elif rel_test_path[0] == "test_amplified":
        config = _test_config["amplified"]
        swap_pool_type = "amplified"


    metafunc.parametrize("swap_pool_type", [swap_pool_type], scope="session")

    if "raw_config" in metafunc.fixturenames:
        metafunc.parametrize("raw_config", [config], indirect=True, scope="session")

    if "raw_pool_config" in metafunc.fixturenames:
        metafunc.parametrize("raw_pool_config", config["pools"], indirect=True, scope="session")
    


# Main parametrized fixture to expose the entire test_config as selected by the user
@pytest.fixture(scope="session")
def raw_config(request):
    yield request.param


# Main parametrized fixture to expose each pool from test_config as selected by the user
@pytest.fixture(scope="session")
def raw_pool_config(request):
    yield request.param

