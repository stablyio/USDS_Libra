// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::path::Path;

use compiler::compiler::compile_program;
use failure::prelude::*;
use lazy_static::lazy_static;
use types::account_address::AccountAddress;
use types::account_config::AccountResource;
use types::transaction::{Program, TransactionArgument};
use vm::file_format::CompiledProgram;

use crate::{client_proxy::*, commands::*, etoken_resource::ETokenResource};

lazy_static! {
    pub static ref ETOKEN_ISSUE_TEMPLATE: String = {include_str!("../move/eToken.mvir").to_string()};
    pub static ref ETOKEN_INIT_TEMPLATE: String = {include_str!("../move/init.mvir").to_string()};
    pub static ref ETOKEN_MINT_TEMPLATE: String = {include_str!("../move/mint.mvir").to_string()};
    pub static ref ETOKEN_TRANSFER_TEMPLATE: String = {include_str!("../move/peer_to_peer_transfer.mvir").to_string()};
    pub static ref ETOKEN_SELL_TEMPLATE: String = {include_str!("../move/sell.mvir").to_string()};
    pub static ref ETOKEN_BUY_TEMPLATE: String = {include_str!("../move/buy.mvir").to_string()};
}


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
            Box::new(HackCommandETokenIssue {}),
            Box::new(HackCommandETokenInit {}),
            Box::new(HackCommandETokenMint {}),
            Box::new(HackCommandETokenTransfer {}),
            Box::new(HackCommandETokenSell {}),
            Box::new(HackCommandETokenBuy {}),
        ];

        subcommand_execute(&params[0], commands, client, &params[1..]);
    }
}

pub struct HackCommandPublishModule {}

impl Command for HackCommandPublishModule {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["publish", "pub"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id> <module_path>"
    }
    fn get_description(&self) -> &'static str {
        "Publish module to an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
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

        execute_script(client, &address, &ETOKEN_ISSUE_TEMPLATE, vec![]).map(handler_result).map_err(handler_err).ok();
    }
}

pub struct HackCommandETokenIssue {}

impl Command for HackCommandETokenIssue {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_issue", "issue"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>"
    }
    fn get_description(&self) -> &'static str {
        "Issue EToken to an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 2 {
            println!("Invalid number of arguments for command");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };

        execute_script(client, &address, &ETOKEN_ISSUE_TEMPLATE, vec![]).map(|(compiled_program, seq)| {
            client.etoken_account = Some(address.clone());
            client.etoken_program = Some(compiled_program.clone());
            (compiled_program, seq)
        }).map(handler_result).map_err(handler_err).ok();
    }
}

// Init an account for accept etoken
pub struct HackCommandETokenInit {}

impl Command for HackCommandETokenInit {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_init", "init"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>"
    }
    fn get_description(&self) -> &'static str {
        "Init the account for accept EToken"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 2 {
            println!("Invalid number of arguments for command");
            return;
        }
        if client.etoken_account.is_none() {
            println!("Please issue etoken first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_INIT_TEMPLATE, vec![]).map(handler_result).map_err(handler_err).ok();
    }
}

// Mint etoken for an account
pub struct HackCommandETokenMint {}

impl Command for HackCommandETokenMint {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_mint", "mint"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id> <amount>"
    }
    fn get_description(&self) -> &'static str {
        "Mint etoken for an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if client.etoken_account.is_none() {
            println!("Please issue etoken first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let amount = match params[2].parse::<u64>() {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_MINT_TEMPLATE, vec![TransactionArgument::U64(amount)]).map(handler_result).map_err(handler_err).ok();
    }
}


// Transfer etoken to an account
pub struct HackCommandETokenTransfer {}

impl Command for HackCommandETokenTransfer {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_transfer", "transfer"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <account_ref_id>|<account_address> <amount>"
    }
    fn get_description(&self) -> &'static str {
        "Transfer etoken to an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 4 {
            println!("Invalid number of arguments for command");
            return;
        }
        if client.etoken_account.is_none() {
            println!("Please issue etoken first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let payee_address = match client.get_account_address_from_parameter(params[2]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let amount = match params[3].parse::<u64>() {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_TRANSFER_TEMPLATE, vec![TransactionArgument::Address(payee_address), TransactionArgument::U64(amount)]).map(handler_result).map_err(handler_err).ok();
    }
}


// Sell etoken and create an order
pub struct HackCommandETokenSell {}

impl Command for HackCommandETokenSell {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_sell", "sell"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <amount> <price>"
    }
    fn get_description(&self) -> &'static str {
        "Sell etoken and create an order"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 4 {
            println!("Invalid number of arguments for command");
            return;
        }
        if client.etoken_account.is_none() {
            println!("Please issue etoken first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let amount = match params[2].parse::<u64>() {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        let price = match params[3].parse::<u64>() {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid price", e.into());
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_SELL_TEMPLATE, vec![TransactionArgument::U64(amount), TransactionArgument::U64(price)]).map(handler_result).map_err(handler_err).ok();
    }
}

// Buy etoken from a order address
pub struct HackCommandETokenBuy {}

