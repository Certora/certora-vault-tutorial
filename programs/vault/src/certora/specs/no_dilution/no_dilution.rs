use crate::certora::specs::base::{
    base_deposit_assets, base_deposit_assets_with_fee, base_process_slash, base_redeem_shares,
    base_update_reward,
};
use crate::certora::specs::no_dilution::props::NoDilutionProp;
use cvlr::prelude::*;

#[rule]
pub fn rule_no_dilution_deposit_assets() {
    base_deposit_assets::<NoDilutionProp>();
}

#[rule]
pub fn rule_no_dilution_deposit_assets_with_fee() {
    base_deposit_assets_with_fee::<NoDilutionProp>();
}

#[rule]
pub fn rule_no_dilution_redeem_shares() {
    base_redeem_shares::<NoDilutionProp>();
}

#[rule]
pub fn rule_no_dilution_update_reward() {
    base_update_reward::<NoDilutionProp>();
}

#[rule]
/// It should produce a counterexample because slashing does not satisfy the "no dilution" property.
pub fn rule_no_dilution_process_slash() {
    base_process_slash::<NoDilutionProp>();
}
