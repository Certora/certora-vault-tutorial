use crate::certora::specs::base_processor::{
    base_process_deposit, base_process_redeem_shares, base_process_update_reward, base_process_slash,
};
use crate::certora::specs::no_dilution::props_processor::NoDilutionProp;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;

#[rule]
pub fn rule_no_dilution_process_deposit() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit::<NoDilutionProp>(&accs);
}

#[rule]
pub fn rule_no_dilution_process_redeem_shares() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_redeem_shares::<NoDilutionProp>(&accs);
}

#[rule]
pub fn rule_no_dilution_process_update_reward() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_update_reward::<NoDilutionProp>(&accs);
}

#[rule]
pub fn rule_no_dilution_process_slash() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_slash::<NoDilutionProp>(&accs);
}