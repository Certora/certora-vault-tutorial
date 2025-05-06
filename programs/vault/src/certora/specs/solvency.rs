use super::{base::*, props::*};
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
pub fn rule_solvency_process_slash() {
    base_process_slash::<SolvencyInvariant>();
}



