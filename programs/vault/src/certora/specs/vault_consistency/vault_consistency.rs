use crate::certora::specs::base_processor::{
    base_process_deposit, base_process_redeem_shares, base_process_slash,
    base_process_update_reward,
};
use crate::certora::specs::vault_consistency::props::VaultConsistencyInvariant;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;

#[rule]
pub fn rule_vault_consistency_process_deposit() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit::<VaultConsistencyInvariant>(&accs);
}

#[rule]
pub fn rule_vault_consistency_process_redeem_shares() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_redeem_shares::<VaultConsistencyInvariant>(&accs);
}

#[rule]
pub fn rule_vault_consistency_process_update_reward() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_update_reward::<VaultConsistencyInvariant>(&accs);
}

#[rule]
pub fn rule_vault_consistency_process_slash() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_slash::<VaultConsistencyInvariant>(&accs);
}
