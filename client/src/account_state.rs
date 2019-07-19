use std::collections::{BTreeMap, HashMap};
use std::convert::{TryFrom, TryInto};

use failure::prelude::*;
use types::account_config::AccountResource;
use types::account_state_blob::AccountStateBlob;

use crate::client_proxy::ModuleRegistryEntry;
use crate::resource::{ChannelResource, ETokenResource, Resource};
use itertools::Itertools;

#[derive(Debug)]
pub struct AccountState {
    pub account_resource: AccountResource,
    pub resources: HashMap<String, Vec<Resource>>,
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

    pub fn find_resource(&self,filter:impl FnMut(&&Resource)->bool) -> Option<Resource>{
        self.resources.iter().map(|(_k,v)|v.as_slice()).collect_vec().as_slice().concat().iter().find(filter).cloned()
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