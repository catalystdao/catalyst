import pytest
from brownie import reverts, convert, ZERO_ADDRESS
from brownie.test import given, strategy
from hypothesis.strategies import floats
import re
    
    
def test_get_template_abi(
    catalyst_describer_filled
):
    templates = catalyst_describer_filled.get_whitelisted_templates()
    
    assert len(templates) > 0
    
    for template in templates:
        assert catalyst_describer_filled.get_vault_abi_version(template) == 1



def test_get_template_abi_blank(
    catalyst_describer_filled, accounts
):
    assert catalyst_describer_filled.get_vault_abi_version(accounts[2]) == -1