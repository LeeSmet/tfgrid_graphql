use std::fmt;

use crate::compat::{de_i64, de_u64};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeContract {
    #[serde(rename = "contractID", deserialize_with = "de_u64")]
    pub contract_id: u64,
    // Timestamp the object was created, in milliseconds.
    #[serde(deserialize_with = "de_i64")]
    pub created_at: i64,
    pub deployment_data: String,
    pub deployment_hash: String,
    #[serde(rename = "nodeID")]
    pub node_id: u32,
    #[serde(rename = "numberOfPublicIPs")]
    pub number_of_public_ips: u32,
    pub resources_used: Option<Resources>,
    #[serde(rename = "solutionProviderID")]
    pub solution_provider_id: Option<u32>,
    pub state: ContractState,
    #[serde(rename = "twinID")]
    pub twin_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameContract {
    #[serde(rename = "contractID", deserialize_with = "de_u64")]
    pub contract_id: u64,
    // Timestamp the object was created, in milliseconds.
    #[serde(deserialize_with = "de_i64")]
    pub created_at: i64,
    #[serde(rename = "solutionProviderID")]
    pub solution_provider_id: Option<u32>,
    pub state: ContractState,
    #[serde(rename = "twinID")]
    pub twin_id: u32,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Resources {
    #[serde(deserialize_with = "de_u64")]
    pub cru: u64,
    #[serde(deserialize_with = "de_u64")]
    pub hru: u64,
    #[serde(deserialize_with = "de_u64")]
    pub mru: u64,
    #[serde(deserialize_with = "de_u64")]
    pub sru: u64,
}

#[derive(Serialize, Deserialize)]
pub enum ContractState {
    Created,
    GracePeriod,
    OutOfFunds,
    Deleted,
}

impl fmt::Display for ContractState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractState::Created => f.pad("Created"),
            ContractState::GracePeriod => f.pad("Grace Period"),
            ContractState::OutOfFunds => f.pad("Out of Funds"),
            ContractState::Deleted => f.pad("Deleted"),
        }
    }
}