impl Command for HackCommandETokenBuy {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_buy", "buy"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <order_account_ref_id>|<order_account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Buy etoken from a order address"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if client.etoken_account.is_none() {
            println!("Please issue etoken first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let payee_address = match client.get_account_address_from_parameter(params[2]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_BUY_TEMPLATE, vec![TransactionArgument::Address(payee_address)]).map(handler_result).map_err(handler_err).ok();
    }
}

pub fn handler_err(e: Error) {
    report_error("execute command fail:", e);
}

pub fn handler_result(result: (CompiledProgram, IndexAndSequence)) {
    let index_and_seq = result.1;
    println!("Finished transaction!");
    println!(
        "To query for transaction status, run: query txn_acc_seq {} {} \
                     <fetch_events=true|false>",
        index_and_seq.account_index, index_and_seq.sequence_number
    );
}

pub fn execute_script(client: &mut ClientProxy, address: &AccountAddress, script_template: &str, args: Vec<TransactionArgument>) -> Result<(CompiledProgram, IndexAndSequence)> {
    let compiled_program = compile_script(script_template, client, &address)?;
    let is_blocking = true;
    println!("{}", compiled_program);
    let program = create_transaction_program(&compiled_program, args)?;
    let result = client.send_transaction(&address, program, None, None, is_blocking)?;
    return Ok((compiled_program.clone(), result));
}

pub fn compile_script(script_template: &str, client: &mut ClientProxy, address: &AccountAddress) -> Result<CompiledProgram> {
    let mut dependencies = vec![];
    dependencies.append(&mut compiler::util::build_stdlib());
    if let Some(program) = client.etoken_program.borrow() {
        program.modules.iter().for_each(|m| {
            dependencies.push(m.clone());
        });
    }
    let etoken_address = client.etoken_account.borrow().unwrap_or(address.clone());
    let program = parse_script(script_template, &etoken_address);
    match compile_program(address, &program, &dependencies) {
        Ok(p) => Ok(p),
        Err(e) => Err(e)
    }
}

pub fn parse_script(script_template: &str, etoken_address: &AccountAddress) -> compiler::parser::ast::Program {
    let mut address_str = "0x".to_owned();
    address_str.push_str(etoken_address.to_string().as_str());
    let script = script_template.replace("${etoken_address}", address_str.as_str());
    compiler::parser::parse_program(script.as_str()).unwrap()
}

fn create_transaction_program(program: &CompiledProgram, args: Vec<TransactionArgument>) -> Result<Program> {
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
    Ok(Program::new(script_blob, module_blobs, args))
}

/// Command to query latest account state from validator.
pub struct HackCommandGetLatestAccountState {}

impl HackCommandGetLatestAccountState {
    fn do_execute(&self, client: &mut ClientProxy, params: &[&str]) -> Result<()> {
        println!(">> Getting latest account state");
        match client.get_latest_account_state(&params) {
            Ok((acc, version)) => match acc {
                Some(blob) => {
                    let account_btree = blob.borrow().try_into()?;
                    let account_resource = AccountResource::make_from(&account_btree).unwrap_or(AccountResource::default());
                    let etoken_resource = client.etoken_account.map(|address|
                        ETokenResource::make_from(address, &account_btree).unwrap_or_default()
                    ).unwrap_or_default();


                    println!(
                        "Latest account state is: \n \
                     Account: {:#?}\n \
                     AccountResource: {:#?}\n \
                     ETokenResource: {:#?}\n \
                     Blockchain Version: {}\n",
                        client
                            .get_account_address_from_parameter(params[1])
                            .expect("Unable to parse account parameter"),
                        account_resource,
                        etoken_resource,
                        version,
                    );
                    let tree = BTreeMap::try_from(&blob).unwrap();
                    println!("AccountStateBlob Tree:");
                    tree.iter().map(|(k, v)| -> (String, String) {
                        let mut key: String = "".to_owned();
                        if k[0] == CODE_TAG {
                            key.push_str("code_")
                        } else if k[0] == RESOURCE_TAG {
                            key.push_str("res_");
                        }
                        key.push_str(hex::encode(k).as_str());
                        (key, hex::encode(v))
                    }).for_each(|(k, v)| {
                        println!("key:{:#?}, value:{:#?}", k, v);
                    })
                }
                None => {
                    println!("Account State is None");
                }
            },
            Err(e) => report_error("Error getting latest account state", e),
        };
        Ok(())
    }
}

const CODE_TAG: u8 = 0;
const RESOURCE_TAG: u8 = 1;

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
        match self.do_execute(client, params) {
            Ok(_) => {}
            Err(e) => {
                report_error("execute command fail:", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use types::account_address::AccountAddress;

    use crate::hack_commands::*;

    #[test]
    fn test_parse_script() {
        //println!("{:?}", AccountAddress::random());
        //println!("{:?}",AccountAddress::default().to_string());

        let program = parse_script(&ETOKEN_ISSUE_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_INIT_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_MINT_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_TRANSFER_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_SELL_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_BUY_TEMPLATE, &AccountAddress::random());
        println!("{:?}", program);
    }
}