use std::collections::BTreeMap;

use serde::export::fmt::{Display, Debug};

use canonical_serialization::{CanonicalDeserialize, CanonicalDeserializer, CanonicalSerialize, CanonicalSerializer, SimpleDeserializer};
use failure::prelude::*;
use types::access_path::{Accesses, AccessPath};
use types::account_address::AccountAddress;
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
pub enum Resource{
    EToken(Option<ETokenResource>),
    Channel(Option<ChannelResource>)
}


pub const ETOKEN_MODULE_NAME: &str = "EToken";

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
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
        serializer.encode_struct(&self.other)?;
        serializer
            .encode_u64(self.coin)?;
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

#[cfg(test)]
mod tests{
    use crate::resource::*;
    use canonical_serialization::SimpleDeserializer;
    use hex::FromHex;

    #[test]
    fn test_channel_deserialize(){
        let bytes:Vec<u8> = Vec::from_hex("0065cd1d000000002000000099ed3e6632ada884225d19d9ba6c5427b1d40638455658dc00923d809a21b7dd").unwrap();
        let channel:ChannelResource = SimpleDeserializer::deserialize(bytes.as_slice()).unwrap();
        println!("channel:{:?}", channel);
    }
}