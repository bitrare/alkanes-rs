use alkanes_runtime::{declare_alkane, message::MessageDispatch, runtime::AlkaneResponder};
use alkanes_support::{
    cellpack::Cellpack,
    parcel::{AlkaneTransfer, AlkaneTransferParcel},
    response::CallResponse,
};
use anyhow::{anyhow, Result};
use metashrew_support::compat::{to_arraybuffer_layout, to_passback_ptr};
use sha2::{Digest, Sha256};
#[allow(unused_imports)]
use {
    alkanes_runtime::{imports::__request_transaction, println, stdio::stdout},
    std::fmt::Write,
};

#[derive(Default)]
pub struct LoggerAlkane(());

#[derive(MessageDispatch)]
enum LoggerAlkaneMessage {
    #[opcode(2)]
    SelfCall,

    #[opcode(3)]
    CheckIncoming,

    #[opcode(4)]
    MintTokens,

    #[opcode(5)]
    #[returns(Vec<u8>)]
    ReturnData1,

    #[opcode(50)]
    GetTransaction,

    #[opcode(78)]
    HashLoop,

    #[opcode(99)]
    #[returns(Vec<u8>)]
    ReturnDefaultData,

    #[opcode(11)]
    ProcessNumbers { numbers: Vec<u128> },

    #[opcode(12)]
    ProcessStrings { strings: Vec<String> },

    #[opcode(13)]
    #[returns(Vec<u128>)]
    GetNumbers,

    #[opcode(14)]
    #[returns(Vec<String>)]
    GetStrings,

    #[opcode(15)]
    ProcessNestedVec { nested: Vec<Vec<u128>> },
}

impl LoggerAlkane {
    fn self_call(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        response.data = self
            .call(
                &Cellpack {
                    target: context.myself.clone(),
                    inputs: vec![50],
                },
                &AlkaneTransferParcel::default(),
                self.fuel(),
            )?
            .data;

        Ok(response)
    }

    fn check_incoming(&self) -> Result<CallResponse> {
        let context = self.context()?;

        if context.incoming_alkanes.0.len() != 1 {
            println!("{:#?}", context.incoming_alkanes.0);
            return Err(anyhow!("received either 0 or more than 1 alkane"));
        } else {
            return Ok(CallResponse::default());
        }
    }

    fn mint_tokens(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        response.alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: 100u128,
        });

        Ok(response)
    }

    fn return_data_1(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        response.data = vec![0x05, 0x06, 0x07, 0x08];

        Ok(response)
    }

    fn get_transaction(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        self.transaction();

        Ok(response)
    }

    fn hash_loop(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        let mut data = vec![0x01, 0x02];
        loop {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let buffer = hasher.finalize();
            data.extend(&buffer);
            if !"1".is_ascii() {
                break;
            }
        }

        Ok(response)
    }

    fn return_default_data(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        response.data = vec![0x01, 0x02, 0x03, 0x04];

        Ok(response)
    }

    fn process_numbers(&self, numbers: Vec<u128>) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        // Sum the numbers and store in response data
        let sum: u128 = numbers.iter().sum();
        response.data = sum.to_le_bytes().to_vec();

        Ok(response)
    }

    fn process_strings(&self, strings: Vec<String>) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        // Concatenate the strings and store in response data
        let concat = strings.join(",");
        response.data = concat.into_bytes();

        Ok(response)
    }

    fn get_numbers(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        // Return a sample vector of numbers
        let numbers = vec![1u128, 2u128, 3u128];
        let mut data = Vec::new();

        // First write the length
        data.extend_from_slice(&(numbers.len() as u128).to_le_bytes());

        // Then write each number
        for num in numbers {
            data.extend_from_slice(&num.to_le_bytes());
        }

        response.data = data;

        Ok(response)
    }

    fn get_strings(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        // Return a sample vector of strings
        let strings = vec!["hello".to_string(), "world".to_string()];
        let mut data = Vec::new();

        // First write the length
        data.extend_from_slice(&(strings.len() as u128).to_le_bytes());

        // Then write each string with null terminator
        for s in strings {
            let mut bytes = s.into_bytes();
            bytes.push(0); // Null terminator
            data.extend_from_slice(&bytes);
        }

        response.data = data;

        Ok(response)
    }

    fn process_nested_vec(&self, nested: Vec<Vec<u128>>) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        // Count total elements in the nested vector
        let total_elements: usize = nested.iter().map(|v| v.len()).sum();
        response.data = (total_elements as u128).to_le_bytes().to_vec();

        Ok(response)
    }
}

impl AlkaneResponder for LoggerAlkane {
    fn execute(&self) -> Result<CallResponse> {
        // The opcode extraction and dispatch logic is now handled by the declare_alkane macro
        // This method is still required by the AlkaneResponder trait, but we can just return an error
        // indicating that it should not be called directly
        Err(anyhow!(
            "This method should not be called directly. Use the declare_alkane macro instead."
        ))
    }
}

// Use the new macro format
declare_alkane! {
    impl AlkaneResponder for LoggerAlkane {
        type Message = LoggerAlkaneMessage;
    }
}
