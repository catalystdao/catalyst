import pytest
from brownie import reverts, convert, ZERO_ADDRESS
from brownie.test import given, strategy
from hypothesis.strategies import floats
import re
    
    
def test_get_templates_filled(
    catalyst_describer_filled, volatile_swap_pool_template, amplified_swap_pool_template
):
    templates = catalyst_describer_filled.get_whitelisted_templates()
    
    assert len(templates) == 2
    
    assert templates[0] == volatile_swap_pool_template.address
    assert templates[1] == amplified_swap_pool_template.address


def test_get_describer(
    catalyst_describer_registry_filled, catalyst_describer_blank
):
    describer = catalyst_describer_registry_filled.get_vault_describer(0)
    
    assert describer == catalyst_describer_blank.address
    
    
    describer = catalyst_describer_registry_filled.get_vault_describer(1)
    
    assert describer == ZERO_ADDRESS


def test_get_describers(
    catalyst_describer_registry_filled, catalyst_describer_blank
):
    describers = catalyst_describer_registry_filled.get_vault_describers()
    
    assert len(describers) == 1
    
    assert describers[0]  == catalyst_describer_blank.address