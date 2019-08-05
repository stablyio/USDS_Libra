// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![feature(duration_float)]
//#![deny(missing_docs)]
//! Libra Client
//!
//! Client (binary) is the CLI tool to interact with Libra validator.
//! It supposes all public APIs.
use crypto::signing::KeyPair;
use failure::prelude::*;
use types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

use crate::resource::*;
use std::collections::HashMap;

pub(crate) mod account_commands;
/// Main instance of client holding corresponding information, e.g. account address.
pub mod client_proxy;
/// Command struct to interact with client.
pub mod commands;
/// gRPC client wrapper to connect to validator.
pub(crate) mod grpc_client;
pub(crate) mod query_commands;
pub(crate) mod submit_transaction_command;
pub(crate) mod transfer_commands;
pub(crate) mod resource;
pub(crate) mod account_state;
pub(crate) mod usds_commands;
pub(crate) mod channel_commands;

/// Offchain transfer request
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransferRequest {
    /// sender.
    pub sender: AccountAddress,
    /// version
    pub version: u64,
    /// amount
    pub amount: u64,
    /// self balance
    pub self_balance: u64,
    /// other balance
    pub other_balance: u64,
    /// sender signature
    pub signature: Vec<u8>,
}

impl TransferRequest {
    pub fn total_balance(&self) -> u64 {
        self.self_balance + self.other_balance
    }
}

/// Offchain transfer conform
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransferConform {
    pub sender: AccountAddress,
    /// sender signature
    pub signature: Vec<u8>,
    pub request: TransferRequest,
}

/// Offchain data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChannelLocalData {
    /// channel data version
    pub version: u64,
    /// my balance
    pub self_balance: u64,
    /// other balance
    pub other_balance: u64,
    /// self signature
    pub self_signature: Vec<u8>,
    /// other party signature
    pub other_signature: Vec<u8>,
}

impl ChannelLocalData {
    pub fn total_balance(&self) -> u64 {
        self.self_balance + self.other_balance
    }
}

/// Channel Status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChannelStatus {
    None(),
    Open(ChannelResource),
    Closed(ChannelResource, Option<ProofResource>),
}

impl ChannelStatus {
    pub fn is_open(&self) -> bool {
        match self {
            Self::Open(_) => true,
            _ => false,
        }
    }
}

/// Offchain channel
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OffchainChannel {
    pub self_address: AccountAddress,
    /// channel other party account
    pub other_address: AccountAddress,
    pub self_status: ChannelStatus,
    pub other_status: ChannelStatus,
    pub data: Option<ChannelLocalData>,
}

impl OffchainChannel {
    pub fn new(self_address: AccountAddress, other_address: AccountAddress, self_channel: ChannelResource, other_channel: Option<ChannelResource>, self_proof: Option<ProofResource>, other_proof: Option<ProofResource>) -> Self {
        let data = match &self_proof {
            Some(proof) => {
                Some(ChannelLocalData {
                    version: proof.version,
                    self_balance: proof.self_balance,
                    self_signature: proof.self_signature.as_bytes().to_vec(),
                    other_balance: proof.other_balance,
                    other_signature: proof.other_signature.as_bytes().to_vec(),
                })
            }
            None => None,
        };

        OffchainChannel {
            self_address,
            other_address,
            self_status:
            if self_channel.closed {
                ChannelStatus::Closed(self_channel, self_proof)
            } else {
                ChannelStatus::Open(self_channel)
            },
            other_status: match other_channel {
                Some(other_channel) => if other_channel.closed {
                    ChannelStatus::Closed(other_channel, other_proof)
                } else {
                    ChannelStatus::Open(other_channel)
                },
                None => ChannelStatus::None()
            },
            data,
        }
    }

    pub fn is_ready(&self) -> bool {
        return self.self_status.is_open() && self.other_status.is_open();
    }

    pub fn transfer(&self, amount: u64) -> Result<TransferRequest> {
        ensure!(self.is_ready(), "channel is not ready");
        if let Some(data) = &self.data {
            ensure!(data.self_balance >= amount, "balance not enough.");
            return Ok(TransferRequest {
                sender: self.self_address,
                amount,
                version: data.version + 1,
                self_balance: data.self_balance - amount,
                other_balance: data.other_balance - amount,
                signature: vec![],
            });
        }
        if let ChannelStatus::Open(resource) = &self.self_status {
            if let ChannelStatus::Open(other_resource) = &self.other_status {
                ensure!(resource.coin >= amount, "balance not enough.");
                return Ok(TransferRequest {
                    sender: self.self_address,
                    amount,
                    version: 1,
                    self_balance: resource.coin - amount,
                    other_balance: other_resource.coin + amount,
                    //TODO
                    signature: vec![],
                });
            }
        }
        bail!("unexpect channel status.")
    }

