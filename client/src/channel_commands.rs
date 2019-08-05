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

use crate::{client_proxy::*, commands::*, resource::*, usds_commands::*, TransferRequest, TransferConform};

lazy_static! {

    pub static ref CHANNEL_TEMPLATE: String = {include_str!("../move/channel.mvir").to_string()};
    pub static ref CHANNEL_OPEN_TEMPLATE: String = {include_str!("../move/channel_open.mvir").to_string()};
    pub static ref CHANNEL_CLOSE_TEMPLATE: String = {include_str!("../move/channel_close.mvir").to_string()};
    pub static ref CHANNEL_CLOSE_WITH_PROOF_TEMPLATE: String = {include_str!("../move/channel_close_with_proof.mvir").to_string()};
    pub static ref CHANNEL_SETTLE_TEMPLATE: String = {include_str!("../move/channel_settle.mvir").to_string()};
}


/// Major command for channel operations.
pub struct ChannelCommand {}

impl Command for ChannelCommand {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["channel", "ch"]
    }
    fn get_description(&self) -> &'static str {
        "Channel operations"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(ChannelCommandDeploy {}),
            Box::new(ChannelCommandOpen {}),
            Box::new(ChannelCommandClose {}),
            Box::new(ChannelCommandShow {}),
            Box::new(ChannelCommandSettle{}),
            Box::new(ChannelCommandOffchainTransfer {}),
            Box::new(ChannelCommandOffchainConform {}),
            Box::new(ChannelCommandOffchainProcessConform {}),
        ];

        subcommand_execute(&params[0], commands, client, &params[1..]);
    }
}

pub struct ChannelCommandDeploy {}

impl Command for ChannelCommandDeploy {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["deploy", "d"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>"
    }
    fn get_description(&self) -> &'static str {
        "Deploy channel Module to an account"
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

        execute_script(client, &address, &CHANNEL_TEMPLATE, vec![]).map(|(compiled_program, deps, seq)| {
            let verified_program = VerifiedProgram::new(compiled_program.clone(), &deps).unwrap();
            client.registry_module("channel".to_string(), address.clone(), verified_program.modules().to_vec());
            (compiled_program, deps, seq)
        }).map(handler_result).map_err(handler_err).ok();
    }
}


/// Open channel
pub struct ChannelCommandOpen {}

impl Command for ChannelCommandOpen {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["open", "o"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <account_ref_id>|<account_address> <amount>"
    }
    fn get_description(&self) -> &'static str {
        "Open channel with an account"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 4 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let other_address = match client.get_account_address_from_parameter(params[2]) {
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
        execute_script(client, &address, &CHANNEL_OPEN_TEMPLATE, vec![TransactionArgument::Address(other_address.clone()), TransactionArgument::U64(amount)]).map(handler_result).map_err(handler_err).ok();
        client.sync_channel_status(address, other_address);
    }
}


/// Close channel
pub struct ChannelCommandClose {}

impl Command for ChannelCommandClose {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["close", "c"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <account_ref_id>|<account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Close a channel."
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let other_address = match client.get_account_address_from_parameter(params[2]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let account_data = match client.get_account_data(address) {
            Some(account_data) => account_data,
            None => {
                println!("get account data fail.");
                return;
            }
        };
        let channel = match account_data.get_channel(&other_address){
            Some(channel) => channel,
            None => {
                println!("get channel with address {} fail.", other_address);
                return;
            }
        };
        match  &channel.data{
            Some(offchain_data) => {
                let args = vec![TransactionArgument::Address(other_address), TransactionArgument::U64(offchain_data.version),
                                TransactionArgument::U64(offchain_data.self_balance), TransactionArgument::U64(offchain_data.other_balance),
                                TransactionArgument::ByteArray(ByteArray::new(offchain_data.self_signature.clone())), TransactionArgument::ByteArray(ByteArray::new(offchain_data.other_signature.clone()))
                ];
                execute_script(client, &address, &CHANNEL_CLOSE_WITH_PROOF_TEMPLATE, args).map(handler_result).map_err(handler_err).ok();
            }
            None => {
                execute_script(client, &address, &CHANNEL_CLOSE_TEMPLATE, vec![TransactionArgument::Address(other_address.clone())]).map(handler_result).map_err(handler_err).ok();
            }
        };
        client.sync_channel_status(address, other_address);
    }
}



/// Close channel
pub struct ChannelCommandShow {}

impl Command for ChannelCommandShow {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["show", "so"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Show channels."
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 2 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };

        let account_data = match client.get_account_data(address) {
            Some(account_data) => account_data,
            None => {
                println!("get account data fail.");
                return;
            }
        };

        for channel in &account_data.channels{
            println!("channel:{:#?}", channel);
        }
    }
}



/// Settle channel
pub struct ChannelCommandSettle {}

impl Command for ChannelCommandSettle {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["settle", "s"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <account_ref_id>|<account_address>"
    }
    fn get_description(&self) -> &'static str {
        "Settle an channel"
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let other_address = match client.get_account_address_from_parameter(params[2]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        execute_script(client, &address, &CHANNEL_SETTLE_TEMPLATE, vec![TransactionArgument::Address(other_address.clone())]).map(handler_result).map_err(handler_err).ok();
        client.sync_channel_status(address, other_address);
    }
}


