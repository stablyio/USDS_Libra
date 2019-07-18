use std::collections::{BTreeMap, HashMap};
use std::convert::{TryFrom, TryInto};

use failure::prelude::*;
use types::account_config::AccountResource;
use types::account_state_blob::AccountStateBlob;

use crate::client_proxy::ModuleRegistryEntry;
use crate::resource::{ChannelResource, ETokenResource, Resource};

#[derive(Debug)]
pub struct AccountState {
    account_resource: AccountResource,
    resources: HashMap<String, Vec<Resource>>,
}

impl AccountState {
    pub fn from_blob(blob: &AccountStateBlob, module_registry: &Vec<ModuleRegistryEntry>) -> Result<Self> {
        let mut resources = HashMap::new();
        let map: BTreeMap<Vec<u8>, Vec<u8>> = blob.try_into()?;
        let account_resource = AccountResource::make_from(&map).unwrap_or(AccountResource::default());
        for module in module_registry {
            resources.insert(module.name.clone(), module.get_resource(&map));
        }
        Ok(AccountState {
            account_resource,
            resources,
        })
    }
}

impl TryFrom<&BTreeMap<Vec<u8>, Vec<u8>>> for AccountState {
    type Error = Error;

    fn try_from(value: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<Self> {
        unimplemented!()
    }
}

impl TryFrom<&AccountStateBlob> for AccountState {
    type Error = Error;

    fn try_from(value: &AccountStateBlob) -> Result<Self> {
        let map: BTreeMap<Vec<u8>, Vec<u8>> = value.try_into()?;
        Self::try_from(&map)
    }
}