import pytest


pytest_plugins = [
    "fixtures.accounts",
    "fixtures.contracts",
    "fixtures.pools",
    "fixtures.tokens"
]


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



def pytest_generate_tests(metafunc):

    # Note that if "--vol-only" nor "--amp-only" are specified, all tests will run
    run_vol_tests = not metafunc.config.getoption("--amp-only")
    run_amp_tests = not metafunc.config.getoption("--vol-only")

    if run_vol_tests:
        pass

    if run_amp_tests:
        pass
            

@pytest.fixture(scope="session")
def pools_data(request):
    pass

@pytest.fixture(scope="session")
def pool_data(request):
    pass
