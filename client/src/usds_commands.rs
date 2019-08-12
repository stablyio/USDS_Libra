// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::path::Path;

use bytecode_verifier::VerifiedModule;
use bytecode_verifier::verifier::VerifiedProgram;
use canonical_serialization::SimpleSerializer;
use compiler::Compiler;
use failure::prelude::*;
use lazy_static::lazy_static;
use types::access_path::AccessPath;
use types::account_address::AccountAddress;
use types::account_config::AccountResource;
use types::byte_array::ByteArray;
use types::transaction::{Program, RawTransaction, TransactionArgument};
use types::write_set::{WriteOp, WriteSetMut};
use vm::access::ScriptAccess;
use vm::file_format::{CompiledProgram, FunctionSignature, SignatureToken};

use crate::{client_proxy::*, commands::*, resource::ETokenResource, account_state::AccountState};
use itertools::Itertools;

lazy_static! {
    pub static ref ETOKEN_ISSUE_TEMPLATE: String = {include_str!("../move/eToken.mvir").to_string()};
    pub static ref ETOKEN_INIT_TEMPLATE: String = {include_str!("../move/init.mvir").to_string()};
    pub static ref ETOKEN_MINT_TEMPLATE: String = {include_str!("../move/mint.mvir").to_string()};
    pub static ref ETOKEN_TRANSFER_TEMPLATE: String = {include_str!("../move/peer_to_peer_transfer.mvir").to_string()};
    pub static ref ETOKEN_SELL_TEMPLATE: String = {include_str!("../move/sell.mvir").to_string()};
    pub static ref ETOKEN_BUY_TEMPLATE: String = {include_str!("../move/buy.mvir").to_string()};
    pub static ref ETOKEN_BURN_TEMPLATE: String = {include_str!("../move/burn.mvir").to_string()};
}


/// Major command for usds operations.
pub struct USDSCommand {}

impl Command for USDSCommand {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["usds", "k"]
    }
    fn get_description(&self) -> &'static str {
        "USDS operations"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(USDSCommandExecuteModule {}),
            Box::new(USDSCommandGetLatestAccountState {}),
            Box::new(USDSCommandETokenIssue {}),
            Box::new(USDSCommandETokenInit {}),
            Box::new(USDSCommandETokenMint {}),
            Box::new(USDSCommandETokenTransfer {}),
            Box::new(USDSCommandETokenSell {}),
            Box::new(USDSCommandETokenBuy {}),
            Box::new(USDSCommandETokenBurn {}),
            Box::new(USDSCommandWriteSet {}),
        ];

        subcommand_execute(&params[0], commands, client, &params[1..]);
    }
}

pub struct USDSCommandExecuteModule {}

impl Command for USDSCommandExecuteModule {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["execute", "exe"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id> <script_path> <script_arguments>"
    }
    fn get_description(&self) -> &'static str {
        "Execute a move script"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() < 3 {
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
        let script_args = params[3..params.len()].to_vec().iter().map(|str| str.to_string()).collect();
        execute_script_with_resolver(client, &address, source.as_str(),
                                     param_parse_arg_resolver(script_args)).map(handler_result).map_err(handler_err).ok();
    }
}

pub struct USDSCommandETokenIssue {}

impl Command for USDSCommandETokenIssue {
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
        let mut template: String = ETOKEN_ISSUE_TEMPLATE.clone();
        let mut etoken_addr = "0x".to_owned();
        etoken_addr.push_str(address.to_string().as_str());
        template = template.replace("${etoken}", etoken_addr.as_str());
        execute_script(client, &address, template.as_str(), vec![]).map(|(compiled_program, deps, seq)| {
            let verified_program = VerifiedProgram::new(compiled_program.clone(), &deps).unwrap();
            client.registry_module("etoken".to_string(), address.clone(), verified_program.modules().to_vec());
            (compiled_program, deps, seq)
        }).map(handler_result).map_err(handler_err).ok();
    }
}

// Init an account for accept etoken
pub struct USDSCommandETokenInit {}

