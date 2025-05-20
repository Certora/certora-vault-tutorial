use crate::certora::specs::base_processor::{
    base_process_deposit, base_process_deposit_exact, base_process_deposit_with_fee,
    base_process_deposit_with_fee_exact, base_process_redeem_shares, base_process_slash,
    base_process_update_reward,
};
use crate::certora::specs::solvency::props_processor::SolvencyInvariant;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;

#[rule]
pub fn rule_solvency_process_deposit() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_deposit_with_fee() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit_with_fee::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_deposit_exact() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit_exact::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_deposit_with_fee_exact() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit_with_fee_exact::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_redeem_shares() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_redeem_shares::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_update_reward() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_update_reward::<SolvencyInvariant>(&accs);
}

#[rule]
pub fn rule_solvency_process_slash() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_slash::<SolvencyInvariant>(&accs);
}
