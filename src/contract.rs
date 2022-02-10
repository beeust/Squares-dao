#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    BankMsg, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult,
    Uint128,
};

use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{State, STATE};
use cw721::Cw721Execute;
use cw721_base::state::TokenInfo;
use cw721_base::{Cw721Contract, Extension};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, cw721_base::ContractError> {
    let token_count = msg.tokens.len();
    let owner = info.sender.clone();

    STATE.save(
        deps.storage,
        &State {
            owner: info.sender.clone(),
            tokens: msg.tokens,
            mint_price: msg.mint_price,
        },
    )?;

    let tract = Cw721Contract::<Extension, Empty>::default();
    let resp = tract.instantiate(deps, _env, info, msg.base)?;

    Ok(resp
        .add_attribute("owner", owner)
        .add_attribute("token_count", token_count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<Extension>,
) -> Result<Response, cw721_base::ContractError> {
    let tract = Cw721Contract::<Extension, Empty>::default();
    match msg {
        ExecuteMsg::BatchMint { amount } => batch_mint(tract, deps, env, info, amount),
        ExecuteMsg::Mint(msg) => tract.mint(deps, env, info, msg),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => tract.approve(deps, env, info, spender, token_id, expires),
        ExecuteMsg::Revoke { spender, token_id } => {
            tract.revoke(deps, env, info, spender, token_id)
        }
        ExecuteMsg::ApproveAll { operator, expires } => {
            tract.approve_all(deps, env, info, operator, expires)
        }
        ExecuteMsg::RevokeAll { operator } => tract.revoke_all(deps, env, info, operator),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => tract.transfer_nft(deps, env, info, recipient, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => tract.send_nft(deps, env, info, contract, token_id, msg),
    }
}

pub fn batch_mint(
    tract: Cw721Contract<Extension, Empty>,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    n: u64,
) -> Result<Response, cw721_base::ContractError> {
    let owned_tokens = tract
        .tokens
        .idx
        .owner
        .prefix(info.sender.clone())
        .keys(deps.storage, None, None, Order::Ascending)
        .count();

    if owned_tokens as u64 + n > 10 {
        return Err(cw721_base::ContractError::Std(
            cosmwasm_std::StdError::generic_err("a single wallet cannot own more than 10 tokens"),
        ));
    }

    let mut token_ids: Vec<String> = Vec::new();
    for _ in 0..n {
        let mut state = STATE.load(deps.storage)?;
        let token_uri = state.tokens.pop().ok_or_else(|| {
            cw721_base::ContractError::Std(cosmwasm_std::StdError::generic_err(
                "contract is out of tokens",
            ))
        })?;
        STATE.save(deps.storage, &state)?;
        let token = TokenInfo {
            owner: info.sender.clone(),
            approvals: vec![],
            token_uri: Some(token_uri.clone()),
            extension: None,
        };

        let id = tract.token_count(deps.storage)? + 1;
        tract
            .tokens
            .update(deps.storage, &id.to_string(), |old| match old {
                Some(_) => Err(cw721_base::ContractError::Claimed {}),
                None => Ok(token),
            })?;
        tract.increment_tokens(deps.storage)?;

        token_ids.push(id.to_string())
    }

    let state = STATE.load(deps.storage)?;
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", info.sender)
        .add_attribute("token_ids", token_ids.join(","))
        .add_message(BankMsg::Send {
            to_address: state.owner.to_string(),
            amount: vec![Coin {
                denom: state.mint_price.denom.clone(),
                amount: Uint128::new((state.mint_price.amount * n) as u128),
            }],
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: cw721_base::QueryMsg) -> StdResult<Binary> {
    let tract = Cw721Contract::<Extension, Empty>::default();
    tract.query(deps, _env, msg)
}
