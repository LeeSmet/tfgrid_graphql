use crate::compat::{de_i64, de_u64};
use serde::{Deserialize, Serialize};

/// A report about nru consumption, also used to prove workload liveliness
#[derive(Serialize, Deserialize)]
pub struct NRUConsumption {
    #[serde(rename = "contractID", deserialize_with = "de_u64")]
    pub contract_id: u64,
    #[serde(deserialize_with = "de_u64")]
    pub window: u64,
    #[serde(deserialize_with = "de_u64")]
    pub nru: u64,
    #[serde(deserialize_with = "de_i64")]
    pub timestamp: i64,
}
