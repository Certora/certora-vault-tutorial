use crate::certora::specs::base_processor::{base_process_deposit, base_process_redeem_shares};
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

// VaultConsistencyInvariant does not make sense with process_update_reward
// If we assume that vault and token account has the same amount then the reward will be always zero
// and the instruction will revert.
