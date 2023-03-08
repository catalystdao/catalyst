#!/bin/bash

function generate_network_yaml() {
    echo "development:
  - name: Ganache-CLI
    id: chain_b
    cmd: ganache-cli
    host: http://127.0.0.1
    cmd_settings:
      port: 8546
      gas_limit: 12000000
      accounts: 10
      evm_version: istanbul
      mnemonic: brownie"
}

# create brownie network configuration
brownie networks delete chain_b
brownie networks import <(generate_network_yaml)

# run multichain test
brownie test tests/catalyst/manual_test_multichain_swap_b.py --network chain_b &
brownie test tests/catalyst/manual_test_multichain_swap_a.py