impl Command for USDSCommandETokenInit {
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
        if !client.exist_module("etoken") {
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
pub struct USDSCommandETokenMint {}

impl Command for USDSCommandETokenMint {
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
        if !client.exist_module("etoken")  {
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
        let amount = match ClientProxy::convert_to_micro_libras(params[2]) {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_MINT_TEMPLATE, vec![TransactionArgument::U64(amount)]).map(handler_result).map_err(handler_err).ok();
    }
}

// Burn etoken from a order address
pub struct USDSCommandETokenBurn {}

impl Command for USDSCommandETokenBurn {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["etoken_burn", "burn"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <amount>"
    }
    fn get_description(&self) -> &'static str {
        "Burn etoken as a minter"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("etoken")  {
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
        let amount = match ClientProxy::convert_to_micro_libras(params[2]) {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        execute_script(client, &address, &ETOKEN_BURN_TEMPLATE, vec![TransactionArgument::U64(amount)]).map(handler_result).map_err(handler_err).ok();
    }
}


// Transfer etoken to an account
pub struct USDSCommandETokenTransfer {}

impl Command for USDSCommandETokenTransfer {
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
        if !client.exist_module("etoken")  {
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
        let amount = match ClientProxy::convert_to_micro_libras(params[3]) {
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
pub struct USDSCommandETokenSell {}

impl Command for USDSCommandETokenSell {
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
        if !client.exist_module("etoken")  {
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
        let amount = match ClientProxy::convert_to_micro_libras(params[2]) {
            Ok(i) => i,
            Err(e) => {
                report_error("invalid amount", e.into());
                return;
            }
        };
        let price = match ClientProxy::convert_to_micro_libras(params[3]) {
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
pub struct USDSCommandETokenBuy {}

impl Command for USDSCommandETokenBuy {
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
        if !client.exist_module("etoken")  {
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

pub fn handler_result(result: (CompiledProgram, Vec<VerifiedModule>, IndexAndSequence)) {
    let index_and_seq = result.2;
    println!("Finished transaction!");
    println!(
        "To query for transaction status, run: query txn_acc_seq {} {} \
                     <fetch_events=true|false>",
        index_and_seq.account_index, index_and_seq.sequence_number
    );
}

fn direct_arg_resolver(args: Vec<TransactionArgument>) -> Box<dyn FnOnce(&CompiledProgram) -> Result<Vec<TransactionArgument>>> {
    return Box::new(|_compiled_program: &CompiledProgram| -> Result<Vec<TransactionArgument>>{
        Ok(args)
    });
}

fn param_parse_arg_resolver(args: Vec<String>) -> Box<dyn FnOnce(&CompiledProgram) -> Result<Vec<TransactionArgument>>> {
    return Box::new(move |compiled_program: &CompiledProgram| -> Result<Vec<TransactionArgument>>{
        let script = compiled_program.script.borrow();
        let script_mut = script.clone().into_inner();
        let main_fun = script.main();
        let main_signature: &FunctionSignature = script_mut.function_signatures.get(main_fun.function.0 as usize).unwrap();
        if main_signature.arg_types.len() != args.len() {
            bail!("miss script arguments, expect:{:#?} ", main_signature.arg_types.clone());
        }
        let tx_args: Result<Vec<_>> = main_signature.arg_types.iter().enumerate().map(|(idx, arg_type)| -> Result<TransactionArgument>{
            match arg_type {
                SignatureToken::String => Ok(TransactionArgument::String(args[idx].clone())),
                SignatureToken::Address => Ok(TransactionArgument::Address(AccountAddress::try_from(args[idx].clone())?)),
                SignatureToken::U64 => Ok(TransactionArgument::U64(args[idx].parse()?)),
                SignatureToken::ByteArray => Ok(TransactionArgument::ByteArray(ByteArray::new(hex::decode(args[idx].clone())?))),
                _ => bail!("unsupported arg type:{:#?}", arg_type)
            }
        }).collect();
        Ok(tx_args?)
        //Ok(vec![])
    });
}

pub fn execute_script(client: &mut ClientProxy, address: &AccountAddress, script_template: &str, args: Vec<TransactionArgument>) -> Result<(CompiledProgram, Vec<VerifiedModule>, IndexAndSequence)> {
    return execute_script_with_resolver(client, address, script_template, direct_arg_resolver(args));
}

pub fn execute_script_with_resolver(client: &mut ClientProxy, address: &AccountAddress, script_template: &str, arg_resolver: Box<dyn FnOnce(&CompiledProgram) -> Result<Vec<TransactionArgument>>>) -> Result<(CompiledProgram, Vec<VerifiedModule>, IndexAndSequence)> {
    let (compiled_program, deps) = compile_script(script_template, client, &address)?;
    let is_blocking = true;
    let tx_args = arg_resolver(&compiled_program)?;
    println!("{:#?}", compiled_program);
    let program = create_transaction_program(&compiled_program, tx_args)?;
    let result = client.send_transaction(&address, program, None, None, is_blocking)?;
    return Ok((compiled_program, deps, result));
}

pub fn compile_script(script_template: &str, client: &mut ClientProxy, address: &AccountAddress) -> Result<(CompiledProgram, Vec<VerifiedModule>)> {
    let module_registry = client.get_module_registry();
    do_compile_script(address, script_template, &module_registry)
}

pub fn do_compile_script(address: &AccountAddress, script_template: &str, module_registry: &Vec<ModuleRegistryEntry>) -> Result<(CompiledProgram, Vec<VerifiedModule>)> {
    let source = parse_script(script_template, module_registry);
    //let mut extra_deps:Vec<VerifiedModule> = vec![];
    let extra_deps = module_registry.iter().map(|entry|entry.modules.as_slice()).collect_vec().as_slice().concat();
        //.fold(extra_deps, |c, module|{c.extend(module);return c;});
    let compiler = Compiler {
        address: address.clone(),
        code: &source,
        skip_stdlib_deps: false,
        stdlib_address: AccountAddress::default(),
        extra_deps,
        ..Compiler::default()
    };
    let (compiled_program, dependencies) = compiler
        .into_compiled_program_and_deps()?;
    Ok((compiled_program, dependencies))
}

pub fn parse_script(script_template: &str, module_registry: &Vec<ModuleRegistryEntry>) -> String {
    let mut script = script_template.to_string();
    for module in module_registry{
        let mut address_str = "0x".to_owned();
        address_str.push_str(module.account.to_string().as_str());
        script = script_template.replace(format!("${{{}}}",module.name).as_str(), address_str.as_str());
    }
    return script;
    //compiler::parser::parse_program(script.as_str())
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

    Ok(Program::new(script_blob, module_blobs, args))
}

/// Command to query latest account state from validator.
pub struct USDSCommandGetLatestAccountState {}

impl USDSCommandGetLatestAccountState {
    fn do_execute(&self, client: &mut ClientProxy, params: &[&str]) -> Result<()> {
        println!(">> Getting latest account state");
        match client.get_latest_account_state(&params) {
            Ok((acc, version)) => match acc {
                Some(blob) => {
                    let account_state = AccountState::from_blob(&blob, &client.get_module_registry())?;


                    println!(
                        "Latest account state is: \n \
                     Account: {:#?}\n \
                     AccountState: {:#?}\n \
                     Blockchain Version: {}\n",
                        client
                            .get_account_address_from_parameter(params[1])
                            .expect("Unable to parse account parameter"),
                        account_state,
                        version,
                    );

                    let account_btree:BTreeMap<Vec<u8>,Vec<u8>> = blob.borrow().try_into()?;
                    println!("AccountStateBlob Tree:");
                    account_btree.iter().map(|(k, v)| -> (String, String) {
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

impl Command for USDSCommandGetLatestAccountState {
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


pub struct USDSCommandWriteSet {}

impl USDSCommandWriteSet {
    fn do_execute(&self, client: &mut ClientProxy, params: &[&str]) -> Result<()> {
        let signer_account_address =
            client.get_account_address_from_parameter(params[1])?;
        let etoken_module = client.module_registry.get("etoken").unwrap();
        let path = ETokenResource::resource_path(etoken_module.account.clone());
        let ap = AccessPath::new(signer_account_address.clone(), path);
        let resource = ETokenResource::new(9999);
        let resource_bytes = SimpleSerializer::serialize(&resource).unwrap();
        let mut write_set = WriteSetMut::default();
        write_set.push((ap, WriteOp::Value(resource_bytes)));
        let ws = write_set.freeze()?;
        let sequence = client.get_account_resource_and_update(signer_account_address.clone()).unwrap().sequence_number();
        let tx = RawTransaction::new_write_set(signer_account_address, sequence, ws);
        client.submit_custom_transaction(signer_account_address, tx, true)?;
        Ok(())
    }
}

impl Command for USDSCommandWriteSet {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["write_set", "ws"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Directly save resource to account, this command will fail because of RejectedWriteSet error"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if !client.exist_module("etoken")  {
            println!("Please issue etoken first.");
            return;
        }
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

    use crate::usds_commands::*;
    use crate::client_proxy::*;
    use vm::file_format::CompiledProgram;
    use bytecode_verifier::VerifiedModule;
    use failure::prelude::*;
    use bytecode_verifier::verifier::VerifiedProgram;

    #[test]
    fn test_parse_script() {
        //println!("{:?}", AccountAddress::random());
        //println!("{:?}",AccountAddress::default().to_string());

        let entry = ModuleRegistryEntry{name:"etoken".to_string(), account: AccountAddress::random(), modules:vec![]};
        let module_registry = vec![entry];

        let program = parse_script(&ETOKEN_ISSUE_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_INIT_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_MINT_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_TRANSFER_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_SELL_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_BUY_TEMPLATE, &module_registry);
        println!("{:?}", program);
        let program = parse_script(&ETOKEN_BURN_TEMPLATE, &module_registry);
        println!("{:?}", program);
    }

    fn compile_etoken() -> Result<Vec<ModuleRegistryEntry>>{
        let etoken_address = AccountAddress::random();

        let mut entry = ModuleRegistryEntry{name:"etoken".to_string(), account: etoken_address.clone(), modules:vec![]};

        let (compiled_program,deps) = do_compile_script(&etoken_address, &ETOKEN_ISSUE_TEMPLATE, &vec![entry.clone()])?;
        let verified_program = VerifiedProgram::new(compiled_program, &deps).unwrap();
        entry.modules.append(verified_program.modules().to_vec().as_mut());

        Ok(vec![entry])
    }

    #[test]
    fn test_etoken_script(){
        do_test_compile_scripts(vec![ETOKEN_INIT_TEMPLATE.to_string(), ETOKEN_MINT_TEMPLATE.to_string(), ETOKEN_TRANSFER_TEMPLATE.to_string(), ETOKEN_SELL_TEMPLATE.to_string(), ETOKEN_BUY_TEMPLATE.to_string(), ETOKEN_BURN_TEMPLATE.to_string()]).expect("test fail.");
    }

    fn do_test_compile_scripts(scripts:Vec<String>)->Result<()>{
        let address = AccountAddress::random();
        let module_registry = compile_etoken().expect("compile etoken fail.");
        for script in scripts{
            match do_compile_script(&address, &script, &module_registry){
                Ok((program,_)) => {
                    println!("{:#?}",program)
                },
                Err(e) => panic!("script:{} err:{:?}",script, e)
            }
        }
        Ok(())
    }

    #[test]
    fn test_other_script(){
        let address = AccountAddress::random();
        let module_registry = compile_etoken().expect("compile etoken fail.");

        let scripts = vec![include_str!("../move/fake_token.mvir")];
        for script in scripts {
            match do_compile_script(&address, script, &module_registry){
                Ok(_) => {},
                Err(e) => panic!("script:{} err:{:?}",script, e)
            }
        }
    }

    #[test]
    fn test_slice() {
        let a = ["0", "1", "2"];
        println!("{}", &a[3..a.len()].len());
    }
}