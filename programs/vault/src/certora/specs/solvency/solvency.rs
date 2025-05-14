use crate::certora::specs::base::{
    base_deposit_assets, base_deposit_assets_with_fee, base_process_slash, base_redeem_shares,
    base_update_reward,
};
use crate::certora::specs::solvency::props::SolvencyInvariant;
use cvlr::prelude::*;

#[rule]
pub fn rule_solvency_deposit_assets() {
    base_deposit_assets::<SolvencyInvariant>();
}

#[rule]
pub fn rule_solvency_deposit_assets_with_fee() {
    base_deposit_assets_with_fee::<SolvencyInvariant>();
}

#[rule]
pub fn rule_solvency_redeem_shares() {
    base_redeem_shares::<SolvencyInvariant>();
}

#[rule]
pub fn rule_solvency_update_reward() {
    base_update_reward::<SolvencyInvariant>();
}

#[rule]
pub fn rule_solvency_slash() {
    base_process_slash::<SolvencyInvariant>();
}
