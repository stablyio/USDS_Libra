use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use canonical_serialization::{CanonicalDeserialize, CanonicalDeserializer, CanonicalSerialize, CanonicalSerializer, SimpleDeserializer};
use failure::prelude::*;
use types::access_path::{Accesses, AccessPath};
use types::account_address::AccountAddress;
use types::byte_array::ByteArray;
use types::language_storage::StructTag;

fn resource_path(module_address: AccountAddress, module_name: &str, struct_name: &str) -> Vec<u8> {
    AccessPath::resource_access_vec(
        &StructTag {
            address: module_address,
            module: module_name.to_string(),
            name: struct_name.to_string(),
            type_params: vec![],
        },
        &Accesses::empty(),
    )
}

const DEFAULT_STRUCT_NAME: &'static str = "T";

#[derive(Debug, Clone, IntoStaticStr)]
pub enum Resource {
    EToken(Option<ETokenResource>),
    Channel(Option<ChannelResource>),
    ClosedChannel(Option<ClosedChannelResource>),
    Proof(Option<ProofResource>),
}


pub const ETOKEN_MODULE_NAME: &str = "EToken";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ETokenResource {
    pub value: u64,
}

impl ETokenResource {
    pub fn new(amount: u64) -> Self {
        ETokenResource {
            value: amount,
        }
    }

    pub fn resource_path(module_address: AccountAddress) -> Vec<u8> {
        resource_path(module_address, ETOKEN_MODULE_NAME, DEFAULT_STRUCT_NAME)
    }

    pub fn make_from(module_address: AccountAddress, account_map: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        let ap = resource_path(module_address, ETOKEN_MODULE_NAME, DEFAULT_STRUCT_NAME);
        match account_map.get(&ap) {
            Some(bytes) => SimpleDeserializer::deserialize(bytes),
            None => bail!("No data for {:?}", ap),
        }
    }
}

impl CanonicalSerialize for ETokenResource {
    fn serialize(&self, serializer: &mut impl CanonicalSerializer) -> Result<()> {
        serializer
            .encode_u64(self.value)?;
        Ok(())
    }
}

impl CanonicalDeserialize for ETokenResource {
    fn deserialize(deserializer: &mut impl CanonicalDeserializer) -> Result<Self> {
        let value = deserializer.decode_u64()?;

        Ok(ETokenResource {
            value,
        })
    }
}

pub const CHANNEL_MODULE_NAME: &str = "Channel";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelResource {
    pub other: AccountAddress,
    pub coin: u64,
}

impl ChannelResource {
    pub fn make_from(module_address: AccountAddress, account_map: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        let ap = resource_path(module_address, CHANNEL_MODULE_NAME, DEFAULT_STRUCT_NAME);
        match account_map.get(&ap) {
            Some(bytes) => SimpleDeserializer::deserialize(bytes),
            None => bail!("No data for {:?}", ap),
        }
    }
}

impl CanonicalSerialize for ChannelResource {
    fn serialize(&self, serializer: &mut impl CanonicalSerializer) -> Result<()> {
        serializer
            .encode_u64(self.coin)?;
        serializer.encode_struct(&self.other)?;
        Ok(())
    }
}

impl CanonicalDeserialize for ChannelResource {
    fn deserialize(deserializer: &mut impl CanonicalDeserializer) -> Result<Self> {
        // fields order is filed name Lexicographical order
        let coin = deserializer.decode_u64()?;
        let other: AccountAddress = deserializer.decode_struct()?;

        Ok(ChannelResource {
            other,
            coin,
        })
    }
}


pub const CLOSED_STRUCT_NAME: &str = "ClosedChannel";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClosedChannelResource {
    pub other: AccountAddress,
    pub coin: u64,
    pub height: u64,
}

impl ClosedChannelResource {
    pub fn make_from(module_address: AccountAddress, account_map: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        let ap = resource_path(module_address, CHANNEL_MODULE_NAME, CLOSED_STRUCT_NAME);
        match account_map.get(&ap) {
            Some(bytes) => SimpleDeserializer::deserialize(bytes),
            None => bail!("No data for {:?}", ap),
        }
    }
}

impl CanonicalSerialize for ClosedChannelResource {
    fn serialize(&self, serializer: &mut impl CanonicalSerializer) -> Result<()> {
        serializer
            .encode_u64(self.coin)?;
        serializer
            .encode_u64(self.height)?;
        serializer.encode_struct(&self.other)?;
        Ok(())
    }
}

impl CanonicalDeserialize for ClosedChannelResource {
    fn deserialize(deserializer: &mut impl CanonicalDeserializer) -> Result<Self> {
        // fields order is filed name Lexicographical order
        let coin = deserializer.decode_u64()?;
        let height = deserializer.decode_u64()?;
        let other: AccountAddress = deserializer.decode_struct()?;

        Ok(ClosedChannelResource {
            other,
            coin,
            height,
        })
    }
}


pub const PROOF_STRUCT_NAME: &str = "Proof";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProofResource {
    pub version: u64,
    pub self_balance: u64,
    pub other_balance: u64,
    pub self_signature: ByteArray,
    pub other_signature: ByteArray,
}

impl ProofResource {
    pub fn make_from(module_address: AccountAddress, account_map: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        let ap = resource_path(module_address, CHANNEL_MODULE_NAME, PROOF_STRUCT_NAME);
        match account_map.get(&ap) {
            Some(bytes) => SimpleDeserializer::deserialize(bytes),
            None => bail!("No data for {:?}", ap),
        }
    }
}

impl CanonicalSerialize for ProofResource {
    fn serialize(&self, serializer: &mut impl CanonicalSerializer) -> Result<()> {
        serializer.encode_u64(self.other_balance)?;
        serializer.encode_struct(&self.other_signature)?;
        serializer.encode_u64(self.self_balance)?;
        serializer.encode_struct(&self.self_signature)?;
        serializer
            .encode_u64(self.version)?;
        Ok(())
    }
}

impl CanonicalDeserialize for ProofResource {
    fn deserialize(deserializer: &mut impl CanonicalDeserializer) -> Result<Self> {
        // fields order is filed name Lexicographical order
        let other_balance = deserializer.decode_u64()?;
        let other_signature: ByteArray = deserializer.decode_struct()?;
        let self_balance = deserializer.decode_u64()?;
        let self_signature: ByteArray = deserializer.decode_struct()?;

        let version = deserializer.decode_u64()?;

        Ok(ProofResource {
            version,
            self_balance,
            self_signature,
            other_balance,
            other_signature,
        })
    }
}


#[cfg(test)]
mod tests {
    use hex::FromHex;

    use canonical_serialization::SimpleDeserializer;

    use crate::resource::*;

    #[test]
    fn test_channel_deserialize() {
        let bytes: Vec<u8> = Vec::from_hex("0065cd1d000000002000000099ed3e6632ada884225d19d9ba6c5427b1d40638455658dc00923d809a21b7dd").unwrap();
        let channel: ChannelResource = SimpleDeserializer::deserialize(bytes.as_slice()).unwrap();
        println!("channel:{:?}", channel);
    }
}