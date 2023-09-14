mod test_types {
    use cosmwasm_std::{testing::{mock_dependencies_with_balance, mock_dependencies, mock_env, mock_info}, coin, Coin};

    use crate::{executors::types::{CoinAmount, Account}, state::lock_router};



    // CoinAmount Tests
    // ********************************************************************************************

    #[test]
    fn test_amount_get_amount_coin() {

        let set_coin = coin(987, "denom");



        // Tested action
        let amount = CoinAmount::Coin(set_coin.clone());
        let read_coin = amount.get_amount(
            &mock_dependencies().as_ref(),
            &mock_env()
        ).unwrap();



        assert_eq!(
            set_coin,
            read_coin
        );
    }


    #[test]
    fn test_amount_get_amount_balance() {

        let denom = "denom".to_string();
        let router_coin = coin(987, denom.clone());



        // Tested action
        let amount = CoinAmount::RouterBalance(denom);
        let read_coin = amount.get_amount(
            &mock_dependencies_with_balance(&[router_coin.clone()]).as_ref(),
            &mock_env()
        ).unwrap();



        assert_eq!(
            router_coin,
            read_coin
        );
    }


    #[test]
    fn test_amount_get_amount_balance_zero() {

        let denom = "denom".to_string();



        // Tested action
        let amount = CoinAmount::RouterBalance(denom.clone());
        let read_coin = amount.get_amount(
            &mock_dependencies().as_ref(),  // Don't set up balances
            &mock_env()
        ).unwrap();



        assert_eq!(
            Coin::new(0u128, denom),
            read_coin
        );
    }
    


    // Account Tests
    // ********************************************************************************************

    #[test]
    fn test_account_get_address_string() {

        let account_address = "some-account".to_string();



        // Tested action
        let account = Account::Address(account_address.clone());
        let read_address = account.get_address(
            &mock_dependencies().as_ref(),
            &mock_env()
        ).unwrap();



        assert_eq!(
            read_address,
            account_address
        )

    }


    #[test]
    fn test_account_get_address_router() {



        // Tested action
        let account = Account::Router;
        let read_address = account.get_address(
            &mock_dependencies().as_ref(),
            &mock_env()
        ).unwrap();



        assert_eq!(
            read_address,
            mock_env().contract.address
        )

    }


    #[test]
    fn test_account_get_address_sender() {

        let mut deps = mock_dependencies();
        let sender = "sender-address";

        // Lock the router, as the 'sender' is saved on the lock
        lock_router(&mut deps.as_mut(), mock_info(sender, &[])).unwrap();



        // Tested action
        let account = Account::Sender;
        let read_address = account.get_address(
            &deps.as_ref(),
            &mock_env()
        ).unwrap();



        assert_eq!(
            read_address,
            sender
        )

    }


    #[test]
    fn test_account_get_address_sender_no_lock() {

        let deps = mock_dependencies();

        // ! Do not lock the router



        // Tested action
        let account = Account::Sender;
        let result = account.get_address(
            &deps.as_ref(),
            &mock_env()
        );



        assert!(
            result.is_err()
        );

    }
}
