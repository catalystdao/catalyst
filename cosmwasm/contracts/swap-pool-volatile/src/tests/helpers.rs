use swap_pool_common::msg::InstantiateMsg;

pub const DEPLOYER_ADDR: &str = "deployer_addr";

pub fn mock_instantiate_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestPool".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        pool_fee: 10000u64,
        governance_fee: 50000u64,
        fee_administrator: "fee_administrator".to_string(),
        setup_master: "setup_master".to_string()
    }
}
