use crate::compat::{de_i64, de_u64};
use serde::{Deserialize, Serialize};

/// A contract bill report on the grid.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBillReport {
    #[serde(deserialize_with = "de_u64")]
    pub amount_billed: u64,
    #[serde(rename = "contractID", deserialize_with = "de_u64")]
    pub contract_id: u64,
    #[serde(deserialize_with = "de_i64")]
    pub timestamp: i64,
    pub discount_received: DiscountLevel,
}

#[derive(Serialize, Deserialize)]
/// Level of discount applied for a contract bill.
pub enum DiscountLevel {
    None,
    Default,
    Bronze,
    Silver,
    Gold,
}