/// Offchain transfer
pub struct ChannelCommandOffchainTransfer {}

impl Command for ChannelCommandOffchainTransfer {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["transfer", "t"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <account_ref_id>|<account_address> <amount>"
    }
    fn get_description(&self) -> &'static str {
        "Transfer offchain LibraCoin to other."
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 4 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let other_address = match client.get_account_address_from_parameter(params[2]) {
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

        match client.sync_channel_status(address, other_address){
            Err(e) => {
                report_error("sync_channel_status error", e.into());
                return;
            }
            Ok(_) =>{
                //ignore
            }
        };

        let account_data = match client.get_account_data(address) {
            Some(account_data) => account_data,
            None => {
                print!("get account data fail.");
                return;
            }
        };
        let channel = match account_data.get_channel(&other_address) {
            Some(channel) => channel,
            None => {
                println!("get channel data fail.");
                return;
            }
        };
        let request = match channel.transfer(amount){
            Ok(request) => request,
            Err(e) => {
                report_error("transfer fail: {:?}", e.into());
                return;
            }
        };

        match bincode::serialize(&request){
            Ok(bytes) => {
                let hex = hex::encode(bytes);
                println!("please send transfer request to other:");
                println!("{}",hex);
            }
            Err(e) => {
                report_error("transfer fail: {:?}", e.into());
                return;
            }
        }
        return;
    }
}

/// Offchain transfer conform
pub struct ChannelCommandOffchainConform {}

impl Command for ChannelCommandOffchainConform {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["conform", "cf"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <request_hex>"
    }
    fn get_description(&self) -> &'static str {
        "Conform offchain transfer request from other."
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let hex = params[2];
        let bytes = match hex::decode(hex){
            Ok(bytes) => bytes,
            Err(e) => {
                report_error("parse hex error.", e.into());
                return;
            }
        };

        let request = match bincode::deserialize::<TransferRequest>(bytes.as_slice()){
            Ok(request) => request,
            Err(e) => {
                report_error("parse request error.", e.into());
                return;
            }
        };
        let other_address = request.sender.clone();
        match client.sync_channel_status(address, other_address){
            Err(e) => {
                report_error("sync_channel_status error", e.into());
                return;
            }
            Ok(_) =>{
                //ignore
            }
        };

        let account_data = match client.get_account_data(address) {
            Some(account_data) => account_data,
            None => {
                print!("get account data fail.");
                return;
            }
        };
        let mut channel = match account_data.get_channel(&other_address) {
            Some(channel) => channel,
            None => {
                println!("get channel offchain data fail.");
                return;
            }
        };
        let conform = match channel.conform(request) {
            Ok(conform) => conform,
            Err(e) => {
                report_error("conform error", e.into());
                return;
            }
        };

        match bincode::serialize(&conform){
            Ok(bytes) => {
                let hex = hex::encode(bytes);
                println!("please send transfer conform to other:");
                println!("{}",hex);
            }
            Err(e) => {
                report_error("transfer conform fail: {:?}", e.into());
                return;
            }
        }
        return;
    }
}


/// Offchain transfer conform
pub struct ChannelCommandOffchainProcessConform {}

impl Command for ChannelCommandOffchainProcessConform {
    fn get_aliases(&self) -> Vec<&'static str> {
        vec!["process_conform", "pcf"]
    }
    fn get_params_help(&self) -> &'static str {
        "<account_ref_id>|<account_address> <conform_hex>"
    }
    fn get_description(&self) -> &'static str {
        "Process transfer conform."
    }
    fn execute(&self, client: &mut ClientProxy, params: &[&str]) {
        if params.len() != 3 {
            println!("Invalid number of arguments for command");
            return;
        }
        if !client.exist_module("channel") {
            println!("Please deploy channel first.");
            return;
        }
        let address = match client.get_account_address_from_parameter(params[1]) {
            Ok(address) => address,
            Err(e) => {
                report_error("get address fail.", e);
                return;
            }
        };
        let hex = params[2];
        let bytes = match hex::decode(hex){
            Ok(bytes) => bytes,
            Err(e) => {
                report_error("parse hex error.", e.into());
                return;
            }
        };

        let conform = match bincode::deserialize::<TransferConform>(bytes.as_slice()){
            Ok(conform) => conform,
            Err(e) => {
                report_error("parse conform error.", e.into());
                return;
            }
        };

        let other_address = conform.sender;

        match client.sync_channel_status(address, other_address){
            Err(e) => {
                report_error("sync_channel_status error", e.into());
                return;
            }
            Ok(_) =>{
                //ignore
            }
        };

        let account_data = match client.get_account_data(address) {
            Some(account_data) => account_data,
            None => {
                print!("get account data fail.");
                return;
            }
        };
        let mut channel = match account_data.get_channel(&other_address) {
            Some(channel) => channel,
            None => {
                println!("get channel offchain data fail.");
                return;
            }
        };
        match channel.process_transfer_conform(conform){
            Ok(()) => {
                println!("channel: {:#?}", channel);
            },
            Err(e) => {
                report_error("process conform error", e.into());
                return;
            }
        }
        return;
    }
}