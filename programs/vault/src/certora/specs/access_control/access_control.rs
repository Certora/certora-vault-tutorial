use crate::certora::specs::{
    access_control::props::AccessControlProcessDeposit, base_processor::base_process_deposit,
};
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;

#[rule]
pub fn rule_access_control_process_deposit() {
    let accs = cvlr_deserialize_nondet_accounts();
    base_process_deposit::<AccessControlProcessDeposit>(&accs);
}
