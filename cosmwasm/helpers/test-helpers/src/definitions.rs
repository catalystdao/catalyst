use catalyst_types::Bytes32;

pub const CHAIN_INTERFACE       : &str = "chain_interface_addr";
pub const DEPLOYER              : &str = "deployer_addr";
pub const FACTORY_OWNER         : &str = "factory_owner_addr";
pub const SETUP_MASTER          : &str = "setup_master_addr";
pub const DEPOSITOR             : &str = "depositor_addr";
pub const WITHDRAWER            : &str = "withdrawer_addr";
pub const LOCAL_SWAPPER         : &str = "local_swapper_addr";
pub const SWAPPER_A             : &str = "swapper_a_addr";
pub const SWAPPER_B             : &str = "swapper_b_addr";
pub const SWAPPER_C             : &str = "swapper_c_addr";
pub const UNDERWRITER           : &str = "underwriter";

pub const VAULT_TOKEN_DENOM     : &str = "CAT";

pub const CHANNEL_ID            : Bytes32 = Bytes32([1; 32]);