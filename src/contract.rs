use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use cosmwasm::errors::{Error, contract_err, unauthorized, Result};
use cosmwasm::traits::{Api, Extern, Storage};
use cosmwasm::types::{log, HumanAddr, CanonicalAddr, Coin, CosmosMsg, Env, Response};
use cw_storage::{singleton, Singleton};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub region: String,
    pub beneficiary: CanonicalAddr,
    pub owner: CanonicalAddr,
    pub oracle: CanonicalAddr,
    pub ecostate: i64,
    pub total_tokens: i64,
    pub released_tokens: i64,
    pub payout_start_height: i64,
    pub payout_end_height: i64,
    pub is_locked: bool,
}

impl State {
    fn is_expired(&self, env: &Env) -> bool {
        if env.block.height > self.payout_end_height {
            return true;
        }
        return false;
    }
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, b"config")
}

pub fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: InitMsg,
) -> Result<Response> {
    let state = State {
        region: msg.region,
        beneficiary: deps.api.canonical_address(&msg.beneficiary)?,
        oracle: deps.api.canonical_address(&msg.oracle)?,
        ecostate: msg.ecostate,
        total_tokens: msg.total_tokens,
        released_tokens: 0,
        payout_start_height: msg.payout_start_height,
        payout_end_height: msg.payout_end_height,
        is_locked: false,
        owner: env.message.signer.clone(),
    };
    if state.is_expired(&env) {
        contract_err("creating expired contract")
    } else {
        config(&mut deps.storage).save(&state)?;
        Ok(Response::default())
    }
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: HandleMsg,
) -> Result<Response> {
    let state = config(&mut deps.storage).load()?;
    match msg {
        HandleMsg::UpdateEcostate { ecostate } => try_payout(deps, env, ecostate),
        HandleMsg::Lock {} => try_lock(deps, env),
        HandleMsg::UnLock {} => try_unlock(deps, env),
        HandleMsg::ChangeBeneficiary { beneficiary } => try_change_beneficiary(deps, env, beneficiary),
        HandleMsg::TransferOwnership { owner } => try_transfer_ownership(deps, env, owner),
    }
}

fn try_payout<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    ecostate: i64,
) -> Result<Response> {
    let state: &mut State = &mut config(&mut deps.storage).load()?;
    let available = state.total_tokens - state.released_tokens;
    if state.oracle != env.message.signer {
        unauthorized()
    } else if state.is_locked {
        contract_err("contract is locked. no payout possible")
    } else if available <= 0 {
        contract_err("No more funds available")
    } else {
        let tokens = calculate_payout(state.ecostate, ecostate);
        if tokens != 0 {
            state.ecostate = ecostate;
            let amount: i64;
            if tokens > available {
                amount = available;
                state.released_tokens = state.total_tokens;
            } else {
                amount = tokens;
                state.released_tokens = state.released_tokens + tokens;
            }

            config(&mut deps.storage).save(state)?;
            
            let coins = vec![Coin {amount: amount.to_string(), denom: String::from("utree")}];
            send_tokens(&deps.api, &env.contract.address, &state.beneficiary, coins, "payout")
        } else {
            contract_err("Not enough improvement for payout")
        }
    }
}

fn calculate_payout(
    former_ecostate: i64,
    current_ecostate: i64,
) -> i64 {

    let diff = current_ecostate - former_ecostate;

    if diff < 0 { // if new state is worse then former state payout 0 coins
        0
    } else if diff < 100 { // if new state is between 0% and 1%  pay out 2 coins per percentage point above 50%
        let above_fifty = current_ecostate - 5000;
        if above_fifty > 0 {
            (above_fifty * 2) / 100 // two coins per percent above 50%
        } else {
            0
        }
    } else {
       diff // diff * 100coins / 100  because percentages are multiplied by 100
    }
}


