use std::{collections::BTreeMap};
use canonical_serialization::{CanonicalSerialize, SimpleDeserializer, CanonicalSerializer, CanonicalDeserializer, CanonicalDeserialize};
use failure::prelude::*;
use types::access_path::{Accesses, AccessPath};
use types::account_address::AccountAddress;
use types::language_storage::StructTag;

pub const ETOKEN_MODULE_NAME: &str = "EToken";
pub const ETOKEN_STRUCT_NAME: &str = "T";

#[derive(Debug, Default)]
pub struct ETokenResource {
    pub value: u64,
}

impl ETokenResource {

    pub fn new(amount:u64) -> Self{
        ETokenResource{
            value:amount,
        }
    }

    pub fn make_from(etoken_issue_address: AccountAddress, account_map: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        let ap = Self::etoken_resource_path(etoken_issue_address);
        match account_map.get(&ap) {
            Some(bytes) => SimpleDeserializer::deserialize(bytes),
            None => bail!("No data for {:?}", ap),
        }
    }

    pub fn etoken_resource_path(etoken_issue_address: AccountAddress) -> Vec<u8> {
        AccessPath::resource_access_vec(
            &StructTag {
                address: etoken_issue_address,
                module: ETOKEN_MODULE_NAME.to_string(),
                name: ETOKEN_STRUCT_NAME.to_string(),
                type_params: vec![],
            },
            &Accesses::empty(),
        )
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