    pub fn conform(&mut self, request: TransferRequest) -> Result<TransferConform> {
        let signature = vec![];
        ensure!(self.is_ready(), "channel is not ready");
        if let Some(data) = self.data.as_mut() {
            ensure!(data.version + 1 == request.version, "check version fail");
            ensure!(data.self_balance + request.amount == request.other_balance, "balance check fail.");
            ensure!(data.total_balance() == request.total_balance(), "balance check fail.");
            //TODO check signature
            data.version = request.version;
            data.self_balance = request.other_balance;
            data.other_balance = request.self_balance;
            data.other_signature = request.signature.clone();
            data.self_signature = signature.clone();
        } else {
            ensure!(request.version == 1, "check version fail");

            if let ChannelStatus::Open(resource) = &self.self_status {
                if let ChannelStatus::Open(other_resource) = &self.other_status {
                    ensure!(other_resource.coin >= request.amount, "balance not enough.");
                    ensure!(resource.coin + other_resource.coin == request.total_balance(), "balance check fail.");
                    ensure!(request.other_balance == resource.coin + request.amount, "balance check fail.");
                } else {
                    bail!("unexpect channel status.")
                }
            } else {
                bail!("unexpect channel status.")
            }

            let data = ChannelLocalData {
                version: request.version,
                self_balance: request.other_balance,
                other_balance: request.self_balance,
                other_signature: request.signature.clone(),
                self_signature: signature.clone(),
            };
            self.data = Some(data);
        }
        Ok(
            TransferConform {
                sender: self.self_address.clone(),
                //TODO
                signature,
                request,
            }
        )
    }

    pub fn process_transfer_conform(&mut self, conform: TransferConform) -> Result<()> {
        //TODO check request
        ensure!(self.is_ready(), "channel is not ready");
        if let Some(data) = self.data.as_mut() {
            data.version = conform.request.version;
            data.self_balance = conform.request.self_balance;
            data.other_balance = conform.request.other_balance;
            data.self_signature = conform.request.signature.clone();
            data.other_signature = conform.signature.clone();
        } else {
            let data = ChannelLocalData {
                version: conform.request.version,
                self_balance: conform.request.self_balance,
                other_balance: conform.request.other_balance,
                other_signature: conform.signature.clone(),
                self_signature: conform.request.signature.clone(),
            };
            self.data = Some(data);
        }
        Ok(())
    }

    pub fn update_with_resource(&mut self, channel_resource: ChannelResource, proof_resource: Option<ProofResource>) {
        if channel_resource.other == self.other_address {
            if channel_resource.closed {
                self.self_status = ChannelStatus::Closed(channel_resource, proof_resource)
            } else {
                self.self_status = ChannelStatus::Open(channel_resource)
            }
        } else if channel_resource.other == self.self_address {
            if channel_resource.closed {
                self.other_status = ChannelStatus::Closed(channel_resource, proof_resource)
            } else {
                self.other_status = ChannelStatus::Open(channel_resource)
            }
        }
    }
}

/// Struct used to store data for each created account.  We track the sequence number
/// so we can create new transactions easily
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountData {
    /// Address of the account.
    pub address: AccountAddress,
    /// (private_key, public_key) pair if the account is not managed by wallet.
    pub key_pair: Option<KeyPair>,
    /// Latest sequence number maintained by client, it can be different from validator.
    pub sequence_number: u64,
    /// Whether the account is initialized on chain, cached local only, or status unknown.
    pub status: AccountStatus,
    /// Offchain channels.
    pub channels: HashMap<AccountAddress, OffchainChannel>,
    /// Offchain transfer request
    pub transfer_requests: Vec<TransferRequest>,
    /// Offchain transfer conform
    pub transfer_conforms: Vec<TransferConform>,
}

/// Enum used to represent account status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    /// Account exists only in loacal cache, it is not persisted on chain.
    Local,
    /// Account is persisted on chain.
    Persisted,
    /// Not able to check account status, probably because client is not able to talk to the
    /// validator.
    Unknown,
}

impl AccountData {
    pub fn new(address: AccountAddress,
               key_pair: Option<KeyPair>,
               sequence_number: u64,
               status: AccountStatus) -> Self {
        AccountData {
            address,
            key_pair,
            sequence_number,
            status,
            channels: HashMap::new(),
            transfer_requests: vec![],
            transfer_conforms: vec![],
        }
    }

    /// Serialize account keypair if exists.
    pub fn keypair_as_string(&self) -> Option<(String, String)> {
        match &self.key_pair {
            Some(key_pair) => Some((
                crypto::utils::encode_to_string(&key_pair.private_key()),
                crypto::utils::encode_to_string(&key_pair.public_key()),
            )),
            None => None,
        }
    }

    /// append channel
    pub fn append_channel(&mut self, channel: OffchainChannel) {
        self.channels.insert(channel.other_address.clone(), channel);
    }

    pub fn delete_channel(&mut self, other: &AccountAddress) {
        self.channels.remove(other);
    }

    /// get channel
    pub fn get_channel(&mut self, other: &AccountAddress) -> Option<&mut OffchainChannel> {
        return self.channels.get_mut(other);
    }

    /// append_transfer_request
    pub fn append_transfer_request(&mut self, request: TransferRequest) {
        self.transfer_requests.push(request);
    }

    /// append_transfer_conform
    pub fn append_transfer_conform(&mut self, conform: TransferConform) {
        self.transfer_conforms.push(conform);
    }
}
