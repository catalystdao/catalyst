use std::str::FromStr;

use cosmwasm_std::{Event, Binary};

pub fn get_response_attribute<T: FromStr>(event: Event, attribute: &str) -> Result<T, String> {
    event.attributes
        .iter()
        .find(|attr| attr.key == attribute).ok_or("Attribute not found")?
        .value
        .parse::<T>().map_err(|_| "Parse error".to_string())
}

pub fn encode_payload_address(address: &[u8]) -> Binary {

    let address_len = address.len();
    if address_len > 64 {
        panic!()
    }

    let mut encoded_address: Vec<u8> = Vec::with_capacity(65);
    encoded_address.push(address_len as u8);             // Casting to u8 is safe, as address_len is <= 64

    if address_len != 64 {
        encoded_address.extend_from_slice(vec![0u8; 64-address_len].as_ref());
    }

    encoded_address.extend_from_slice(address.as_ref());

    Binary(encoded_address)
}