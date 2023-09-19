mod test_cancel_swap_commands {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env}, Binary};

    use crate::{executors::cancel_swap::set_cancel_swap_state, commands::{CommandResult, execute_command, CommandMsg}};



    #[test]
    fn test_allow_cancel_command_with_cancel_unset() {

        let deps = mock_dependencies();

        let authority = "authority";
        let identifier = Binary("id".as_bytes().to_vec());



        // Tested action
        let command_result = execute_command(
            &deps.as_ref(),
            &mock_env(),
            CommandMsg::AllowCancel {
                authority: authority.to_string(),
                identifier: identifier.clone()
            }
        ).unwrap();



        // Verify the check is successful
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_allow_cancel_command_with_cancel_set() {

        let mut deps = mock_dependencies();

        let authority = "authority";
        let identifier = Binary("id".as_bytes().to_vec());

        // Set cancel state
        set_cancel_swap_state(
            &mut deps.as_mut(),
            authority.to_string(),
            identifier.clone(),
            true
        ).unwrap();



        // Tested action
        let command_result = execute_command(
            &deps.as_ref(),
            &mock_env(),
            CommandMsg::AllowCancel {
                authority: authority.to_string(),
                identifier: identifier.clone()
            }
        ).unwrap();



        // Verify the check is unsuccessful
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.clone().err().unwrap() == format!(
                    "Swap cancelled (authority {}, identifier {})",
                    authority,
                    identifier.to_base64()
                )
        ));

    }
}
