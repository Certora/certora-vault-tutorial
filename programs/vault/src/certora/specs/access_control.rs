use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use crate::certora::specs::{base_processor::base_process_deposit, props_processor::AccessControlProcessDeposit};

#[rule]
pub fn rule_access_control_process_deposit() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit::<AccessControlProcessDeposit>(&accs);
}


