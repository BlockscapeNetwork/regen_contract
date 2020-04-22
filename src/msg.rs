use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm::types::{Coin, HumanAddr, CanonicalAddr};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub region: String,
    pub beneficiary: HumanAddr,
    pub oracle: HumanAddr,
    pub ecostate: i64,
    pub total_tokens: i64,
    pub payout_start_height: i64,
    pub payout_end_height: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateEcostate {ecostate: i64},
    Lock {},
    UnLock {},
    ChangeBeneficiary {beneficiary: HumanAddr},
    TransferOwnership {owner: HumanAddr},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    State {},
    Balance {address: HumanAddr}
}
