use crate::tests::helpers::clear;
use crate::tests::std::alkanes_std_auth_token_build;
use crate::tests::std::alkanes_std_genesis_alkane_build;
use crate::tests::std::alkanes_std_genesis_protorune_build;
use crate::tests::std::alkanes_std_merkle_distributor_build;
use crate::tests::std::alkanes_std_orbital_build;
use crate::tests::std::alkanes_std_owned_token_build;
use crate::tests::std::alkanes_std_proxy_build;
use crate::tests::std::alkanes_std_test_build;
use crate::tests::std::alkanes_std_upgradeable_build;
use crate::vm::fuel::FuelTank;
use crate::vm::instance::AlkanesInstance;
use crate::vm::runtime::AlkanesRuntimeContext;
use alkanes::vm::fuel::VirtualFuelBytes;
use alkanes_support::constants::AUTH_TOKEN_FACTORY_ID;
use anyhow::Result;
#[allow(unused_imports)]
use metashrew::{ println, stdio::{ stdout, Write } };
use protorune::message::MessageContextParcel;
use protorune_support::rune_transfer::RuneTransfer;
use protorune_support::balance_sheet::BalanceSheet;
use protorune::message::MessageContext;
use protorune::test_helpers::create_block_with_rune_tx;
use crate::view::meta_safe;
use protorune::Protorune;
use serde_json::{ json, Value };
use std::sync::{ Arc, Mutex };
use alkanes_support::cellpack::Cellpack;
use crate::index_block;
use alkanes_support::id::AlkaneId;
use crate::tests::helpers::{ self as alkane_helpers, init_with_multiple_cellpacks_with_tx };

struct MyMessageContext(());

impl MessageContext for MyMessageContext {
    fn handle(_parcel: &MessageContextParcel) -> Result<(Vec<RuneTransfer>, BalanceSheet)> {
        let ar: Vec<RuneTransfer> = vec![];
        Ok((ar, BalanceSheet::default()))
    }
    fn protocol_tag() -> u128 {
        100
    }
}

fn test_contract_abi(
    contract_name: &str,
    contract_bytes: Vec<u8>,
    expected_methods: Vec<(&str, u128, Vec<(&str, &str)>)>
) -> Result<()> {
    let context = Arc::new(Mutex::new(AlkanesRuntimeContext::default()));

    // Create a new instance of the contract
    let mut instance = AlkanesInstance::from_alkane(context, Arc::new(contract_bytes), 100000000)?;

    // Call the __meta function to get the ABI
    let abi_bytes = instance.call_meta()?;

    // Convert the ABI bytes to a string and parse as JSON
    let abi_string = String::from_utf8(abi_bytes.clone())?;
    let abi_json: Value = serde_json::from_slice(&abi_bytes)?;

    // Print the ABI for debugging
    println!("{} ABI: {}", contract_name, abi_string);

    // Verify the contract name
    assert_eq!(abi_json["contract"], contract_name);

    // Verify that methods array exists
    assert!(abi_json["methods"].is_array());
    let methods = abi_json["methods"].as_array().unwrap();

    // Verify that we have the expected number of methods
    assert_eq!(
        methods.len(),
        expected_methods.len(),
        "Unexpected number of methods in ABI for {}",
        contract_name
    );

    // Verify each method
    for (expected_name, expected_opcode, expected_params) in expected_methods {
        // Find the method in the ABI
        let method = methods
            .iter()
            .find(|m| m["name"] == expected_name)
            .unwrap_or_else(|| {
                panic!("Method {} not found in ABI for {}", expected_name, contract_name)
            });

        // Verify the opcode
        assert_eq!(
            method["opcode"].as_u64().unwrap() as u128,
            expected_opcode,
            "Incorrect opcode for method {} in {}",
            expected_name,
            contract_name
        );

        // Verify the parameters
        let params = method["params"].as_array().unwrap();
        assert_eq!(
            params.len(),
            expected_params.len(),
            "Incorrect number of parameters for method {} in {}",
            expected_name,
            contract_name
        );

        // Verify each parameter type and name
        for (i, (expected_param_name, expected_param_type)) in expected_params.iter().enumerate() {
            assert_eq!(
                params[i]["type"].as_str().unwrap(),
                *expected_param_type,
                "Incorrect parameter type at index {} for method {} in {}",
                i,
                expected_name,
                contract_name
            );

            assert_eq!(
                params[i]["name"].as_str().unwrap(),
                *expected_param_name,
                "Incorrect parameter name at index {} for method {} in {}",
                i,
                expected_name,
                contract_name
            );
        }
    }

    Ok(())
}