fn try_change_beneficiary<S: Storage, A: Api>(
    deps: &mut Extern<S,A>,
    env: Env,
    newBeneficiary: HumanAddr,
) -> Result<Response> {
    let state: &mut State = &mut config(&mut deps.storage).load()?;
    if state.beneficiary != env.message.signer {
        unauthorized()
    } else {
        state.beneficiary = deps.api.canonical_address(&newBeneficiary)?;
        match config(&mut deps.storage).save(&state) {
            Err(why) => contract_err("couldn't save updated state"),
            Ok(x) => {
                let r = Response {
                    messages: vec![],
                    log: vec![
                        log("action", "change_beneficiary"),
                        log(
                            "account",
                            deps.api.human_address(&env.message.signer)?.as_str(),
                        ),
                    ],
                    data: None,
                };
                Ok(r)
            }
        }
    }
}

fn try_transfer_ownership<S: Storage, A: Api>(
    deps: &mut Extern<S,A>,
    env: Env,
    newOwner: HumanAddr,
) -> Result<Response> {
    let state: &mut State = &mut config(&mut deps.storage).load()?;
    if state.owner != env.message.signer {
        unauthorized()
    } else {
        state.owner = deps.api.canonical_address(&newOwner)?;
        match config(&mut deps.storage).save(&state) {
            Err(why) => contract_err("couldn't save updated state"),
            Ok(x) => {
                let r = Response {
                    messages: vec![],
                    log: vec![
                        log("action", "change_owner"),
                        log(
                            "account",
                            deps.api.human_address(&env.message.signer)?.as_str(),
                        ),
                    ],
                    data: None,
                };
                Ok(r)
            }
        }
    }
}

fn try_lock<T: Storage, A: Api>(
    deps: &mut Extern<T, A>,
    env: Env,
) -> Result<Response> {
    let state: &mut State = &mut config(&mut deps.storage).load()?;
    // only the owner can lock or unlock the contract
    if state.owner != env.message.signer {
        unauthorized()
    } else if state.is_locked { // can't lock a locked contract
        contract_err("contract already locked")
    } else {
        state.is_locked = true;
        match config(&mut deps.storage).save(&state) {
            Err(why) => contract_err("couldn't save updated state"),
            Ok(x) => {
                let r = Response {
                    messages: vec![],
                    log: vec![
                        log("action", "lock"),
                        log(
                            "account",
                            deps.api.human_address(&env.message.signer)?.as_str(),
                        ),
                    ],
                    data: None,
                };
                Ok(r)
            }
        }
    }
}

fn try_unlock<T: Storage, A: Api>(
    deps: &mut Extern<T, A>,
    env: Env,
) -> Result<Response> {
    let state: &mut State = &mut config(&mut deps.storage).load()?;
    // only the owner can lock or unlock the contract
    if state.owner != env.message.signer {
        unauthorized()
    } else if !state.is_locked { // can't lock a locked contract
        contract_err("contract already unlocked")
    } else {
        state.is_locked = false;
        match config(&mut deps.storage).save(&state) {
            Err(why) => contract_err("couldn't save updated state"),
            Ok(x) => {
                let r = Response {
                    messages: vec![],
                    log: vec![
                        log("action", "unlock"),
                        log(
                            "account",
                            deps.api.human_address(&env.message.signer)?.as_str(),
                        ),
                    ],
                    data: None,
                };
                Ok(r)
            }
        }
    }
}

// this is a helper to move the tokens, so the business logic is easy to read
fn send_tokens<A: Api>(
    api: &A,
    from_address: &CanonicalAddr,
    to_address: &CanonicalAddr,
    amount: Vec<Coin>,
    action: &str,
) -> Result<Response> {
    let from_human = api.human_address(from_address)?;
    let to_human = api.human_address(to_address)?;
    let log = vec![log("action", action), log("to", to_human.as_str())];

    let r = Response {
        messages: vec![CosmosMsg::Send {
            from_address: from_human,
            to_address: to_human,
            amount,
        }],
        log: log,
        data: None,
    };
    Ok(r)
}

pub fn query<S: Storage, A: Api>(_deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    
    Err(Error::NotFound {kind: "not implemented"})
    // this always returns error
    //match msg {}
}