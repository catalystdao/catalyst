mod test_query {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env}, Empty, StdError};

    use crate::contract::query;


    #[test]
    fn test_query_error() {

        let deps = mock_dependencies();
        let env = mock_env();

        let result = query(deps.as_ref(), env, Empty {});

        assert!(matches!(
            result.err().unwrap(),
            StdError::GenericErr { msg }
                if msg == "The router does not implement any queries.".to_string()
        ))
    }
}