fn test_meta_call() -> Result<()> {
    clear();
    let block_height = 840_000;

    // Create test blocks with cellpacks
    let test_cellpack = Cellpack {
        target: AlkaneId { block: 1, tx: 0 },
        inputs: vec![0, 1, 1000],
    };
    let mint_test_cellpack = Cellpack {
        target: AlkaneId { block: 2, tx: 1 },
        inputs: vec![1, 1000],
    };
    let auth_cellpack = Cellpack {
        target: AlkaneId {
            block: 3,
            tx: AUTH_TOKEN_FACTORY_ID,
        },
        inputs: vec![100],
    };

    // Initialize test environment with contracts
    let test_block = init_with_multiple_cellpacks_with_tx(
        vec![
            alkanes_std_auth_token_build::get_bytes(),
            alkanes_std_owned_token_build::get_bytes(),
            vec![]
        ],
        vec![auth_cellpack, test_cellpack, mint_test_cellpack]
    );

    index_block(&test_block, block_height)?;

    // Create a properly formatted message context parcel
    let parcel = MessageContextParcel {
        block: test_block,
        height: block_height,
        calldata: vec![2, 1], // Targeting the second contract (owned_token
        ..Default::default()
    };

    // Call meta_safe with the properly formatted parcel
    let abi_bytes = meta_safe(&parcel)?;

    // Verify the response
    let abi_string = String::from_utf8(abi_bytes.clone())?;
    let abi_json: Value = serde_json::from_slice(&abi_bytes)?;

    // Add some basic assertions
    assert!(abi_json.is_object(), "ABI should be a valid JSON object");
    assert!(abi_json.get("methods").is_some(), "ABI should contain methods");

    println!("ABI: {}", abi_string);
    Ok(())
}

fn test_owned_token_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![("auth_token_units", "u128"), ("token_units", "u128")]),
        ("mint", 77, vec![("token_units", "u128")]),
        ("get_name", 99, vec![]),
        ("get_symbol", 100, vec![]),
        ("get_total_supply", 101, vec![]),
        ("get_data", 1000, vec![])
    ];

    test_contract_abi("OwnedToken", alkanes_std_owned_token_build::get_bytes(), expected_methods)
}

#[test]
fn test_auth_token_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![("amount", "u128")]),
        ("authenticate", 1, vec![]),
        ("get_name", 99, vec![]),
        ("get_symbol", 100, vec![])
    ];

    test_contract_abi("AuthToken", alkanes_std_auth_token_build::get_bytes(), expected_methods)
}

#[test]
fn test_proxy_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![]),
        ("call_witness", 1, vec![("witness_index", "u128")]),
        ("delegatecall_witness", 2, vec![("witness_index", "u128")]),
        ("call_inputs", 3, vec![]),
        ("delegatecall_inputs", 4, vec![])
    ];

    test_contract_abi("Proxy", alkanes_std_proxy_build::get_bytes(), expected_methods)
}

#[test]
fn test_upgradeable_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        (
            "initialize",
            0x7fff,
            vec![("block", "u128"), ("tx", "u128"), ("auth_token_units", "u128")],
        ),
        ("upgrade", 0x7ffe, vec![("block", "u128"), ("tx", "u128")]),
        ("delegate", 0x7ffd, vec![])
    ];

    test_contract_abi("Upgradeable", alkanes_std_upgradeable_build::get_bytes(), expected_methods)
}

#[test]
fn test_logger_alkane_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("self_call", 2, vec![]),
        ("check_incoming", 3, vec![]),
        ("mint_tokens", 4, vec![]),
        ("return_data_1", 5, vec![]),
        ("get_transaction", 50, vec![]),
        ("hash_loop", 78, vec![]),
        ("return_default_data", 99, vec![])
    ];

    test_contract_abi("LoggerAlkane", alkanes_std_test_build::get_bytes(), expected_methods)
}

#[test]
fn test_orbital_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![]),
        ("get_name", 99, vec![]),
        ("get_symbol", 100, vec![]),
        ("get_total_supply", 101, vec![]),
        ("get_data", 1000, vec![])
    ];

    test_contract_abi("Orbital", alkanes_std_orbital_build::get_bytes(), expected_methods)
}

#[test]
fn test_merkle_distributor_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![("length", "u128"), ("root_bytes", "u128")]),
        ("claim", 1, vec![])
    ];

    test_contract_abi(
        "MerkleDistributor",
        alkanes_std_merkle_distributor_build::get_bytes(),
        expected_methods
    )
}

#[test]
fn test_genesis_alkane_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![]),
        ("mint", 77, vec![]),
        ("get_name", 99, vec![]),
        ("get_symbol", 100, vec![]),
        ("get_total_supply", 101, vec![])
    ];

    test_contract_abi(
        "GenesisAlkane",
        alkanes_std_genesis_alkane_build::get_bytes(),
        expected_methods
    )
}

#[test]
fn test_genesis_protorune_abi() -> Result<()> {
    clear();

    // Expected methods with their opcodes and parameter names and types
    let expected_methods = vec![
        ("initialize", 0, vec![]),
        ("mint", 77, vec![]),
        ("get_name", 99, vec![]),
        ("get_symbol", 100, vec![]),
        ("get_total_supply", 101, vec![])
    ];

    test_contract_abi(
        "GenesisProtorune",
        alkanes_std_genesis_protorune_build::get_bytes(),
        expected_methods
    )
}

#[test]
fn test_all_abis() -> Result<()> {
    test_meta_call()?;
    test_owned_token_abi()?;
    test_auth_token_abi()?;
    test_proxy_abi()?;
    test_upgradeable_abi()?;
    test_logger_alkane_abi()?;
    test_orbital_abi()?;
    test_merkle_distributor_abi()?;
    test_genesis_alkane_abi()?;
    test_genesis_protorune_abi()?;
    Ok(())
}
