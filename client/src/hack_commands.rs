// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use failure::prelude::*;
use types::account_config::get_account_resource_or_default;
use types::transaction::Program;
use vm::file_format::CompiledProgram;
use vm_genesis::get_transaction_name;

use crate::{client_proxy::ClientProxy, commands::*};

/// Major command for hack operations.
pub struct HackCommand {}

impl Command for HackCommand {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["hack", "k"]
    }
    fn get_description(&self) -> &'static str {
        "Hack operations"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(HackCommandPublishModule {}),
            Box::new(HackCommandGetLatestAccountState {}),
        ];

        subcommand_execute(&params[0], commands, client, &params[1..]);
    }
}

/// Sub commands to query balance for the account specified.
pub struct HackCommandPublishModule {}

impl Command for HackCommandPublishModule {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["publish", "pub"]
    }
    fn get_params_help(&self) -> &'static str {
        "<module_path>"
    }
    fn get_description(&self) -> &'static str {
        "Publish module to an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for publish module");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let path = Path::new(params[2]);

        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                report_error("Unable to read file", e.into());
                return;
            }
        };
        let parsed_program = match compiler::parser::parse_program(&source) {
            Ok(p) => p,
            Err(e) => {
                report_error("parse program fail", e);
                return;
            }
        };

        let dependencies = compiler::util::build_stdlib();

        let compiled_program = match compiler::compiler::compile_program(&address, &parsed_program, &dependencies) {
            Ok(p) => p,
            Err(e) => {
                report_error("compile program fail.", e);
                return;
            }
        };
        let is_blocking = true;
        println!("{}", compiled_program);
        let program = match create_transaction_program(&compiled_program) {
            Ok(p) => p,
            Err(e) => {
                report_error("create transaction program fail.", e);
                return;
            }
        };
        match client.send_transaction(&address, program, None, None, is_blocking) {
            Ok(index_and_seq) => {
                if is_blocking {
                    println!("Finished transaction!");
                } else {
                    println!("Transaction submitted to validator");
                }
                println!(
                    "To query for transaction status, run: query txn_acc_seq {} {} \
                     <fetch_events=true|false>",
                    index_and_seq.account_index, index_and_seq.sequence_number
                );
            }
            Err(e) => report_error("Failed to perform transaction", e),
        }
    }
}

fn create_transaction_program(program: &CompiledProgram) -> Result<Program> {
    let mut script_blob = vec![];
    program.script.serialize(&mut script_blob)?;

    let module_blobs = program
        .modules
        .iter()
        .map(|m| {
            let mut module_blob = vec![];
            m.serialize(&mut module_blob)?;
            Ok(module_blob)
        })
        .collect::<Result<Vec<_>>>()?;

    // Currently we do not support transaction arguments in functional tests.
    Ok(Program::new(script_blob, module_blobs, vec![]))
}

/// Command to query latest account state from validator.
pub struct HackCommandGetLatestAccountState {}

impl Command for HackCommandGetLatestAccountState {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["account_state", "as"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Get the latest state for an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        println!(">> Getting latest account state");
        match client.get_latest_account_state(&params) {
            Ok((acc, version)) => match get_account_resource_or_default(&acc) {
                Ok(_) => {
                    let blob = acc.clone().unwrap();
                    let tree = BTreeMap::try_from(&blob).unwrap();
                    println!(
                        "Latest account state is: \n \
                     Account: {:#?}\n \
                     State: {:#?}\n \
                     Blockchain Version: {}\n",
                        client
                            .get_account_address_from_parameter(params[1])
                            .expect("Unable to parse account parameter"),
                        acc,
                        version,
                    );
                    println!("AccountStateBlob Tree:");
                    tree.iter().for_each(|(k,v)| {
                        println!("key:{:#?}, value:{:#?}", hex::encode(k), hex::encode(v));
                    })
                }
                Err(e) => report_error("Error converting account blob to account resource", e),
            },
            Err(e) => report_error("Error getting latest account state", e),
        }
    }
}
